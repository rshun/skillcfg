use crate::bridge::SafeError;
use std::path::{Path, PathBuf};
pub trait DirectoryPicker { fn pick(&self, initial: Option<&Path>) -> Result<Option<PathBuf>, SafeError>; }
pub fn choose_directory(_picker: &dyn DirectoryPicker, _initial: Option<&Path>) -> Result<Option<PathBuf>, SafeError> {
    Err(SafeError { code: "not_implemented".into(), message: "尚未实现".into() })
}
