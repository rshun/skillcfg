use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetKey(pub String);
#[derive(Clone)]
pub struct LibraryPackage {
    pub id: PackageId,
    pub metadata: SkillMetadata,
    pub tree: PackageTree,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectionPair {
    pub package_id: PackageId,
    pub agent: AgentId,
}
#[derive(Clone)]
pub struct TargetSnapshot {
    pub package_id: PackageId,
    pub agent: AgentId,
    pub key: TargetKey,
    pub path: PathBuf,
    pub platform: Platform,
    pub tree: Option<PackageTree>,
    pub possible_readers: Vec<ReaderExposure>,
    pub blocked: Option<&'static str>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictDecision {
    Skip,
    Replace,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Disposition {
    New,
    Update,
    Unchanged,
    Skipped,
    Blocked,
}
#[derive(Clone)]
pub struct PlanGroup {
    pub key: TargetKey,
    pub path: PathBuf,
    pub package_ids: Vec<PackageId>,
    pub selected_agents: Vec<AgentId>,
    pub possible_readers: Vec<ReaderExposure>,
    pub before: Option<PackageTree>,
    pub after: Arc<PackageTree>,
    pub disposition: Disposition,
    pub reason: Option<&'static str>,
    pub comparison: PackageComparison,
}
#[derive(Clone, Default)]
pub struct DistributionPlan {
    pub groups: Vec<PlanGroup>,
}
pub fn plan_distribution(
    packages: &[LibraryPackage],
    targets: &[TargetSnapshot],
    selection: &[SelectionPair],
    decisions: &BTreeMap<TargetKey, ConflictDecision>,
) -> Result<DistributionPlan, CoreError> {
    let selected: BTreeSet<_> = selection.iter().cloned().collect();
    if selected.is_empty() {
        return Ok(DistributionPlan::default());
    }
    if selected.len() > 4096 {
        return Err(CoreError::new("too_many_pairs"));
    }
    let mut package_map = BTreeMap::new();
    for package in packages {
        if package_map.insert(package.id.clone(), package).is_some() {
            return Err(CoreError::new("duplicate_package_id"));
        }
    }
    let mut snapshots = BTreeMap::new();
    let mut sources = BTreeSet::new();
    let mut target_map = BTreeMap::new();
    let mut bytes = 0usize;
    for target in targets {
        let pair = SelectionPair {
            package_id: target.package_id.clone(),
            agent: target.agent.clone(),
        };
        if selected.contains(&pair) && target_map.insert(pair, target).is_some() {
            return Err(CoreError::new("ambiguous_target"));
        }
    }
    for pair in &selected {
        let package = package_map
            .get(&pair.package_id)
            .ok_or(CoreError::new("unknown_package"))?;
        if sources.insert(package.id.clone()) {
            bytes = bytes.saturating_add(tree_bytes(&package.tree));
        }
        let target = *target_map.get(pair).ok_or(CoreError::new("missing_target"))?;
        if snapshots.insert(target.key.clone(), true).is_none() {
            if let Some(tree) = &target.tree {
                bytes = bytes.saturating_add(tree_bytes(tree));
            }
        }
        if bytes > 64 * 1024 * 1024 {
            return Err(CoreError::new("preview_too_large"));
        }
    }
    // No raw content is cloned until the complete request fits the budget.
    let mut source_trees = BTreeMap::new();
    for id in sources {
        let package = package_map[&id];
        let parsed = package.tree.entries.iter().find_map(|e| {
            if e.relative_path == "SKILL.md" {
                if let EntryKind::File { bytes, .. } = &e.kind {
                    parse_skill_document(bytes).ok()
                } else {
                    None
                }
            } else {
                None
            }
        });
        if parsed.as_ref() != Some(&package.metadata) {
            return Err(CoreError::new("metadata_mismatch"));
        }
        source_trees.insert(id, Arc::new(package.tree.clone()));
    }
    let mut groups: BTreeMap<TargetKey, PlanGroup> = BTreeMap::new();
    for pair in selected {
        let package = package_map[&pair.package_id];
        let target = target_map[&pair];
        if let Some(group) = groups.get_mut(&target.key) {
            group.package_ids.push(pair.package_id);
            group.selected_agents.push(pair.agent);
            group
                .possible_readers
                .extend(target.possible_readers.clone());
            if group.after.as_ref() != &package.tree {
                group.disposition = Disposition::Blocked;
                group.reason = Some("shared_content_conflict");
            }
            if group.before != target.tree {
                group.disposition = Disposition::Blocked;
                group.reason = Some("shared_baseline_changed");
            }
            if let Some(issue) = validate_package(&package.tree, target.platform).first() {
                group.disposition = Disposition::Blocked;
                group.reason = Some(issue.code);
            }
            if let Some(reason) = target.blocked {
                group.disposition = Disposition::Blocked;
                group.reason = Some(reason);
            }
            continue;
        }
        let comparison = compare_packages(
            target.tree.as_ref().unwrap_or(&PackageTree::default()),
            &package.tree,
        );
        let invalid = validate_package(&package.tree, target.platform)
            .first()
            .map(|i| i.code);
        let sensitive_target=target.tree.as_ref().is_some_and(|t|t.entries.iter().any(|e|matches!(&e.kind,EntryKind::File{bytes,..} if std::str::from_utf8(bytes).is_ok_and(compatibility::contains_sensitive_literal))));
        let reason = target.blocked.or(invalid).or(if sensitive_target {
            Some("sensitive_target")
        } else {
            None
        });
        let disposition = if reason.is_some() {
            Disposition::Blocked
        } else if decisions.get(&target.key) == Some(&ConflictDecision::Skip) {
            Disposition::Skipped
        } else if target.tree.is_none() {
            Disposition::New
        } else if comparison.is_equal() {
            Disposition::Unchanged
        } else if decisions.get(&target.key) == Some(&ConflictDecision::Replace) {
            Disposition::Update
        } else {
            Disposition::Skipped
        };
        groups.insert(
            target.key.clone(),
            PlanGroup {
                key: target.key.clone(),
                path: target.path.clone(),
                package_ids: vec![pair.package_id.clone()],
                selected_agents: vec![pair.agent],
                possible_readers: target.possible_readers.clone(),
                before: target.tree.clone(),
                after: source_trees[&pair.package_id].clone(),
                disposition,
                reason,
                comparison,
            },
        );
    }
    for group in groups.values_mut() {
        group.package_ids.sort();
        group.package_ids.dedup();
        group.selected_agents.sort();
        group.selected_agents.dedup();
        group.possible_readers.sort();
        group.possible_readers.dedup();
    }
    Ok(DistributionPlan {
        groups: groups.into_values().collect(),
    })
}

fn tree_bytes(tree: &PackageTree) -> usize {
    tree.entries.iter().fold(0usize, |total, e| {
        total.saturating_add(match &e.kind {
            EntryKind::File { bytes, .. } => bytes.len(),
            EntryKind::Directory => 0,
        })
    })
}
