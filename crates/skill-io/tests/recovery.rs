mod tx_support;
use skill_io::*;
use std::{fs, path::Path, sync::Arc};
use tx_support::*;
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
