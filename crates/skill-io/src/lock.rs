use crate::{
    disk::{directories, err, sync_parent, write_new},
    IoError,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct RecoveryLock {
    path: PathBuf,
    key: String,
    bytes: Vec<u8>,
    owner: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Owner {
    version: u32,
    operation_id: String,
}
pub(crate) struct Lock {
    path: PathBuf,
    released: PathBuf,
    key: String,
    file: Option<File>,
    bytes:Vec<u8>,
}
fn open(path: &Path, create: bool) -> Result<File, IoError> {
    if !create
        && (crate::paths::is_link(path)?
            || !fs::symlink_metadata(path)
                .map_err(|_| err("lock_unreadable"))?
                .is_file())
    {
        return Err(err("invalid_lock"));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        #[cfg(target_os = "linux")]
        options.custom_flags(0x800 | 0x20000);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x4 | 0x100);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(4).custom_flags(0x00200000);
    }
    let file = options
        .open(path)
        .map_err(|_| err("locked_or_unwritable"))?;
    let meta = file.metadata().map_err(|_| err("invalid_lock"))?;
    if !meta.is_file() {
        return Err(err("invalid_lock"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(err("invalid_lock"));
        }
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        unsafe extern "C" {
            fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
        }
        // flock belongs to this open description and the kernel releases it on process exit.
        if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
            return Err(err("operation_running"));
        }
    }
    Ok(file)
}
fn contents(file: &mut File) -> Result<Vec<u8>, IoError> {
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| err("lock_unreadable"))?;
    if bytes.len() > 4096 {
        return Err(err("invalid_lock"));
    }
    Ok(bytes)
}
pub(crate) fn probe(path: &Path, id: &str) -> Result<Option<RecoveryLock>, IoError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(err("lock_unreadable")),
        Ok(_) => {}
    }
    let mut file = open(path, false)?;
    let key = crate::paths::physical_key(path)?;
    let bytes = contents(&mut file)?;
    let owner: Owner = serde_json::from_slice(&bytes).map_err(|_| err("unknown_lock_owner"))?;
    if owner.version != 1 || owner.operation_id != id {
        return Err(err("different_lock_owner"));
    }
    Ok(Some(RecoveryLock {
        path: path.into(),
        key,
        bytes,
        owner: id.into(),
    }))
}
impl Lock {
    pub fn acquire(
        path: &Path,
        released: &Path,
        id: &str,
        recovery: Option<&RecoveryLock>,
    ) -> Result<Self, IoError> {
        directories(path.parent().ok_or(err("invalid_path"))?)?;
        if let Some(evidence) = recovery {
            if evidence.path != path || !id.starts_with("op-") {
                return Err(err("invalid_recovery_lock"));
            }
            let mut old = open(path, false)?;
            if crate::paths::physical_key(path)? != evidence.key
                || contents(&mut old)? != evidence.bytes
            {
                return Err(err("recovery_lock_changed"));
            }
            let destination = released
                .parent()
                .ok_or(err("invalid_path"))?
                .join("recovered.lock");
            if destination.exists() {
                return Err(err("recovery_lock_changed"));
            }
            archive(path,&destination,&evidence.bytes)?;
        }
        let mut file = open(path, true)?;
        let bytes = serde_json::to_vec(&Owner {
            version: 1,
            operation_id: id.into(),
        })
        .map_err(|_| err("invalid_lock"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| err("lock_write_failed"))?;
        let key = crate::paths::physical_key(path)?;
        Ok(Self {
            path: path.into(),
            released: released.into(),
            key,
            file: Some(file),
            bytes,
        })
    }
    pub fn valid(&self) -> Result<(), IoError> {
        if self.file.is_none() || crate::paths::physical_key(&self.path)? != self.key {
            return Err(err("lock_changed"));
        }
        Ok(())
    }
    pub fn release(&mut self) -> Result<(), IoError> {
        self.valid()?;
        if self.released.exists() {
            return Err(err("lock_archive_failed"));
        }
        archive(&self.path,&self.released,&self.bytes)?;
        self.file = None;
        sync_parent(&self.released)
    }
}
impl RecoveryLock {
    pub(crate) fn matches(&self, id: &str) -> bool {
        self.owner == id
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        if self.file.is_some() {
            let _ = self.release();
        }
    }
}

pub(crate) fn stale_owner(path: &Path) -> Result<Option<String>, IoError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(err("lock_unreadable")),
        Ok(_) => {}
    }
    let mut file = open(path, false)?;
    let owner: Owner =
        serde_json::from_slice(&contents(&mut file)?).map_err(|_| err("unknown_lock_owner"))?;
    if owner.version != 1 || !skill_core::package::is_slug(&owner.operation_id) {
        return Err(err("unknown_lock_owner"));
    }
    Ok(Some(owner.operation_id))
}

fn archive(path:&Path,copy:&Path,bytes:&[u8])->Result<(),IoError>{
    // Preserve the lock inode on its own volume. The central backup receives a
    // durable metadata copy, so a different backup filesystem needs no rename.
    write_new(copy,bytes)?;sync_parent(copy)?;
    let local=path.parent().ok_or(err("invalid_path"))?.join(".lock-records").join(crate::disk::operation_id());
    directories(&local)?;
    let archived=local.join(copy.file_name().ok_or(err("invalid_path"))?);
    fs::rename(path,&archived).map_err(|_|err("lock_archive_failed"))?;
    sync_parent(&archived)?;sync_parent(path)
}
