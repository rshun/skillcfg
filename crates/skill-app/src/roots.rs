use crate::{AppContext, AppError, ProfileState};
use skill_core::{
    profile::{AgentProfile, Base, Discovery, MissingValue, ProbeFormat, ReadEvidence},
    AgentId, ReadState, ReaderExposure, Scope,
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

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
pub fn resolve_profile_roots(
    profile: &AgentProfile,
    ctx: &AppContext,
) -> Result<ResolvedRoots, AppError> {
    let variant = ctx
        .registry
        .selected_variants
        .get(&profile.id)
        .unwrap_or(&profile.default_variant);
    if !profile.variants.contains(variant) {
        return Err(AppError {
            code: "invalid_variant",
        });
    }
    let mut result = ResolvedRoots::default();
    let project = if ctx.scope == Scope::Project {
        let p = ctx
            .env
            .project_root
            .as_ref()
            .filter(|p| p.is_absolute() && p.is_dir())
            .ok_or(AppError {
                code: "invalid_project",
            })?;
        Some(fs::canonicalize(p).map_err(|_| AppError {
            code: "invalid_project",
        })?)
    } else {
        None
    };
    let entry = ctx
        .registry
        .entries
        .iter()
        .find(|e| e.profile.as_ref().is_some_and(|p| p.id == profile.id));
    let approval = entry.and_then(|e| {
        ctx.registry
            .approvals
            .iter()
            .find(|a| a.id == profile.id && crate::registry::approval_matches(a, e, &ctx.env))
    });
    for rule in &profile.roots {
        if rule.scope != ctx.scope
            || !rule.platforms.contains(&ctx.env.platform)
            || !rule.applies(variant)
            || !rule.read
        {
            continue;
        }
        let Some(base) = ctx.env.base(rule.base) else {
            result.issues.push("missing_base");
            continue;
        };
        if !base.is_absolute() {
            result.issues.push("invalid_base");
            continue;
        }
        let mut bases = vec![if rule.base == Base::Project {
            project.as_ref().unwrap().clone()
        } else {
            base.clone()
        }];
        if rule.discovery == Discovery::AncestorsToRepo {
            let start = bases[0].clone();
            let repo = start
                .ancestors()
                .find(|p| p.join(".git").exists())
                .map(Path::to_owned);
            if let Some(repo) = repo {
                let mut parent = start.clone();
                while parent != repo {
                    if !parent.pop() {
                        break;
                    }
                    bases.push(parent.clone());
                }
            }
        }
        for (i, base) in bases.iter().enumerate() {
            let mut path = base.join(&rule.relative);
            if i == 0 {
                if let Some(overridden) = approval.and_then(|a| {
                    a.overrides.iter().find(|o| {
                        o.root_id == rule.root_id
                            && o.platform == ctx.env.platform
                            && o.scope == ctx.scope
                            && (ctx.scope == Scope::User
                                || o.project.as_ref() == ctx.env.project_root.as_ref())
                    })
                }) {
                    path = overridden.path.clone();
                }
            }
            if path == ctx.env.home
                || path.starts_with(ctx.env.home.join(".skill-hub"))
                || project.as_ref().is_some_and(|p| p == &path)
            {
                result.issues.push("protected_root");
                continue;
            }
            result.roots.push(ResolvedRoot {
                agent: profile.id.clone(),
                variant: variant.clone(),
                root_id: rule.root_id.clone(),
                path,
                writable: rule.write && i == 0,
                priority: rule.priority,
            });
        }
    }
    Ok(result)
}

fn probe_state(profile: &AgentProfile, id: &str, ctx: &AppContext) -> ReadState {
    let Some(probe) = profile.probes.iter().find(|p| p.probe_id == id) else {
        return ReadState::Unknown;
    };
    let missing = || match &probe.on_missing {
        MissingValue::Bool(true) => ReadState::ConfigEnabled,
        MissingValue::Bool(false) => ReadState::ConfigDisabled,
        _ => ReadState::Unknown,
    };
    let Some(base) = ctx.env.base(probe.base) else {
        return ReadState::Unknown;
    };
    let bytes =
        match skill_io::snapshot::read_bounded_file(&base.join(&probe.relative), 4 * 1024 * 1024) {
            Ok(b) => b,
            Err(e) if e.code == "not_found" => return missing(),
            Err(_) => return ReadState::Unknown,
        };
    let value = match probe.format {
        ProbeFormat::Json => {
            let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                return ReadState::Unknown;
            };
            let mut item = &doc;
            for key in &probe.key_path {
                match item.get(key) {
                    Some(v) => item = v,
                    None => return missing(),
                }
            }
            item.as_bool()
        }
        ProbeFormat::Toml => {
            let Ok(text) = std::str::from_utf8(&bytes) else {
                return ReadState::Unknown;
            };
            let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
                return ReadState::Unknown;
            };
            let Some(mut item) = doc.get(&probe.key_path[0]) else {
                return missing();
            };
            for key in &probe.key_path[1..] {
                let Some(table) = item.as_table_like() else {
                    return ReadState::Unknown;
                };
                let Some(next) = table.get(key) else {
                    return missing();
                };
                item = next;
            }
            item.as_bool()
        }
    };
    match value {
        Some(value) if value == probe.expected => ReadState::ConfigEnabled,
        Some(_) => ReadState::ConfigDisabled,
        None => ReadState::Unknown,
    }
}

pub fn reader_exposures(path: &Path, ctx: &AppContext) -> Result<Vec<ReaderExposure>, AppError> {
    let Some(name) = path.file_name() else {
        return Err(AppError {
            code: "invalid_target",
        });
    };
    let key = skill_io::paths::physical_key(path)?;
    let mut found = BTreeSet::new();
    for entry in &ctx.registry.entries {
        if !matches!(
            entry.state,
            ProfileState::Builtin | ProfileState::Enabled | ProfileState::Disabled
        ) {
            continue;
        }
        let Some(profile) = &entry.profile else {
            continue;
        };
        for variant in &profile.variants {
            let mut local = ctx.clone();
            local
                .registry
                .selected_variants
                .insert(profile.id.clone(), variant.clone());
            let scopes = if ctx.scope == Scope::Project {
                vec![Scope::User, Scope::Project]
            } else {
                vec![Scope::User]
            };
            for scope in scopes {
                local.scope = scope;
                for root in resolve_profile_roots(profile, &local)?.roots {
                    if skill_io::paths::physical_key(&root.path.join(name))
                        .is_ok_and(|candidate| candidate == key)
                    {
                        let rule = profile
                            .roots
                            .iter()
                            .find(|r| r.root_id == root.root_id)
                            .unwrap();
                        let evidence = if entry.state == ProfileState::Disabled {
                            ReadState::Unknown
                        } else {
                            match &rule.read_evidence {
                                ReadEvidence::Always => ReadState::Declared,
                                ReadEvidence::BoolConfigV1 { probe_id } => {
                                    probe_state(profile, probe_id, &local)
                                }
                            }
                        };
                        found.insert(ReaderExposure {
                            agent: profile.id.clone(),
                            variant: variant.clone(),
                            root_id: root.root_id,
                            evidence,
                        });
                    }
                }
            }
        }
    }
    Ok(found.into_iter().collect())
}
