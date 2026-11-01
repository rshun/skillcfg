use skill_core::PackageTree;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct IoError { pub code: &'static str }

#[derive(Clone, Copy)]
pub struct ScanLimits {
    pub file_bytes: usize,
    pub package_bytes: usize,
    pub entries: usize,
    pub depth: usize,
}
impl Default for ScanLimits {
    fn default() -> Self { Self { file_bytes: 4*1024*1024, package_bytes: 16*1024*1024, entries: 1024, depth: 16 } }
}

#[derive(Clone)]
pub struct PackageSnapshot {
    pub source: PathBuf,
    pub actual_path: PathBuf,
    pub physical_key: String,
    pub root_was_link: bool,
    pub tree: PackageTree,
}

pub fn read_package_snapshot(_root: &Path, _limits: &ScanLimits) -> Result<PackageSnapshot, IoError> {
    Err(IoError { code: "not_implemented" })
}
