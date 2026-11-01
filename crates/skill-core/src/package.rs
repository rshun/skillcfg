use crate::{CoreError, PackageComparison, PackageIssue, PackageTree, Platform, SkillMetadata};

pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 1024;
pub const MAX_DEPTH: usize = 16;

pub fn parse_skill_document(_bytes: &[u8]) -> Result<SkillMetadata, CoreError> {
    Err(CoreError::new("not_implemented"))
}

pub fn validate_package(_tree: &PackageTree, _platform: Platform) -> Vec<PackageIssue> {
    vec![PackageIssue { code: "not_implemented", path: None }]
}

pub fn compare_packages(_left: &PackageTree, _right: &PackageTree) -> PackageComparison {
    PackageComparison::default()
}
