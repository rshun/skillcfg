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

impl RootSpec {
    pub fn applies(&self, variant: &str) -> bool {
        self.applies_to_variants.as_ref().is_none_or(|items| items.iter().any(|v| v == variant))
    }
}

pub fn parse_agent_profile(bytes: &[u8]) -> Result<AgentProfile, CoreError> {
    use std::collections::BTreeSet;
    use crate::package::{is_slug, valid_relative_path};
    let fail = || CoreError::new("invalid_profile");
    if bytes.len() > MAX_PROFILE_BYTES { return Err(CoreError::new("profile_too_large")); }
    let mut depth = 0usize; let mut quoted = false; let mut escaped = false;
    for byte in bytes {
        if quoted {
            if escaped { escaped = false; } else if *byte == b'\\' { escaped = true; } else if *byte == b'"' { quoted = false; }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => { depth += 1; if depth > 16 { return Err(CoreError::new("profile_too_deep")); } }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let profile: AgentProfile = serde_json::from_slice(bytes).map_err(|_| fail())?;
    if profile.schema_version != 1 || profile.package_format != "skill-md-v1"
        || profile.display_name.trim().is_empty() || profile.display_name.chars().count() > 80
        || profile.display_name.chars().any(char::is_control)
        || profile.variants.is_empty() || profile.variants.len() > 8 || profile.roots.is_empty()
        || profile.roots.len() > 32 || profile.probes.len() > 8
        || profile.variants.iter().any(|v| !is_slug(v))
        || profile.variants.iter().collect::<BTreeSet<_>>().len() != profile.variants.len()
        || !profile.variants.contains(&profile.default_variant)
        || std::str::from_utf8(bytes).is_ok_and(crate::compatibility::contains_sensitive_literal)
        || profile.binary_names.len() > 8
        || profile.binary_names.iter().any(|n| n.is_empty() || n.len() > 64 || n == "." || n == ".." || !n.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)))
        || profile.icon_key.as_ref().is_some_and(|s| !is_slug(s))
        || profile.monogram.as_ref().is_some_and(|s| s.chars().count() > 4 || s.chars().any(char::is_control))
        || profile.notes.as_ref().is_some_and(|s| s.len() > 4096) {
        return Err(fail());
    }
    let mut probe_ids = BTreeSet::new();
    for probe in &profile.probes {
        if !is_slug(&probe.probe_id) || !probe_ids.insert(probe.probe_id.clone())
            || ![Platform::Windows, Platform::Macos, Platform::Linux].iter().all(|p| valid_relative_path(&probe.relative,*p))
            || probe.key_path.is_empty() || probe.key_path.len() > 16
            || probe.key_path.iter().any(|p| p.is_empty() || p.len() > 128 || p.chars().any(char::is_control))
            || matches!(&probe.on_missing, MissingValue::Unknown(s) if s != "unknown") {
            return Err(fail());
        }
    }
    let mut root_ids = BTreeSet::new();
    let mut write_keys = BTreeSet::new();
    for root in &profile.roots {
        let variants = root.applies_to_variants.as_ref().unwrap_or(&profile.variants);
        if !is_slug(&root.root_id) || !root_ids.insert(root.root_id.clone())
            || root.platforms.is_empty() || root.platforms.iter().collect::<BTreeSet<_>>().len() != root.platforms.len()
            || !root.platforms.iter().all(|p| valid_relative_path(&root.relative,*p))
            || (root.scope == Scope::Project) != (root.base == Base::Project)
            || (root.discovery == Discovery::AncestorsToRepo && root.scope != Scope::Project)
            || (root.write && !root.read) || variants.is_empty()
            || variants.iter().collect::<BTreeSet<_>>().len() != variants.len()
            || variants.iter().any(|v| !profile.variants.contains(v))
            || matches!(&root.read_evidence, ReadEvidence::BoolConfigV1{probe_id} if !probe_ids.contains(probe_id)) {
            return Err(fail());
        }
        if root.write {
            for platform in &root.platforms { for variant in variants {
                if !write_keys.insert((*platform,root.scope,variant.clone())) { return Err(fail()); }
            } }
        }
    }
    Ok(profile)
}
