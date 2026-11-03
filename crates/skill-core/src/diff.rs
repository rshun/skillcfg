use crate::{
    compatibility::safe_text,
    plan::{Disposition, DistributionPlan},
    EntryKind, PackageTree, ReaderExposure,
};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeFileDiff {
    pub path: String,
    pub binary: bool,
    pub before: Option<String>,
    pub after: Option<String>,
    pub before_bytes: usize,
    pub after_bytes: usize,
    pub truncated: bool,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafePlanGroup {
    pub target_key: String,
    pub path: String,
    pub disposition: Disposition,
    pub selected_agents: Vec<String>,
    pub possible_readers: Vec<ReaderExposure>,
    pub files: Vec<SafeFileDiff>,
    pub reason: Option<&'static str>,
}
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafePlanView {
    pub groups: Vec<SafePlanGroup>,
}
pub fn redacted_plan_dto(plan: &DistributionPlan) -> SafePlanView {
    let mut budget = 1024 * 1024usize;
    SafePlanView {
        groups: plan
            .groups
            .iter()
            .map(|group| {
                let files = if group.disposition == Disposition::Blocked {
                    vec![]
                } else {
                    let mut paths = group.comparison.added.clone();
                    paths.extend(group.comparison.changed.clone());
                    paths.extend(group.comparison.removed.clone());
                    paths.sort();
                    paths.dedup();
                    paths
                        .into_iter()
                        .map(|path| {
                            let before = file_bytes(group.before.as_ref(), &path);
                            let after = file_bytes(Some(&group.after), &path);
                            let binary = [before, after]
                                .into_iter()
                                .flatten()
                                .any(|b| std::str::from_utf8(b).is_err());
                            let mut truncated = false;
                            let mut text = |value: Option<&[u8]>| -> Option<String> {
                                if binary {
                                    return None;
                                }
                                let value = std::str::from_utf8(value?).ok()?;
                                let safe = safe_text(value);
                                let limit = budget.min(64 * 1024);
                                let mut end = safe.len().min(limit);
                                while !safe.is_char_boundary(end) {
                                    end -= 1;
                                }
                                truncated |= end < safe.len();
                                budget -= end;
                                Some(safe[..end].to_owned())
                            };
                            let old = text(before);
                            let new = text(after);
                            SafeFileDiff {
                                path: safe_text(&path),
                                binary,
                                before: old,
                                after: new,
                                before_bytes: before.map_or(0, <[u8]>::len),
                                after_bytes: after.map_or(0, <[u8]>::len),
                                truncated,
                            }
                        })
                        .collect()
                };
                SafePlanGroup {
                    target_key: safe_text(&group.key.0),
                    path: safe_text(&group.path.to_string_lossy()),
                    disposition: group.disposition,
                    selected_agents: group
                        .selected_agents
                        .iter()
                        .map(|a| a.as_str().to_owned())
                        .collect(),
                    possible_readers: group.possible_readers.clone(),
                    files,
                    reason: group.reason,
                }
            })
            .collect(),
    }
}
fn file_bytes<'a>(tree: Option<&'a PackageTree>, path: &str) -> Option<&'a [u8]> {
    tree?.entries.iter().find_map(|e| {
        if e.relative_path == path {
            match &e.kind {
                EntryKind::File { bytes, .. } => Some(bytes.as_slice()),
                EntryKind::Directory => None,
            }
        } else {
            None
        }
    })
}
