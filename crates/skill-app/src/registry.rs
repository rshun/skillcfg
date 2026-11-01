use crate::{AppError, HostEnv};
use skill_core::{profile::AgentProfile, AgentId, Platform};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileState { Builtin, Enabled, Pending, Disabled, Invalid }

#[derive(Clone)]
pub struct ProfileEntry {
    pub profile: Option<AgentProfile>,
    pub state: ProfileState,
    pub raw: Vec<u8>,
    pub issues: Vec<&'static str>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub id: AgentId,
    pub document: Vec<u8>,
    pub platform: Platform,
    pub home: PathBuf,
    pub appdata: Option<PathBuf>,
    pub xdg_config: Option<PathBuf>,
    pub enabled: bool,
    #[serde(default)] pub overrides: BTreeMap<String, PathBuf>,
}

pub struct RegistrySource {
    pub builtins: Vec<Vec<u8>>,
    pub custom_dir: PathBuf,
    pub approvals_path: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub struct RegistrySnapshot {
    pub entries: Vec<ProfileEntry>,
    pub approvals: Vec<Approval>,
    pub selected_variants: BTreeMap<AgentId, String>,
}

pub fn load_registry(_source: &RegistrySource, _env: &HostEnv) -> Result<RegistrySnapshot, AppError> {
    Err(AppError { code: "not_implemented" })
}
