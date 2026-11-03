use crate::{App, AppError};
use std::path::Path;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStatus {
    pub package_id: String,
    pub agent_id: skill_core::AgentId,
    pub state: &'static str,
    pub path: String,
}
impl App {
    pub fn library_statuses(&self, _library: &Path) -> Result<Vec<TargetStatus>, AppError> {
        Err(AppError { code: "not_implemented" })
    }
    pub fn read_agent_profile_document(&self, _id: &skill_core::AgentId) -> Result<(String, String), AppError> {
        Err(AppError { code: "not_implemented" })
    }
}
