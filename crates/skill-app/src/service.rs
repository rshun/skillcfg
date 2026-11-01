use crate::{App, AppError};
use skill_core::{
    plan::{ConflictDecision, PackageId, SelectionPair, TargetKey},
    AgentId, Scope,
};
use std::{collections::BTreeMap, path::PathBuf};

pub trait Clock: Send + Sync {
    fn now(&self) -> u64;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuiSettings {
    pub version: u32,
    pub library_path: PathBuf,
    pub backup_path: PathBuf,
    pub default_scope: Scope,
    pub last_project_dir: Option<PathBuf>,
    pub agent_variants: BTreeMap<AgentId, String>,
}
#[derive(Debug, Clone)]
pub struct SettingsView {
    pub settings: GuiSettings,
    pub revision: String,
    pub warnings: Vec<String>,
}
pub struct SettingsRequest {
    pub expected_revision: String,
    pub settings: GuiSettings,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryRecord {
    pub id: String,
    pub current_revision: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub source_kind: String,
    pub source_agent: Option<AgentId>,
    pub source_path: PathBuf,
    pub imported_at: u64,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryView {
    pub first_use: bool,
    pub records: Vec<LibraryRecord>,
}
pub struct SafePackageView {
    pub package_id: String,
    pub revision_id: String,
    pub document: Option<String>,
    pub edit_token: Option<String>,
    pub files: Vec<String>,
    pub issues: Vec<String>,
}
pub struct ImportRequest {
    pub source: PathBuf,
    pub library_path: PathBuf,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    pub item_id: String,
    pub name: String,
    pub source_path: String,
    pub actual_path: String,
    pub issues: Vec<String>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportScan {
    pub scan_id: String,
    pub items: Vec<ImportItem>,
}
pub enum ImportAction {
    Skip,
    New,
    Replace(PackageId),
}
pub struct ImportChoice {
    pub item_id: String,
    pub action: ImportAction,
}
pub struct EditDocumentRequest {
    pub package_id: PackageId,
    pub edit_token: String,
    pub document: String,
}
pub struct DistributionRequest {
    pub library_path: PathBuf,
    pub scope: Scope,
    pub project_root: Option<PathBuf>,
    pub selection: Vec<SelectionPair>,
    pub conflict_decisions: BTreeMap<TargetKey, ConflictDecision>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDto {
    pub preview_id: String,
    pub expires_at: u64,
    pub groups: Vec<skill_core::diff::SafePlanGroup>,
    pub warnings: Vec<String>,
    pub blocked: bool,
}
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptDto {
    pub operation_id: String,
    pub backup_location: String,
    pub outcomes: Vec<SafeOutcome>,
}
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeOutcome {
    pub path: String,
    pub outcome: skill_io::Outcome,
    pub reason: Option<String>,
}
pub struct ProfileChangeRequest {
    pub profile_json: Vec<u8>,
    pub expected_revision: Option<String>,
    pub local_root_overrides: Vec<crate::registry::LocalOverride>,
    pub enable_after_save: bool,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePreviewDto {
    pub preview_id: String,
    pub profile_id: AgentId,
    pub resolved_roots: Vec<String>,
    pub probe_paths: Vec<String>,
    pub variants: Vec<String>,
    pub blocked: bool,
    pub warnings: Vec<String>,
}
pub struct ProfileEnableRequest {
    pub agent_id: AgentId,
    pub enabled: bool,
    pub expected_revision: String,
}
impl App {
    pub fn preview_distribution(&self, _req: DistributionRequest) -> Result<PreviewDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn commit_distribution(&self, _id: &str) -> Result<ReceiptDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn list_history(&self) -> Result<Vec<ReceiptDto>, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn list_recoveries(&self) -> Result<Vec<skill_io::RecoveryRecord>, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn preview_restore(&self, _id: &str) -> Result<PreviewDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn commit_restore(&self, _id: &str) -> Result<ReceiptDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn preview_agent_profile(
        &self,
        _req: ProfileChangeRequest,
    ) -> Result<ProfilePreviewDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn commit_agent_profile(&self, _id: &str) -> Result<ReceiptDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
    pub fn set_agent_profile_enabled(
        &self,
        _req: ProfileEnableRequest,
    ) -> Result<ReceiptDto, AppError> {
        Err(AppError {
            code: "not_implemented",
        })
    }
}
