use crate::{AppError, HostEnv};
use skill_core::{profile::AgentProfile, AgentId, Platform};
use std::{collections::BTreeMap, path::PathBuf};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/builtin_profiles.rs"));
}
pub fn builtin_profiles() -> Vec<Vec<u8>> {
    embedded::BUILTIN_DOCUMENTS
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileState {
    Builtin,
    Enabled,
    Pending,
    Disabled,
    Invalid,
}

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
    #[serde(default)]
    pub overrides: Vec<LocalOverride>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalOverride {
    pub root_id: String,
    pub platform: Platform,
    pub scope: skill_core::Scope,
    pub project: Option<PathBuf>,
    pub path: PathBuf,
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

pub struct AgentProfileView {
    pub revision: Option<String>,
    pub id: Option<AgentId>,
    pub display_name: String,
    pub source: &'static str,
    pub state: ProfileState,
    pub variants: Vec<String>,
    pub current_variant: Option<String>,
    pub roots: Vec<crate::roots::ResolvedRoot>,
    pub issues: Vec<&'static str>,
}

impl crate::App {
    pub fn list_agent_profiles(&self) -> Result<Vec<AgentProfileView>, AppError> {
        let mut views = Vec::new();
        for entry in &self.context.registry.entries {
            let mut view = AgentProfileView {
                revision: None,
                id: entry.profile.as_ref().map(|p| p.id.clone()),
                display_name: entry
                    .profile
                    .as_ref()
                    .map_or_else(|| "Invalid profile".into(), |p| p.display_name.clone()),
                source: if entry.state == ProfileState::Builtin {
                    "builtin"
                } else {
                    "custom"
                },
                state: entry.state,
                variants: entry
                    .profile
                    .as_ref()
                    .map_or_else(Vec::new, |p| p.variants.clone()),
                current_variant: None,
                roots: Vec::new(),
                issues: entry.issues.clone(),
            };
            if let Some(profile) = &entry.profile {
                view.current_variant = Some(
                    self.context
                        .registry
                        .selected_variants
                        .get(&profile.id)
                        .unwrap_or(&profile.default_variant)
                        .clone(),
                );
                // Pending/invalid definitions are listed without touching their roots.
                if matches!(
                    entry.state,
                    ProfileState::Builtin | ProfileState::Enabled | ProfileState::Disabled
                ) {
                    match crate::roots::resolve_profile_roots(profile, &self.context) {
                        Ok(resolved) => {
                            view.roots = resolved.roots;
                            view.issues.extend(resolved.issues);
                        }
                        Err(error) => view.issues.push(error.code),
                    }
                }
            }
            views.push(view);
        }
        Ok(views)
    }
}

pub fn approval_matches(approval: &Approval, entry: &ProfileEntry, env: &HostEnv) -> bool {
    approval.document == entry.raw
        && approval.platform == env.platform
        && approval.home == env.home
        && approval.appdata == env.appdata
        && approval.xdg_config == env.xdg_config
        && approval.overrides.iter().all(|o| {
            o.path.is_absolute()
                && (o.scope != skill_core::Scope::Project
                    || o.project.as_ref().is_some_and(|p| p.is_absolute()))
                && entry.profile.as_ref().is_some_and(|p| {
                    p.roots.iter().any(|r| {
                        r.root_id == o.root_id
                            && r.scope == o.scope
                            && r.platforms.contains(&o.platform)
                    })
                })
        })
}

pub fn load_registry(source: &RegistrySource, env: &HostEnv) -> Result<RegistrySnapshot, AppError> {
    use skill_core::{
        package::is_slug,
        profile::{parse_agent_profile, MAX_PROFILE_BYTES},
    };
    use std::{collections::BTreeSet, fs, io::ErrorKind};
    if !env.home.is_absolute() {
        return Err(AppError {
            code: "invalid_home",
        });
    }
    let mut result = RegistrySnapshot::default();
    if let Some(path) = &source.approvals_path {
        match skill_io::snapshot::read_bounded_file(path, 64 * 1024 * 1024) {
            Ok(bytes) => match serde_json::from_slice::<Vec<Approval>>(&bytes) {
                Ok(a) if a.len() <= 256 => result.approvals = a,
                _ => result.entries.push(ProfileEntry {
                    profile: None,
                    state: ProfileState::Invalid,
                    raw: vec![],
                    issues: vec!["invalid_approvals"],
                }),
            },
            Err(e) if e.code == "not_found" => {}
            Err(_) => result.entries.push(ProfileEntry {
                profile: None,
                state: ProfileState::Invalid,
                raw: vec![],
                issues: vec!["unreadable_approvals"],
            }),
        }
    }
    let mut documents: Vec<(Vec<u8>, bool)> =
        source.builtins.iter().cloned().map(|b| (b, true)).collect();
    match fs::read_dir(&source.custom_dir) {
        Ok(entries) => {
            let mut names = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|_| AppError {
                    code: "registry_read_failed",
                })?;
                if entry.path().extension().is_some_and(|s| s == "json") {
                    names.push(entry.path());
                }
                if documents.len() + names.len() > 256 {
                    return Err(AppError {
                        code: "too_many_profiles",
                    });
                }
            }
            names.sort();
            for path in names {
                let bytes = if skill_io::paths::is_link(&path).unwrap_or(true) {
                    Err(skill_io::IoError {
                        code: "profile_link",
                    })
                } else {
                    skill_io::snapshot::read_bounded_file(&path, MAX_PROFILE_BYTES)
                };
                match bytes {
                    Ok(b) => documents.push((b, false)),
                    Err(e) => result.entries.push(ProfileEntry {
                        profile: None,
                        state: ProfileState::Invalid,
                        raw: vec![],
                        issues: vec![e.code],
                    }),
                }
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(_) => result.entries.push(ProfileEntry {
            profile: None,
            state: ProfileState::Invalid,
            raw: vec![],
            issues: vec!["registry_read_failed"],
        }),
    }
    if documents.len() > 256 {
        return Err(AppError {
            code: "too_many_profiles",
        });
    }
    for (raw, builtin) in documents {
        let profile = match parse_agent_profile(&raw) {
            Ok(p) => p,
            Err(e) => {
                result.entries.push(ProfileEntry {
                    profile: None,
                    state: ProfileState::Invalid,
                    raw: vec![],
                    issues: vec![e.code],
                });
                continue;
            }
        };
        let namespace = builtin
            || profile
                .id
                .as_str()
                .strip_prefix("custom.")
                .is_some_and(is_slug);
        let mut entry = ProfileEntry {
            profile: Some(profile),
            state: if builtin {
                ProfileState::Builtin
            } else {
                ProfileState::Pending
            },
            raw,
            issues: vec![],
        };
        if !namespace {
            entry.state = ProfileState::Invalid;
            entry.issues.push("invalid_namespace");
        } else if !builtin {
            let approvals: Vec<_> = result
                .approvals
                .iter()
                .filter(|a| entry.profile.as_ref().is_some_and(|p| p.id == a.id))
                .collect();
            if approvals.len() == 1 && approval_matches(approvals[0], &entry, env) {
                entry.state = if approvals[0].enabled {
                    ProfileState::Enabled
                } else {
                    ProfileState::Disabled
                };
            }
        }
        result.entries.push(entry);
    }
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for entry in &result.entries {
        if let Some(p) = &entry.profile {
            if !seen.insert(p.id.clone()) {
                duplicates.insert(p.id.clone());
            }
        }
    }
    for entry in &mut result.entries {
        if entry
            .profile
            .as_ref()
            .is_some_and(|p| duplicates.contains(&p.id))
        {
            entry.state = ProfileState::Invalid;
            entry.issues.push("duplicate_id");
        }
    }
    Ok(result)
}
