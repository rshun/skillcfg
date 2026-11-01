use crate::{roots::ResolvedRoot, App, AppError};
use skill_core::{AgentId, SkillMetadata};
use std::path::PathBuf;

pub struct InventoryItem {
    pub physical_key: String,
    pub metadata: Option<SkillMetadata>,
    pub sources: Vec<PathBuf>,
    pub files: Vec<String>,
    pub issues: Vec<&'static str>,
}
pub struct AgentInventory {
    pub agent: AgentId,
    pub installed: bool,
    pub roots: Vec<ResolvedRoot>,
    pub items: Vec<InventoryItem>,
    pub errors: Vec<&'static str>,
}
impl App {
    pub fn list_agent_skills(&self, _agent: AgentId) -> Result<AgentInventory, AppError> {
        Err(AppError { code: "not_implemented" })
    }
}
