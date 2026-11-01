use crate::IoError;
use std::{fs, path::{Path,PathBuf}};

pub(crate) fn error(error: std::io::Error) -> IoError {
    IoError { code: match error.kind() { std::io::ErrorKind::NotFound => "not_found", std::io::ErrorKind::PermissionDenied => "permission_denied", _ => "read_failed" } }
}

pub fn is_link(path: &Path) -> Result<bool, IoError> {
    let meta = fs::symlink_metadata(path).map_err(error)?;
    #[cfg(windows)] { use std::os::windows::fs::MetadataExt; return Ok(meta.file_attributes() & 0x400 != 0); }
    #[cfg(not(windows))] { Ok(meta.file_type().is_symlink()) }
}

pub fn physical_key(path: &Path) -> Result<String, IoError> {
    if !path.is_absolute() { return Err(IoError { code: "invalid_root" }); }
    let mut parent = path.to_path_buf(); let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(&parent) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(parent.file_name().ok_or(IoError { code: "invalid_root" })?.to_owned());
                if !parent.pop() { return Err(IoError { code: "invalid_root" }); }
            }
            Err(e) => return Err(error(e)),
        }
    }
    let actual = fs::canonicalize(&parent).map_err(error)?;
    let base = existing_key(&actual)?;
    let mut remaining = PathBuf::new(); for part in suffix.iter().rev() { remaining.push(part); }
    let remaining = remaining.to_str().ok_or(IoError { code: "non_utf8_path" })?;
    #[cfg(windows)] let remaining = remaining.to_ascii_lowercase();
    Ok(format!("{base}/{}",remaining.replace('\\',"/")))
}

#[cfg(unix)]
fn existing_key(path:&Path)->Result<String,IoError>{
    use std::os::unix::fs::MetadataExt;
    let meta=fs::metadata(path).map_err(error)?; Ok(format!("unix:{}:{}",meta.dev(),meta.ino()))
}

#[cfg(windows)]
fn existing_key(path:&Path)->Result<String,IoError>{
    use std::os::windows::{fs::OpenOptionsExt,io::AsRawHandle};
    #[repr(C)] struct Info { attributes:u32,creation:[u32;2],access:[u32;2],write:[u32;2],volume:u32,size_high:u32,size_low:u32,links:u32,index_high:u32,index_low:u32 }
    #[link(name="kernel32")]
    unsafe extern "system" { #[link_name="GetFileInformationByHandle"] fn get_info(handle:*mut std::ffi::c_void,info:*mut Info)->i32; }
    let file=fs::OpenOptions::new().read(true).access_mode(0).share_mode(7).custom_flags(0x02000000).open(path).map_err(error)?;
    // The zeroed POD structure is initialized by the Windows API before fields are used.
    let mut info:Info=unsafe{std::mem::zeroed()};
    if unsafe{get_info(file.as_raw_handle(),&mut info)}==0{return Err(error(std::io::Error::last_os_error()));}
    Ok(format!("windows:{}:{}:{}",info.volume,info.index_high,info.index_low))
}
