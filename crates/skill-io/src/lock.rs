use crate::{
    disk::{directories, err, write_new},
    IoError,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(crate) struct Lock {
    path: PathBuf,
    released: PathBuf,
}
impl Lock {
    pub fn acquire(path: &Path, released: &Path) -> Result<Self, IoError> {
        directories(path.parent().ok_or(err("invalid_path"))?)?;
        write_new(path, b"Skill Hub cooperative lock\n")
            .map_err(|_| err("locked_or_unwritable"))?;
        Ok(Self {
            path: path.into(),
            released: released.into(),
        })
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::rename(&self.path, &self.released);
    }
}
