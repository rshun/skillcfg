use crate::{
    CoreError, EntryKind, PackageComparison, PackageIssue, PackageTree, Platform, SkillMetadata,
};
use std::collections::BTreeMap;

pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 1024;
pub const MAX_DEPTH: usize = 16;

pub fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

pub fn parse_skill_document(bytes: &[u8]) -> Result<SkillMetadata, CoreError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(CoreError::new("file_too_large"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| CoreError::new("invalid_utf8"))?;
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next() != Some("---") {
        return Err(CoreError::new("missing_frontmatter"));
    }
    let mut header = Vec::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        header.push(line);
    }
    if !closed {
        return Err(CoreError::new("unclosed_frontmatter"));
    }
    let value: serde_yaml::Value = serde_yaml::from_str(&header.join("\n"))
        .map_err(|_| CoreError::new("invalid_frontmatter"))?;
    if !value.is_mapping() {
        return Err(CoreError::new("invalid_frontmatter"));
    }
    let name = value
        .get("name")
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| is_slug(s))
        .ok_or_else(|| CoreError::new("invalid_name"))?;
    let description = value
        .get("description")
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| CoreError::new("invalid_description"))?;
    Ok(SkillMetadata {
        name: name.to_owned(),
        description: description.to_owned(),
    })
}

pub fn valid_relative_path(path: &str, platform: Platform) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', '\0'])
        || (path.as_bytes().get(1) == Some(&b':') && path.as_bytes()[0].is_ascii_alphabetic())
    {
        return false;
    }
    path.split('/').all(|part| {
        if part.is_empty() || part == "." || part == ".." {
            return false;
        }
        if platform != Platform::Windows {
            return true;
        }
        if part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return false;
        }
        let stem = part
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(' ')
            .to_ascii_uppercase();
        !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) && !["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|n| {
                matches!(
                    n,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    })
}

pub fn validate_package(tree: &PackageTree, platform: Platform) -> Vec<PackageIssue> {
    let mut issues = Vec::new();
    let mut issue = |code, path: Option<&str>| {
        issues.push(PackageIssue {
            code,
            path: path.map(crate::compatibility::safe_text),
        })
    };
    if tree.entries.len() > MAX_ENTRIES {
        issue("too_many_entries", None);
    }
    let mut total = 0usize;
    let mut paths = BTreeMap::new();
    for entry in &tree.entries {
        let path = &entry.relative_path;
        if !valid_relative_path(path, platform) {
            issue("invalid_path", Some(path));
        }
        if path.split('/').count() > MAX_DEPTH {
            issue("too_deep", Some(path));
        }
        let key = if platform == Platform::Windows {
            path.to_lowercase()
        } else {
            path.clone()
        };
        if let Some((old_path, old_kind)) = paths.insert(key, (path, &entry.kind)) {
            issue(
                if old_path == path {
                    "duplicate_path"
                } else {
                    "case_collision"
                },
                Some(path),
            );
            if std::mem::discriminant(old_kind) != std::mem::discriminant(&entry.kind) {
                issue("path_collision", Some(path));
            }
        }
        if let EntryKind::File { bytes, .. } = &entry.kind {
            total = total.saturating_add(bytes.len());
            if bytes.len() > MAX_FILE_BYTES {
                issue("file_too_large", Some(path));
            }
            if std::str::from_utf8(bytes)
                .is_ok_and(crate::compatibility::contains_sensitive_literal)
            {
                issue("sensitive_content", Some(path));
            }
        }
    }
    if total > MAX_PACKAGE_BYTES {
        issue("package_too_large", None);
    }
    for (path, _) in &paths {
        let parts: Vec<_> = path.split('/').collect();
        for depth in 1..parts.len() {
            if paths
                .get(&parts[..depth].join("/"))
                .is_some_and(|(_, kind)| matches!(kind, EntryKind::File { .. }))
            {
                issue("path_collision", Some(path));
            }
        }
    }
    match tree.entries.iter().find(|e| e.relative_path == "SKILL.md") {
        Some(crate::TreeEntry {
            kind: EntryKind::File { bytes, .. },
            ..
        }) => {
            if let Err(error) = parse_skill_document(bytes) {
                issue(error.code, Some("SKILL.md"));
            }
        }
        _ => issue("missing_skill_document", None),
    }
    issues
}

pub fn compare_packages(left: &PackageTree, right: &PackageTree) -> PackageComparison {
    let left: BTreeMap<_, _> = left
        .entries
        .iter()
        .map(|e| (&e.relative_path, &e.kind))
        .collect();
    let right: BTreeMap<_, _> = right
        .entries
        .iter()
        .map(|e| (&e.relative_path, &e.kind))
        .collect();
    let mut result = PackageComparison::default();
    for (path, kind) in &right {
        match left.get(path) {
            None => result.added.push(path.to_string()),
            Some(old) if old != kind => result.changed.push(path.to_string()),
            _ => {}
        }
    }
    for path in left.keys() {
        if !right.contains_key(path) {
            result.removed.push(path.to_string());
        }
    }
    result
}
