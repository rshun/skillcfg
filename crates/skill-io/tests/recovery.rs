mod tx_support;
use skill_io::*;
use std::{fs, path::Path, sync::Arc};
use tx_support::*;

struct ExitAt(Step);
impl Observer for ExitAt {
    fn event(&self, step: Step, _: &Path) -> Result<(), IoError> {
        if step == self.0 {
            std::process::exit(71);
        }
        Ok(())
    }
}
#[test]
fn real_crash_child() {
    if let Some(root) = std::env::var_os("SKILL_HUB_CRASH_ROOT") {
        let root = std::path::PathBuf::from(root);
        let mut engine = engine_at(&root);
        if let Some(backup) = std::env::var_os("SKILL_HUB_BACKUP_ROOT") {
            engine.backup_root = backup.into();
        }
        engine.observer = Arc::new(ExitAt(
            if std::env::var_os("SKILL_HUB_AUDIT_CRASH").is_some() {
                Step::Audit
            } else if std::env::var_os("SKILL_HUB_PREPARE_CRASH").is_some() {
                Step::BackedUp
            } else {
                Step::Archived
            },
        ));
        let target = root.join("agent/skills/review");
        let prepared = engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap();
        engine.apply(prepared);
        panic!("crash point was not reached");
    }
}
fn crash_fixture(preparing: bool) -> (IoEngine, std::path::PathBuf) {
    let (engine, target) = fixture();
    let root = engine.backup_root.parent().unwrap().parent().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "real_crash_child"])
        .env("SKILL_HUB_CRASH_ROOT", root);
    if preparing {
        child.env("SKILL_HUB_PREPARE_CRASH", "1");
    }
    assert_eq!(child.status().unwrap().code(), Some(71));
    assert!(root.join("hub/write.lock").exists());
    (engine, target)
}
#[test]
fn real_process_crash_requires_explicit_preview_and_preserves_lock_record() {
    let (engine, target) = crash_fixture(false);
    assert!(!target.exists());
    let records = engine.list_recoveries().unwrap();
    assert_eq!(records.len(), 1);
    let restore = engine.plan_restore(&records[0].operation_id).unwrap();
    assert!(!target.exists());
    let receipt = engine.apply(engine.prepare(&restore).unwrap());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
    assert!(receipt.backup_location.join("recovered.lock").exists());
    assert!(engine.list_recoveries().unwrap().is_empty());
}
#[test]
fn unfinished_preparation_can_be_abandoned_without_touching_later_edits() {
    let (engine, target) = crash_fixture(true);
    fs::write(target.join("later"), b"later").unwrap();
    let records = engine.list_recoveries().unwrap();
    let restore = engine.plan_restore(&records[0].operation_id).unwrap();
    let receipt = engine.apply(engine.prepare(&restore).unwrap());
    assert!(receipt.backup_location.join("recovered.lock").exists());
    assert_eq!(fs::read(target.join("later")).unwrap(), b"later");
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
    assert!(engine.list_recoveries().unwrap().is_empty());
}
#[test]
fn recovery_does_not_take_over_an_active_operation() {
    let (engine, target) = fixture();
    let prepared = engine
        .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
        .unwrap();
    let records = engine.list_recoveries().unwrap();
    assert!(engine.plan_restore(&records[0].operation_id).is_err());
    engine.apply(prepared);
}
#[test]
fn lock_contention_does_not_poison_completed_history() {
    let (engine, target) = fixture();
    let prepared = engine
        .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
        .unwrap();
    let other = target.with_file_name("other");
    assert!(engine
        .prepare(&plan(&other, None, Some(tree("new"))))
        .is_err());
    assert!(engine.list_history().unwrap().is_empty());
    engine.apply(prepared);
    assert_eq!(engine.list_history().unwrap().len(), 1);
}
struct Stop(Step);
impl Observer for Stop {
    fn event(&self, step: Step, _path: &Path) -> Result<(), IoError> {
        if step == self.0 {
            Err(IoError {
                code: "interrupted",
            })
        } else {
            Ok(())
        }
    }
}
#[test]
fn crash_after_archive_is_recovery_required() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(Stop(Step::Archived));
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::RecoveryRequired));
    assert!(!target.exists());
    let records = engine.list_recoveries().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].operation_id, receipt.operation_id);
    engine.observer = Arc::new(Quiet);
    let restore = engine.plan_restore(&receipt.operation_id).unwrap();
    engine.apply(engine.prepare(&restore).unwrap());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
}
struct EditAfterActivation;
impl Observer for EditAfterActivation {
    fn event(&self, step: Step, path: &Path) -> Result<(), IoError> {
        if step == Step::Activated {
            fs::write(path.join("external.txt"), b"external").unwrap();
        }
        Ok(())
    }
}
#[test]
fn external_edit_after_activation_prevents_auto_rollback() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(EditAfterActivation);
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::RecoveryRequired));
    assert_eq!(fs::read(target.join("external.txt")).unwrap(), b"external");
    assert_eq!(
        fs::read(receipt.backup_location.join("original/0/SKILL.md")).unwrap(),
        document("old")
    );
}
#[test]
fn audit_failure_keeps_real_receipt() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(Stop(Step::Audit));
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::Written));
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::AuditFailed));
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("new"));
}
#[test]
fn one_group_failure_does_not_hide_other_success() {
    let (mut engine, target) = fixture();
    let other = target.with_file_name("other");
    let mut changes = plan(&target, Some(tree("old")), Some(tree("new")));
    changes.directories.push(DirectoryChange {
        path: other.clone(),
        before: None,
        after: Some(tree("new")),
    });
    let prepared = engine.prepare(&changes).unwrap();
    fs::write(target.join("external"), b"later").unwrap();
    engine.observer = Arc::new(Quiet);
    let receipt = engine.apply(prepared);
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.path == target && i.outcome == Outcome::Blocked));
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.path == other && i.outcome == Outcome::Written));
}

#[test]
fn verified_operation_with_stale_lock_is_reported_for_recovery() {
    let (engine, target) = fixture();
    let root = engine.backup_root.parent().unwrap().parent().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "real_crash_child"])
        .env("SKILL_HUB_CRASH_ROOT", root)
        .env("SKILL_HUB_AUDIT_CRASH", "1")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(71));
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("new"));
    let records = engine.list_recoveries().unwrap();
    assert_eq!(records.len(), 1);
    let restore = engine.plan_restore(&records[0].operation_id).unwrap();
    engine.apply(engine.prepare(&restore).unwrap());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
}

struct ReplaceWithLink;
impl Observer for ReplaceWithLink {
    fn event(&self, step: Step, path: &Path) -> Result<(), IoError> {
        if step == Step::Activated {
            let original = path.with_file_name("preserved-activation");
            let foreign = path.with_file_name("foreign");
            fs::create_dir(&foreign).unwrap();
            fs::write(foreign.join("unrelated"), b"keep").unwrap();
            fs::rename(path, original).unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&foreign, path).unwrap();
            #[cfg(windows)]
            assert!(std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(path)
                .arg(foreign)
                .output()
                .unwrap()
                .status
                .success());
        }
        Ok(())
    }
}
#[test]
fn cannot_read_owned_target_keeps_current_directory() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(ReplaceWithLink);
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::RecoveryRequired));
    assert_eq!(fs::read(target.join("unrelated")).unwrap(), b"keep");
    assert!(target
        .with_file_name("preserved-activation")
        .join("SKILL.md")
        .exists());
    assert!(receipt.backup_location.join("original/0/SKILL.md").exists());
}
#[test]
fn interrupted_revision_keeps_old_catalog_reference() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(Stop(Step::Activated));
    let catalog = engine.backup_root.parent().unwrap().join("catalog.json");
    fs::write(&catalog, b"old revision").unwrap();
    engine.write_files.push(catalog.clone());
    let mut changes = plan(&target, None, Some(tree("new")));
    changes.directories[0].path = target.with_file_name("new-revision");
    changes.files.push(FileChange {
        path: catalog.clone(),
        before: Some(b"old revision".to_vec()),
        after: Some(b"new revision".to_vec()),
    });
    let receipt = engine.apply(engine.prepare(&changes).unwrap());
    assert_eq!(fs::read(catalog).unwrap(), b"old revision");
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::RecoveryRequired));
    assert!(target.with_file_name("new-revision").exists());
}
#[cfg(unix)]
#[test]
fn transaction_and_restore_preserve_unix_execution_flag() {
    use std::os::unix::fs::PermissionsExt;
    let (engine, target) = fixture();
    fs::set_permissions(target.join("SKILL.md"), fs::Permissions::from_mode(0o700)).unwrap();
    let before = skill_io::read_package_snapshot(&target, &ScanLimits::default())
        .unwrap()
        .tree;
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(before), Some(tree("new"))))
            .unwrap(),
    );
    assert_eq!(
        fs::metadata(target.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0
    );
    let restore = engine.plan_restore(&receipt.operation_id).unwrap();
    engine.apply(engine.prepare(&restore).unwrap());
    assert_ne!(
        fs::metadata(target.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0
    );
}

#[cfg(windows)]
struct DenyRename(std::sync::Mutex<Option<std::fs::File>>);
#[cfg(windows)]
impl Observer for DenyRename {
    fn event(&self, step: Step, path: &Path) -> Result<(), IoError> {
        use std::os::windows::fs::OpenOptionsExt;
        if step == Step::BackedUp {
            *self.0.lock().unwrap() = Some(
                fs::OpenOptions::new()
                    .read(true)
                    .share_mode(1)
                    .open(path.join("SKILL.md"))
                    .unwrap(),
            );
        }
        Ok(())
    }
}
#[cfg(windows)]
#[test]
fn windows_rename_failure_keeps_original_and_backup() {
    let (mut engine, target) = fixture();
    engine.observer = Arc::new(DenyRename(std::sync::Mutex::new(None)));
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
    assert!(receipt.outcomes.iter().any(|i| matches!(
        i.outcome,
        Outcome::RecoveryRequired | Outcome::FailedRestored | Outcome::Blocked
    )));
    assert!(receipt.backup_location.join("original/0/SKILL.md").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn cross_filesystem_backup_releases_lock_and_recovers_real_crash() {
    use std::os::unix::fs::MetadataExt;
    let (mut engine, target) = fixture();
    let root = engine
        .backup_root
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let backup = std::path::PathBuf::from("/dev/shm").join(skill_io::disk::operation_id());
    fs::create_dir(&backup).unwrap();
    assert_ne!(
        fs::metadata(&root).unwrap().dev(),
        fs::metadata(&backup).unwrap().dev()
    );
    engine.backup_root = backup;
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .all(|o| o.outcome == Outcome::Written));
    assert!(!root.join("hub/write.lock").exists());
    let restore = engine.plan_restore(&receipt.operation_id).unwrap();
    engine.apply(engine.prepare(&restore).unwrap());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "real_crash_child"])
        .env("SKILL_HUB_CRASH_ROOT", &root)
        .env("SKILL_HUB_BACKUP_ROOT", &engine.backup_root)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(71));
    let records = engine.list_recoveries().unwrap();
    assert_eq!(records.len(), 1);
    let restore = engine.plan_restore(&records[0].operation_id).unwrap();
    let receipt = engine.apply(engine.prepare(&restore).unwrap());
    assert!(receipt.backup_location.join("recovered.lock").exists());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
    assert!(!root.join("hub/write.lock").exists());
}
