use crate::{
    preview::{error, tree_bytes},
    roots::resolve_profile_roots,
    App, AppError, ProfileState,
};
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
    pub fn library_statuses(&self, library: &Path) -> Result<Vec<TargetStatus>, AppError> {
        let loaded = self.checked_library(library)?;
        let context = self.runtime_context()?;
        let (authority, _) = self.authority()?;
        let protected = authority
            .libraries
            .iter()
            .chain(&authority.backups)
            .cloned()
            .chain([context.env.home.join(".skill-hub"), loaded.path.clone()])
            .map(|p| skill_io::disk::resolved_path(&p))
            .collect::<Result<Vec<_>, _>>()?;
        let mut budget = loaded
            .bindings
            .iter()
            .map(|b| tree_bytes(&b.tree))
            .sum::<usize>();
        let mut statuses = Vec::new();
        for entry in &context.registry.entries {
            let Some(profile) = &entry.profile else {
                continue;
            };
            let active = matches!(entry.state, ProfileState::Builtin | ProfileState::Enabled)
                && self.profile_authorized(profile, &context, true).is_ok();
            let roots = if active {
                resolve_profile_roots(profile, &context)
                    .ok()
                    .map(|r| {
                        r.roots
                            .into_iter()
                            .filter(|r| r.writable)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            };
            let root = if roots.len() == 1 {
                skill_io::disk::resolved_path(&roots[0].path).ok()
            } else {
                None
            };
            for (index, record) in loaded.catalog.records.iter().enumerate() {
                if statuses.len() >= 4096 {
                    return Err(error("too_many_pairs"));
                }
                let path = root.as_ref().map(|p| p.join(&record.name));
                let state = match &path {
                    None => "unsupported",
                    Some(path) if protected.iter().any(|p| crate::settings::overlap(path, p)) => {
                        "blocked"
                    }
                    Some(path) => match skill_io::disk::optional_tree(path) {
                        Ok(None) => "missing",
                        Err(_) => "read_error",
                        Ok(Some(tree)) => {
                            budget = budget.saturating_add(tree_bytes(&tree));
                            if budget > 64 * 1024 * 1024 {
                                return Err(error("preview_too_large"));
                            }
                            if skill_core::compare_packages(&loaded.bindings[index].tree, &tree)
                                .is_equal()
                            {
                                "equal"
                            } else {
                                "different"
                            }
                        }
                    },
                };
                statuses.push(TargetStatus {
                    package_id: record.id.clone(),
                    agent_id: profile.id.clone(),
                    state,
                    path: path
                        .map(|p| skill_core::compatibility::safe_text(&p.to_string_lossy()))
                        .unwrap_or_default(),
                });
            }
        }
        Ok(statuses)
    }
}
