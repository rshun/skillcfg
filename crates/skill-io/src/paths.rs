use crate::IoError;
use std::path::Path;

pub fn physical_key(_path: &Path) -> Result<String, IoError> { Err(IoError { code: "not_implemented" }) }
pub fn is_link(_path: &Path) -> Result<bool, IoError> { Err(IoError { code: "not_implemented" }) }
