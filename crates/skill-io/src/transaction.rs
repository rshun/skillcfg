use crate::IoError;
use skill_core::PackageTree;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct DirectoryChange {
    pub path: PathBuf,
    pub before: Option<PackageTree>,
    pub after: Option<PackageTree>,
}
#[derive(Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}
#[derive(Clone, Default)]
pub struct MutationPlan {
    pub recovery_lock: Option<crate::lock::RecoveryLock>,
    pub retained: Vec<(PathBuf, PackageTree)>,
    pub directories: Vec<DirectoryChange>,
    pub files: Vec<FileChange>,
    pub kind: String,
    pub metadata: std::collections::BTreeMap<String, String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Written,
    Unchanged,
    Skipped,
    Blocked,
    FailedRestored,
    RecoveryRequired,
    AuditFailed,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemReceipt {
    pub path: PathBuf,
    pub outcome: Outcome,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationReceipt {
    pub operation_id: String,
    pub backup_location: PathBuf,
    pub outcomes: Vec<ItemReceipt>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryRecord {
    pub operation_id: String,
    pub kind: String,
    pub state: String,
    pub paths: Vec<PathBuf>,
    pub reason: Option<String>,
}
pub type HistoryDetails = Vec<(MutationReceipt, std::collections::BTreeMap<String, String>)>;
pub struct RestoreSources {
    pub directories: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    BackedUp,
    Archived,
    Activated,
    BeforeVerify,
    Verified,
    Audit,
}
pub trait Observer: Send + Sync {
    fn event(&self, _step: Step, _path: &std::path::Path) -> Result<(), IoError> {
        Ok(())
    }
}
pub struct Quiet;
impl Observer for Quiet {}
#[derive(Clone)]
pub struct IoEngine {
    pub backup_root: PathBuf,
    pub audit_path: PathBuf,
    pub write_roots: Vec<PathBuf>,
    pub write_files: Vec<PathBuf>,
    pub write_file_roots: Vec<PathBuf>,
    pub scan_roots: Vec<PathBuf>,
    pub protected_roots: Vec<PathBuf>,
    pub observer: Arc<dyn Observer>,
}
pub struct PreparedMutation {
    inner: Option<Prepared>,
    unchanged: Vec<ItemReceipt>,
}
struct Prepared {
    engine: IoEngine,
    plan: MutationPlan,
    journal: crate::journal::Journal,
    location: PathBuf,
    _lock: crate::lock::Lock,
}

use crate::{
    disk::*,
    journal::{anchor, anchor_valid, Item, Journal},
    lock::Lock,
};
use std::{fs, io::Write, path::Path};

impl IoEngine {
    fn target(&self, path: &Path, directory: bool) -> Result<PathBuf, IoError> {
        let actual = resolved_path(path)?;
        if fs::symlink_metadata(path).is_ok() && crate::paths::is_link(path)? {
            return Err(err("target_link"));
        }
        if overlaps(&actual, &resolved_path(&self.backup_root)?)
            || actual == resolved_path(&self.audit_path)?
        {
            return Err(err("protected_target"));
        }
        if directory {
            let allowed = self.write_roots.iter().any(|root| {
                resolved_path(root).is_ok_and(|r| actual != r && actual.starts_with(r))
            });
            if !allowed
                || self
                    .protected_roots
                    .iter()
                    .any(|r| resolved_path(r).is_ok_and(|r| overlaps(&actual, &r)))
            {
                return Err(err("unauthorized_target"));
            }
        } else if !(self
            .write_files
            .iter()
            .any(|p| resolved_path(p).is_ok_and(|p| p == actual))
            || self.write_file_roots.iter().any(|root| {
                resolved_path(root).is_ok_and(|root| {
                    actual.parent() == Some(root.as_path())
                        && actual.extension().is_some_and(|e| e == "json")
                })
            }))
            || self
                .scan_roots
                .iter()
                .any(|r| resolved_path(r).is_ok_and(|r| actual.starts_with(r)))
        {
            return Err(err("unauthorized_file"));
        }
        Ok(actual)
    }
    fn work(
        &self,
        path: &Path,
        directory: bool,
        id: &str,
        index: usize,
    ) -> Result<PathBuf, IoError> {
        let base = if directory {
            self.write_roots
                .iter()
                .filter_map(|r| resolved_path(r).ok())
                .filter(|r| path != r && path.starts_with(r))
                .max_by_key(|r| r.components().count())
                .and_then(|r| r.parent().map(Path::to_owned))
                .ok_or(err("unauthorized_target"))?
        } else {
            path.parent().ok_or(err("invalid_path"))?.to_owned()
        };
        let work = base
            .join(".skill-hub-transactions")
            .join(id)
            .join(index.to_string());
        if self
            .scan_roots
            .iter()
            .any(|r| resolved_path(r).is_ok_and(|r| overlaps(&work, &r)))
            || overlaps(&work, &resolved_path(&self.backup_root)?)
        {
            return Err(err("unsafe_staging"));
        }
        Ok(work)
    }
    pub fn prepare(&self, plan: &MutationPlan) -> Result<PreparedMutation, IoError> {
        if plan.retained.len() > 1024
            || plan.directories.len() + plan.files.len() > 4096
            || plan.kind.len() > 128
            || !plan
                .kind
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-:".contains(&b))
        {
            return Err(err("invalid_plan"));
        }
        let mut bytes = 0usize;
        let mut paths: Vec<PathBuf> = Vec::new();
        let mut items = Vec::new();
        let mut unchanged = Vec::new();
        let id = operation_id();
        for change in &plan.directories {
            bytes = bytes
                .saturating_add(change.before.as_ref().map_or(0, tree_size))
                .saturating_add(change.after.as_ref().map_or(0, tree_size));
        }
        for change in &plan.files {
            bytes = bytes
                .saturating_add(change.before.as_ref().map_or(0, Vec::len))
                .saturating_add(change.after.as_ref().map_or(0, Vec::len));
        }
        for (_, tree) in &plan.retained {
            bytes = bytes.saturating_add(tree_size(tree));
        }
        if bytes > 64 * 1024 * 1024 {
            return Err(err("preview_too_large"));
        }
        let mut effective = MutationPlan {
            kind: plan.kind.clone(),
            recovery_lock: plan.recovery_lock.clone(),
            metadata: plan.metadata.clone(),
            ..Default::default()
        };
        for (path, tree) in &plan.retained {
            let actual = self.target(path, true)?;
            if optional_tree(&actual)?.as_ref() != Some(tree) {
                return Err(err("retained_baseline_changed"));
            }
            effective.retained.push((actual, tree.clone()));
        }
        for (path, directory, equal) in plan
            .directories
            .iter()
            .map(|c| (&c.path, true, c.before == c.after))
            .chain(
                plan.files
                    .iter()
                    .map(|c| (&c.path, false, c.before == c.after)),
            )
        {
            let actual = self.target(path, directory)?;
            if paths.iter().any(|p| overlaps(&actual, p)) {
                return Err(err("overlapping_targets"));
            }
            paths.push(actual.clone());
            let index = items.len();
            let work = self.work(&actual, directory, &id, index)?;
            if paths.iter().any(|p| overlaps(&work, p)) {
                return Err(err("unsafe_staging"));
            }
            if equal {
                unchanged.push(ItemReceipt {
                    path: actual,
                    outcome: Outcome::Unchanged,
                    reason: None,
                });
                continue;
            }
            let (anchor, anchor_key) = anchor(&actual)?;
            items.push(Item {
                path: actual,
                directory,
                before: false,
                after: false,
                state: "preparing".into(),
                work,
                baseline_key: None,
                activated_key: None,
                anchor,
                anchor_key,
            });
        }
        // Rebuild in the same directory-then-file order, dropping no-op entries.
        for change in &plan.directories {
            if change.before != change.after {
                let mut c = change.clone();
                c.path = self.target(&c.path, true)?;
                if optional_tree(&c.path)? != c.before {
                    return Err(err("baseline_changed"));
                }
                effective.directories.push(c);
            }
        }
        for change in &plan.files {
            if change.before != change.after {
                let mut c = change.clone();
                c.path = self.target(&c.path, false)?;
                if optional_file(&c.path)? != c.before {
                    return Err(err("baseline_changed"));
                }
                effective.files.push(c);
            }
        }
        for (i, item) in items.iter_mut().enumerate() {
            let (before, after) = if item.directory {
                let c = &effective.directories[i];
                (c.before.is_some(), c.after.is_some())
            } else {
                let c = &effective.files[i - effective.directories.len()];
                (c.before.is_some(), c.after.is_some())
            };
            item.before = before;
            item.after = after;
            if before {
                item.baseline_key = Some(crate::paths::physical_key(&item.path)?);
            }
        }
        if items.is_empty()
            && plan.recovery_lock.is_none()
            && !plan.metadata.contains_key("abandonPreparation")
        {
            return Ok(PreparedMutation {
                inner: None,
                unchanged,
            });
        }
        for item in &items {
            if paths.iter().any(|p| overlaps(p, &item.work)) {
                return Err(err("unsafe_staging"));
            }
        }
        let location = resolved_path(&self.backup_root)?.join(&id);
        directories(location.parent().ok_or(err("invalid_path"))?)?;
        let initializing = location.with_file_name(format!(".initializing-{id}"));
        new_directory(&initializing)?;
        let lock_path = resolved_path(&self.audit_path)?
            .parent()
            .and_then(Path::parent)
            .ok_or(err("invalid_path"))?
            .join("write.lock");
        let mut journal = Journal {
            retained: effective.retained.iter().map(|(p, _)| p.clone()).collect(),
            version: 1,
            id,
            kind: plan.kind.clone(),
            state: "preparing".into(),
            items,
            receipt: None,
            metadata: plan.metadata.clone(),
        };
        persist(&initializing, &journal)?;
        fs::rename(&initializing, &location).map_err(|_| err("backup_publish_failed"))?;
        sync_parent(&location)?;
        if let Some(evidence) = &plan.recovery_lock {
            if !plan
                .kind
                .strip_prefix("restore:")
                .is_some_and(|id| evidence.matches(id))
            {
                journal.state = "aborted".into();
                persist(&location, &journal)?;
                return Err(err("invalid_recovery_lock"));
            }
        }
        let lock = match Lock::acquire(
            &lock_path,
            &location.join("released.lock"),
            &journal.id,
            plan.recovery_lock.as_ref(),
        ) {
            Ok(lock) => lock,
            Err(e) => {
                journal.state = "aborted".into();
                persist(&location, &journal)?;
                return Err(e);
            }
        };
        let result = (|| {
            for (i, c) in effective.directories.iter().enumerate() {
                if let Some(t) = &c.before {
                    write_tree(&location.join(format!("original/{i}")), t)?;
                }
                if let Some(t) = &c.after {
                    write_tree(&location.join(format!("planned/{i}")), t)?;
                }
            }
            for (i, c) in effective.files.iter().enumerate() {
                if let Some(t) = &c.before {
                    write_blob(&location.join(format!("original-files/{i}")), t)?;
                }
                if let Some(t) = &c.after {
                    write_blob(&location.join(format!("planned-files/{i}")), t)?;
                }
            }
            for (i, (_, tree)) in effective.retained.iter().enumerate() {
                write_tree(&location.join(format!("retained/{i}")), tree)?;
            }
            for item in &journal.items {
                self.observer.event(Step::BackedUp, &item.path)?;
            }
            for (i, item) in journal.items.iter_mut().enumerate() {
                directories(&item.work)?;
                if item.directory {
                    if let Some(tree) = &effective.directories[i].after {
                        write_tree(&item.work.join("staged"), tree)?;
                    }
                } else if let Some(data) = &effective.files[i - effective.directories.len()].after {
                    write_new(&item.work.join("staged-file"), data)?;
                }
                item.state = "prepared".into();
            }
            journal.state = "prepared".into();
            persist(&location, &journal)
        })();
        if let Err(error) = result {
            journal.state = "aborted".into();
            let _ = persist(&location, &journal);
            return Err(error);
        }
        Ok(PreparedMutation {
            inner: Some(Prepared {
                engine: self.clone(),
                plan: effective,
                journal,
                location,
                _lock: lock,
            }),
            unchanged,
        })
    }
    pub fn apply(&self, prepared: PreparedMutation) -> MutationReceipt {
        let Some(mut p) = prepared.inner else {
            return MutationReceipt {
                operation_id: String::new(),
                backup_location: PathBuf::new(),
                outcomes: prepared.unchanged,
            };
        };
        let mut outcomes = prepared.unchanged;
        let mut directory_failed = false;
        for index in 0..p.journal.items.len() {
            let item = p.journal.items[index].clone();
            if !item.directory && directory_failed {
                outcomes.push(ItemReceipt {
                    path: item.path,
                    outcome: Outcome::Blocked,
                    reason: Some("prior_commit_failed".into()),
                });
                continue;
            }
            let result = commit_item(&mut p, index);
            let outcome = match result {
                Ok(()) => Outcome::Written,
                Err(error) => {
                    let current = &p.journal.items[index];
                    let outcome = if current.state == "prepared" {
                        Outcome::Blocked
                    } else if error.code != "interrupted" && rollback(&mut p, index).is_ok() {
                        Outcome::FailedRestored
                    } else {
                        Outcome::RecoveryRequired
                    };
                    p.journal.items[index].state = if outcome == Outcome::Blocked {
                        "blocked"
                    } else if outcome == Outcome::FailedRestored {
                        "rolled_back"
                    } else {
                        "recovery_required"
                    }
                    .into();
                    directory_failed = true;
                    outcomes.push(ItemReceipt {
                        path: item.path,
                        outcome,
                        reason: Some(error.code.into()),
                    });
                    let _ = persist(&p.location, &p.journal);
                    continue;
                }
            };
            outcomes.push(ItemReceipt {
                path: item.path,
                outcome,
                reason: None,
            });
        }
        p.journal.state = if outcomes
            .iter()
            .all(|o| matches!(o.outcome, Outcome::Written | Outcome::Unchanged))
        {
            "verified"
        } else {
            "recovery_required"
        }
        .into();
        let mut receipt = MutationReceipt {
            operation_id: p.journal.id.clone(),
            backup_location: p.location.clone(),
            outcomes,
        };
        p.journal.receipt = Some(receipt.clone());
        let recorded = persist(&p.location, &p.journal);
        let audited = recorded
            .and_then(|_| p.engine.observer.event(Step::Audit, &p.engine.audit_path))
            .and_then(|_| audit(&p.engine.audit_path, &receipt));
        if let Err(error) = audited {
            receipt.outcomes.push(ItemReceipt {
                path: p.engine.audit_path.clone(),
                outcome: Outcome::AuditFailed,
                reason: Some(error.code.into()),
            });
            p.journal.receipt = Some(receipt.clone());
            let _ = persist(&p.location, &p.journal);
        }
        if p.journal.state == "verified" {
            if let Some(previous) = p.journal.kind.strip_prefix("restore:") {
                let mut source = self.clone();
                if let Some(root) = p.plan.metadata.get("restoreBackupRoot") {
                    source.backup_root = PathBuf::from(root);
                }
                if let Ok((location, mut old)) = source.manifest(previous) {
                    old.state = "resolved".into();
                    let _ = persist(&location, &old);
                }
            }
        }
        if let Err(error) = p._lock.release() {
            receipt.outcomes.push(ItemReceipt {
                path: lock_path(&p.engine).unwrap_or_else(|_| p.engine.audit_path.clone()),
                outcome: Outcome::AuditFailed,
                reason: Some(error.code.into()),
            });
            p.journal.receipt = Some(receipt.clone());
            let _ = persist(&p.location, &p.journal);
        }
        receipt
    }
    fn read_manifest(&self, id: &str) -> Result<(PathBuf, Journal), IoError> {
        if !skill_core::package::is_slug(id) {
            return Err(err("invalid_operation_id"));
        }
        let location = resolved_path(&self.backup_root)?.join(id);
        if crate::paths::is_link(&location)? || resolved_path(&location)? != location {
            return Err(err("invalid_backup"));
        }
        let journal: Journal = read_json(&location.join("manifest.json"))?;
        if journal.version != 1 || journal.id != id || journal.items.len() > 4096 {
            return Err(err("invalid_journal"));
        }
        if journal
            .items
            .iter()
            .any(|i| !i.path.is_absolute() || !i.work.is_absolute())
        {
            return Err(err("invalid_journal_path"));
        }
        Ok((location, journal))
    }
    fn manifest(&self, id: &str) -> Result<(PathBuf, Journal), IoError> {
        let (location, journal) = self.read_manifest(id)?;
        for (index, item) in journal.items.iter().enumerate() {
            if self.target(&item.path, item.directory)? != item.path
                || self.work(&item.path, item.directory, id, index)? != item.work
            {
                return Err(err("invalid_journal_path"));
            }
        }
        if journal.retained.len() > 1024 {
            return Err(err("invalid_journal"));
        }
        for path in &journal.retained {
            self.target(path, true)?;
        }
        Ok((location, journal))
    }
    fn journals(&self) -> Result<Vec<(PathBuf, Journal)>, IoError> {
        let entries = match fs::read_dir(&self.backup_root) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(_) => return Err(err("backups_unreadable")),
        };
        let mut records = Vec::new();
        let mut metadata_bytes = 0usize;
        for entry in entries {
            let entry = entry.map_err(|_| err("backups_unreadable"))?;
            if records.len() >= 4096 {
                return Err(err("too_many_backups"));
            }
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| err("invalid_backup"))?;
            if id.starts_with(".initializing-") {
                continue;
            }
            let record = self.read_manifest(&id)?;
            metadata_bytes = metadata_bytes.saturating_add(
                serde_json::to_vec(&record.1)
                    .map_err(|_| err("invalid_journal"))?
                    .len(),
            );
            if metadata_bytes > 64 * 1024 * 1024 {
                return Err(err("history_too_large"));
            }
            records.push(record);
        }
        records.sort_by(|a, b| a.1.id.cmp(&b.1.id));
        Ok(records)
    }
    pub fn history_details(&self) -> Result<HistoryDetails, IoError> {
        Ok(self
            .journals()?
            .into_iter()
            .filter_map(|(_, mut j)| {
                j.metadata.insert(
                    "kind".into(),
                    j.kind.split(':').next().unwrap_or("mutation").to_owned(),
                );
                j.receipt.map(|r| (r, j.metadata))
            })
            .collect())
    }
    pub fn restore_sources(&self, id: &str) -> Result<RestoreSources, IoError> {
        let (location, j) = self.manifest(id)?;
        let mut result = RestoreSources {
            directories: vec![],
            files: vec![],
        };
        if matches!(j.state.as_str(), "preparing" | "aborted") {
            return Ok(result);
        }
        let mut di = 0;
        let mut fi = 0;
        for item in j.items {
            if item.directory {
                if item.before {
                    result
                        .directories
                        .push(location.join(format!("original/{di}")));
                }
                di += 1;
            } else {
                if item.before {
                    result
                        .files
                        .push(location.join(format!("original-files/{fi}")));
                }
                fi += 1;
            }
        }
        for i in 0..j.retained.len() {
            result
                .directories
                .push(location.join(format!("retained/{i}")));
        }
        Ok(result)
    }
    pub fn list_history(&self) -> Result<Vec<MutationReceipt>, IoError> {
        Ok(self
            .journals()?
            .into_iter()
            .filter_map(|(_, j)| j.receipt)
            .collect())
    }
    pub fn list_recoveries(&self) -> Result<Vec<RecoveryRecord>, IoError> {
        let stale = crate::lock::stale_owner(&lock_path(self)?).ok().flatten();
        Ok(self
            .journals()?
            .into_iter()
            .filter(|(_, j)| {
                stale.as_ref() == Some(&j.id)
                    || !matches!(j.state.as_str(), "verified" | "resolved" | "aborted")
            })
            .map(|(location, j)| RecoveryRecord {
                operation_id: j.id,
                kind: j.kind,
                state: j.state,
                paths: j
                    .items
                    .into_iter()
                    .flat_map(|i| [i.path, i.work])
                    .chain([location])
                    .chain(lock_path(self).ok())
                    .collect(),
                reason: Some("explicit_recovery_required".into()),
            })
            .collect())
    }
    pub fn plan_restore(&self, id: &str) -> Result<MutationPlan, IoError> {
        let (location, journal) = self.manifest(id)?;
        let recovery_lock = crate::lock::probe(&lock_path(self)?, id)?;
        let mut plan = MutationPlan {
            kind: format!("restore:{id}"),
            recovery_lock,
            ..Default::default()
        };
        plan.metadata.insert("restoredKind".into(),journal.kind.clone());
        plan.metadata.insert(
            "restoreBackupRoot".into(),
            resolved_path(&self.backup_root)?
                .to_str()
                .ok_or(err("non_utf8_path"))?
                .to_owned(),
        );
        if matches!(journal.state.as_str(), "preparing" | "aborted") {
            plan.metadata
                .insert("abandonPreparation".into(), "true".into());
            return Ok(plan);
        }
        let mut di = 0;
        let mut fi = 0;
        let mut restored_bytes = 0usize;
        for (i, path) in journal.retained.iter().enumerate() {
            let backup = location.join(format!("retained/{i}"));
            if !resolved_path(&backup)?.starts_with(&location) {
                return Err(err("invalid_backup"));
            }
            let tree = optional_tree(&backup)?.ok_or(err("incomplete_backup"))?;
            let current = optional_tree(path)?;
            restored_bytes = restored_bytes
                .saturating_add(tree_size(&tree))
                .saturating_add(current.as_ref().map_or(0, tree_size));
            if restored_bytes > 64 * 1024 * 1024 {
                return Err(err("preview_too_large"));
            }
            plan.directories.push(DirectoryChange {
                path: path.clone(),
                before: current,
                after: Some(tree),
            });
        }
        for item in journal.items {
            if item.directory {
                let original = if item.before {
                    let path = location.join(format!("original/{di}"));
                    if !resolved_path(&path)?.starts_with(&location) {
                        return Err(err("invalid_backup"));
                    }
                    Some(optional_tree(&path)?.ok_or(err("incomplete_backup"))?)
                } else {
                    None
                };
                let current = optional_tree(&item.path)?;
                restored_bytes = restored_bytes
                    .saturating_add(current.as_ref().map_or(0, tree_size))
                    .saturating_add(original.as_ref().map_or(0, tree_size));
                if restored_bytes > 64 * 1024 * 1024 {
                    return Err(err("preview_too_large"));
                }
                plan.directories.push(DirectoryChange {
                    path: item.path.clone(),
                    before: current,
                    after: original,
                });
                di += 1;
            } else {
                let original = if item.before {
                    let path = location.join(format!("original-files/{fi}"));
                    if !resolved_path(&path)?.starts_with(&location) {
                        return Err(err("invalid_backup"));
                    }
                    Some(crate::snapshot::read_bounded_file(&path, 64 * 1024 * 1024)?)
                } else {
                    None
                };
                let current = optional_file(&item.path)?;
                restored_bytes = restored_bytes
                    .saturating_add(current.as_ref().map_or(0, Vec::len))
                    .saturating_add(original.as_ref().map_or(0, Vec::len));
                if restored_bytes > 64 * 1024 * 1024 {
                    return Err(err("preview_too_large"));
                }
                plan.files.push(FileChange {
                    path: item.path.clone(),
                    before: current,
                    after: original,
                });
                fi += 1;
            }
        }
        Ok(plan)
    }
}
fn persist(location: &Path, journal: &Journal) -> Result<(), IoError> {
    if serde_json::to_vec(journal)
        .map_err(|_| err("invalid_journal"))?
        .len()
        > 4 * 1024 * 1024
    {
        return Err(err("journal_too_large"));
    }
    json(&location.join("manifest.json"), journal)
}
fn write_blob(path: &Path, bytes: &[u8]) -> Result<(), IoError> {
    directories(path.parent().ok_or(err("invalid_path"))?)?;
    write_new(path, bytes)?;
    sync_parent(path)
}
fn baseline(p: &Prepared, index: usize) -> Result<(), IoError> {
    let item = &p.journal.items[index];
    if !anchor_valid(item) {
        return Err(err("path_changed"));
    }
    if item.directory {
        if optional_tree(&item.path)? != p.plan.directories[index].before {
            return Err(err("baseline_changed"));
        }
    } else if optional_file(&item.path)? != p.plan.files[index - p.plan.directories.len()].before {
        return Err(err("baseline_changed"));
    }
    if let Some(key) = &item.baseline_key {
        if &crate::paths::physical_key(&item.path)? != key {
            return Err(err("path_changed"));
        }
    }
    Ok(())
}
fn commit_item(p: &mut Prepared, index: usize) -> Result<(), IoError> {
    p._lock.valid()?;
    baseline(p, index)?;
    let item = p.journal.items[index].clone();
    directories(item.path.parent().ok_or(err("invalid_path"))?)?;
    if item.before && (item.directory || !item.after) {
        p.journal.items[index].state = "archive_pending".into();
        persist(&p.location, &p.journal)?;
        fs::rename(&item.path, item.work.join("original")).map_err(|_| err("archive_failed"))?;
        sync_parent(&item.path)?;
        p.journal.items[index].state = "archived".into();
        persist(&p.location, &p.journal)?;
        p.engine.observer.event(Step::Archived, &item.path)?;
    }
    if item.after {
        let staged = item.work.join(if item.directory {
            "staged"
        } else {
            "staged-file"
        });
        p.journal.items[index].activated_key = Some(crate::paths::physical_key(&staged)?);
        p.journal.items[index].state = "activation_pending".into();
        persist(&p.location, &p.journal)?;
        if item.directory && fs::symlink_metadata(&item.path).is_ok() {
            return Err(err("target_appeared"));
        }
        fs::rename(&staged, &item.path).map_err(|_| err("activate_failed"))?;
        sync_parent(&item.path)?;
        p.journal.items[index].state = "activated".into();
        persist(&p.location, &p.journal)?;
        p.engine.observer.event(Step::Activated, &item.path)?;
    }
    p.engine.observer.event(Step::BeforeVerify, &item.path)?;
    if !anchor_valid(&item) {
        return Err(err("path_changed"));
    }
    if item.directory {
        if optional_tree(&item.path)? != p.plan.directories[index].after {
            return Err(err("verify_failed"));
        }
    } else if optional_file(&item.path)? != p.plan.files[index - p.plan.directories.len()].after {
        return Err(err("verify_failed"));
    }
    if let Some(key) = &p.journal.items[index].activated_key {
        if &crate::paths::physical_key(&item.path)? != key {
            return Err(err("path_changed"));
        }
    }
    p.journal.items[index].state = "verified".into();
    persist(&p.location, &p.journal)?;
    p.engine.observer.event(Step::Verified, &item.path)
}
fn rollback(p: &mut Prepared, index: usize) -> Result<(), IoError> {
    let item = p.journal.items[index].clone();
    if !anchor_valid(&item) {
        return Err(err("path_changed"));
    }
    if !item.directory {
        return Err(err("file_recovery_required"));
    }
    let change = &p.plan.directories[index];
    let current = optional_tree(&item.path)?;
    if item.before {
        let archived = item.work.join("original");
        if optional_tree(&archived)? != change.before
            || item.baseline_key.as_ref() != Some(&crate::paths::physical_key(&archived)?)
        {
            return Err(err("archive_changed"));
        }
    }
    if current.is_some() {
        if current != change.after
            || item.activated_key.as_ref() != Some(&crate::paths::physical_key(&item.path)?)
        {
            return Err(err("external_change"));
        }
        fs::rename(&item.path, item.work.join("failed-product"))
            .map_err(|_| err("rollback_failed"))?;
    }
    if item.before {
        fs::rename(item.work.join("original"), &item.path).map_err(|_| err("rollback_failed"))?;
    }
    sync_parent(&item.path)
}
fn audit(path: &Path, receipt: &MutationReceipt) -> Result<(), IoError> {
    directories(path.parent().ok_or(err("invalid_path"))?)?;
    if fs::symlink_metadata(path).is_ok() && crate::paths::is_link(path)? {
        return Err(err("audit_link"));
    }
    let safe = serde_json::json!({"operationId":receipt.operation_id,"outcomes":receipt.outcomes.iter().map(|i|serde_json::json!({"path":skill_core::compatibility::safe_text(&i.path.to_string_lossy()),"outcome":i.outcome,"reason":i.reason})).collect::<Vec<_>>()});
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| err("audit_failed"))?;
    serde_json::to_writer(&mut file, &safe).map_err(|_| err("audit_failed"))?;
    file.write_all(b"\n")
        .and_then(|_| file.sync_all())
        .map_err(|_| err("audit_failed"))
}

fn lock_path(engine: &IoEngine) -> Result<PathBuf, IoError> {
    Ok(resolved_path(&engine.audit_path)?
        .parent()
        .and_then(Path::parent)
        .ok_or(err("invalid_path"))?
        .join("write.lock"))
}
