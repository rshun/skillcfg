use crate::{AgentId, CoreError, Platform, Scope};

pub const MAX_PROFILE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentProfile {
    pub schema_version: u32,
    pub id: AgentId,
    pub display_name: String,
    pub package_format: String,
    pub variants: Vec<String>,
    pub default_variant: String,
    pub roots: Vec<RootSpec>,
    #[serde(default)] pub probes: Vec<BoolProbe>,
    #[serde(default)] pub binary_names: Vec<String>,
    pub icon_key: Option<String>,
    pub monogram: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Base { Home, Project, Appdata, XdgConfig }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Discovery { Children, AncestorsToRepo }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role { Native, Compatibility }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RootSpec {
    pub root_id: String,
    pub platforms: Vec<Platform>,
    pub scope: Scope,
    pub base: Base,
    pub relative: String,
    pub discovery: Discovery,
    pub role: Role,
    pub read: bool,
    pub write: bool,
    pub priority: i32,
    pub applies_to_variants: Option<Vec<String>>,
    #[serde(default)] pub read_evidence: ReadEvidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReadEvidence {
    #[default] Always,
    BoolConfigV1 { #[serde(rename = "probeId")] probe_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeFormat { Json, Toml }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MissingValue { Bool(bool), Unknown(String) }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoolProbe {
    pub probe_id: String,
    pub base: Base,
    pub relative: String,
    pub format: ProbeFormat,
    pub key_path: Vec<String>,
    pub expected: bool,
    pub on_missing: MissingValue,
}

pub fn parse_agent_profile(_bytes: &[u8]) -> Result<AgentProfile, CoreError> {
    Err(CoreError::new("not_implemented"))
}
