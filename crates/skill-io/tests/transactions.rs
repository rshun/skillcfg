mod tx_support;
use skill_io::*;
use std::fs;
use tx_support::*;

#[test]
fn backup_failure_means_zero_target_mutations() {
    let (mut engine, target) = fixture();
    let invalid = engine.backup_root.parent().unwrap().join("file");
    fs::write(&invalid, b"occupied").unwrap();
    engine.backup_root = invalid.join("backups");
    assert!(engine
        .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
        .is_err());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
}
#[test]
fn extra_old_files_leave_load_path_but_are_archived() {
    let (engine, target) = fixture();
    fs::write(target.join("notes.bin"), [0, 255]).unwrap();
    let before = read_package_snapshot(&target, &ScanLimits::default())
        .unwrap()
        .tree;
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, Some(before), Some(tree("new"))))
            .unwrap(),
    );
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::Written));
    assert!(!target.join("notes.bin").exists());
    assert_eq!(
        fs::read(receipt.backup_location.join("original/0/notes.bin")).unwrap(),
        vec![0, 255]
    );
    assert!(engine.list_recoveries().unwrap().is_empty());
}
#[test]
fn no_staging_inside_any_skill_root() {
    let (mut engine, target) = fixture();
    engine
        .scan_roots
        .push(target.parent().unwrap().parent().unwrap().to_owned());
    assert!(engine
        .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
        .is_err());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), document("old"));
}
#[test]
fn restore_original_absence_without_delete() {
    let (engine, existing) = fixture();
    let target = existing.with_file_name("new");
    let receipt = engine.apply(
        engine
            .prepare(&plan(&target, None, Some(tree("new"))))
            .unwrap(),
    );
    assert!(target.exists());
    let restore = engine.plan_restore(&receipt.operation_id).unwrap();
    let receipt = engine.apply(engine.prepare(&restore).unwrap());
    assert!(!target.exists());
    assert_eq!(
        fs::read(receipt.backup_location.join("original/0/SKILL.md")).unwrap(),
        document("new")
    );
}
#[test]
fn target_change_after_prepare_is_blocked() {
    let (engine, target) = fixture();
    let prepared = engine
        .prepare(&plan(&target, Some(tree("old")), Some(tree("new"))))
        .unwrap();
    fs::write(target.join("later.txt"), b"later edit").unwrap();
    let receipt = engine.apply(prepared);
    assert!(receipt
        .outcomes
        .iter()
        .any(|i| i.outcome == Outcome::Blocked));
    assert!(target.join("later.txt").exists());
}
#[test]
fn file_commit_follows_directory_commit_and_checks_baseline() {
    let (mut engine, target) = fixture();
    let catalog = engine.backup_root.parent().unwrap().join("catalog.json");
    fs::write(&catalog, b"old").unwrap();
    engine.write_files.push(catalog.clone());
    let mut changes = plan(&target, Some(tree("old")), Some(tree("new")));
    changes.files.push(FileChange {
        path: catalog.clone(),
        before: Some(b"old".to_vec()),
        after: Some(b"new".to_vec()),
    });
    let receipt = engine.apply(engine.prepare(&changes).unwrap());
    assert!(receipt
        .outcomes
        .iter()
        .all(|i| i.outcome == Outcome::Written));
    assert_eq!(fs::read(catalog).unwrap(), b"new");
}
