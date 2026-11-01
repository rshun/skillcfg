use crate::{AppContext, AppError};
use skill_core::{profile::AgentProfile, AgentId, ReaderExposure};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct ResolvedRoot {
    pub agent: AgentId,
    pub variant: String,
    pub root_id: String,
    pub path: PathBuf,
    pub writable: bool,
    pub priority: i32,
}
#[derive(Default)]
pub struct ResolvedRoots {
    pub roots: Vec<ResolvedRoot>,
    pub issues: Vec<&'static str>,
}
pub fn resolve_profile_roots(_profile: &AgentProfile, _ctx: &AppContext) -> Result<ResolvedRoots, AppError> {
    Err(AppError { code: "not_implemented" })
}
pub fn reader_exposures(_path: &Path, _ctx: &AppContext) -> Result<Vec<ReaderExposure>, AppError> {
    Err(AppError { code: "not_implemented" })
}
