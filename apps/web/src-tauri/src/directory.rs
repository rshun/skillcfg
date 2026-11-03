use crate::bridge::SafeError;
use std::path::{Path, PathBuf};
pub trait DirectoryPicker { fn pick(&self, initial: Option<&Path>) -> Result<Option<PathBuf>, SafeError>; }
pub fn choose_directory(picker: &dyn DirectoryPicker, initial: Option<&Path>) -> Result<Option<PathBuf>, SafeError> {
    if initial.is_some_and(|p|!p.is_absolute()){return Err(SafeError::new("invalid_directory"));}
    let chosen=picker.pick(initial)?;
    if chosen.as_ref().is_some_and(|p|!p.is_absolute()||!p.is_dir()){return Err(SafeError::new("invalid_directory"));}
    Ok(chosen)
}
