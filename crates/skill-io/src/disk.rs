use crate::{read_package_snapshot, IoError, ScanLimits};
use skill_core::{EntryKind, PackageTree, Platform};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
pub(crate) fn err(code: &'static str) -> IoError {
    IoError { code }
}
pub fn operation_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "op-{:x}-{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn resolved_path(path: &Path) -> Result<PathBuf, IoError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(err("invalid_path"));
    }
    let mut parent = path.to_owned();
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(&parent) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(parent.file_name().ok_or(err("invalid_path"))?.to_owned());
                if !parent.pop() {
                    return Err(err("invalid_path"));
                }
            }
            Err(_) => return Err(err("path_unreadable")),
        }
    }
    let mut actual = fs::canonicalize(parent).map_err(|_| err("path_unreadable"))?;
    for part in suffix.into_iter().rev() {
        actual.push(part);
    }
    Ok(actual)
}
pub(crate) fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
pub(crate) fn directories(path: &Path) -> Result<(), IoError> {
    if path.is_dir() {
        if crate::paths::is_link(path)? || resolved_path(path)? != path {
            return Err(err("directory_link"));
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        directories(parent)?;
    }
    fs::create_dir(path).map_err(|_| err("create_directory_failed"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| err("permissions_failed"))?;
    }
    Ok(())
}
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), IoError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| err("create_file_failed"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| err("persist_failed"))
}
pub(crate) fn sync_parent(path: &Path) -> Result<(), IoError> {
    #[cfg(unix)]
    fs::File::open(path.parent().ok_or(err("invalid_path"))?)
        .and_then(|f| f.sync_all())
        .map_err(|_| err("persist_failed"))?;
    #[cfg(windows)]
    let _ = path;
    Ok(())
}
pub(crate) fn json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), IoError> {
    let data = serde_json::to_vec(value).map_err(|_| err("invalid_record"))?;
    let temp = path.with_file_name(format!(".{}.tmp", operation_id()));
    write_new(&temp, &data)?;
    fs::rename(temp, path).map_err(|_| err("persist_failed"))?;
    sync_parent(path)
}
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, IoError> {
    serde_json::from_slice(&crate::snapshot::read_bounded_file(path, 4 * 1024 * 1024)?)
        .map_err(|_| err("invalid_record"))
}
pub fn optional_file(path: &Path) -> Result<Option<Vec<u8>>, IoError> {
    match crate::snapshot::read_bounded_file(path, 64 * 1024 * 1024) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.code == "not_found" => Ok(None),
        Err(e) => Err(e),
    }
}
pub fn optional_tree(path: &Path) -> Result<Option<PackageTree>, IoError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(err("target_unreadable")),
        Ok(_) => {}
    }
    if crate::paths::is_link(path)? {
        return Err(err("target_link"));
    }
    Ok(Some(
        read_package_snapshot(path, &ScanLimits::default())?.tree,
    ))
}
pub(crate) fn native() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else {
        Platform::Linux
    }
}
pub(crate) fn tree_size(tree: &PackageTree) -> usize {
    tree.entries.iter().fold(0usize, |n, e| {
        n.saturating_add(match &e.kind {
            EntryKind::File { bytes, .. } => bytes.len(),
            _ => 0,
        })
    })
}
pub(crate) fn write_tree(path: &Path, tree: &PackageTree) -> Result<(), IoError> {
    let unsafe_issue = skill_core::validate_package(tree, native())
        .into_iter()
        .any(|i| {
            matches!(
                i.code,
                "invalid_path"
                    | "path_collision"
                    | "duplicate_path"
                    | "case_collision"
                    | "too_deep"
                    | "too_many_entries"
                    | "file_too_large"
                    | "package_too_large"
            )
        });
    if unsafe_issue {
        return Err(err("unsafe_tree"));
    }
    directories(path)?;
    for entry in &tree.entries {
        let destination = path.join(&entry.relative_path);
        match &entry.kind {
            EntryKind::Directory => directories(&destination)?,
            EntryKind::File {
                bytes,
                unix_executable,
            } => {
                directories(destination.parent().ok_or(err("invalid_path"))?)?;
                write_new(&destination, bytes)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(
                        &destination,
                        fs::Permissions::from_mode(if *unix_executable { 0o700 } else { 0o600 }),
                    )
                    .map_err(|_| err("permissions_failed"))?;
                    fs::File::open(&destination)
                        .and_then(|f| f.sync_all())
                        .map_err(|_| err("persist_failed"))?;
                }
                #[cfg(windows)]
                let _ = unix_executable;
                sync_parent(&destination)?;
            }
        }
    }
    if &read_package_snapshot(path, &ScanLimits::default())?.tree != tree {
        return Err(err("copy_mismatch"));
    }
    sync_parent(path)
}
