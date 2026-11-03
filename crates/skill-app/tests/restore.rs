mod service_support;
use service_support::*;
use skill_app::*;
use skill_core::plan::PackageId;
use std::fs;
#[test]
fn restore_original_absence_backs_up_later_edits_without_delete() {
    let (app, source, library, target) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    let receipt = app.commit_distribution(&p.preview_id).unwrap();
    fs::write(target.join("later"), b"later").unwrap();
    let p = app.preview_restore(&receipt.operation_id).unwrap();
    assert!(target.exists());
    let restored = app.commit_restore(&p.preview_id).unwrap();
    assert!(!target.exists());
    assert_eq!(
        fs::read(std::path::Path::new(&restored.backup_location).join("original/0/later")).unwrap(),
        b"later"
    );
}
#[test]
fn restore_uses_name_at_write_path_and_does_not_hide_old_location() {
    let (app, source, library, old) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    let first = app.commit_distribution(&p.preview_id).unwrap();
    let view = app.read_package(&PackageId(id.clone())).unwrap();
    app.save_skill_document(EditDocumentRequest {
        package_id: PackageId(id.clone()),
        edit_token: view.edit_token.unwrap(),
        document: view
            .document
            .unwrap()
            .replace("name: review", "name: renamed"),
    })
    .unwrap();
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    app.commit_distribution(&p.preview_id).unwrap();
    assert!(old.exists());
    let new = old.with_file_name("renamed");
    assert!(new.exists());
    let p = app.preview_restore(&first.operation_id).unwrap();
    app.commit_restore(&p.preview_id).unwrap();
    assert!(!old.exists());
    assert!(new.exists());
}

#[test]
fn changed_backup_root_keeps_old_history_and_corrupt_settings_require_explicit_restore() {
    let (app, source, library, target) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    let original = app.commit_distribution(&p.preview_id).unwrap();
    let mut settings = app.load_settings().unwrap();
    settings.settings.backup_path = app.context.env.home.join("new-backups");
    app.save_settings(SettingsRequest {
        expected_revision: settings.revision,
        settings: settings.settings,
    })
    .unwrap();
    assert!(app
        .list_history()
        .unwrap()
        .iter()
        .any(|r| r.operation_id == original.operation_id));
    fs::write(
        app.context.env.home.join(".skill-hub/settings.json"),
        b"broken",
    )
    .unwrap();
    assert!(app.load_settings().is_err());
    assert!(target.exists());
    let preview = app.preview_restore(&original.operation_id).unwrap();
    assert!(target.exists());
    let receipt = app.commit_restore(&preview.preview_id).unwrap();
    assert!(!receipt.operation_id.is_empty());
    assert!(!target.exists());
    assert_eq!(
        fs::read(app.context.env.home.join(".skill-hub/settings.json")).unwrap(),
        b"broken"
    );
}

#[test]
fn same_bytes_new_target_identity_invalidates_restore() {
    let (app, source, library, target) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    let receipt = app.commit_distribution(&p.preview_id).unwrap();
    let preview = app.preview_restore(&receipt.operation_id).unwrap();
    let bytes = fs::read(target.join("SKILL.md")).unwrap();
    fs::rename(&target, target.with_file_name("preserved")).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), &bytes).unwrap();
    assert!(app.commit_restore(&preview.preview_id).is_err());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), bytes);
}

#[test]
fn restoring_first_import_keeps_library_usable() {
    let (app, source, library, _) = ready();
    let scan = app
        .scan_import(ImportRequest {
            source: source.clone(),
            library_path: library.clone(),
        })
        .unwrap();
    let p = app
        .preview_import(
            &scan.scan_id,
            &[ImportChoice {
                item_id: scan.items[0].item_id.clone(),
                action: ImportAction::New,
            }],
        )
        .unwrap();
    let imported = app.commit_import(&p.preview_id).unwrap();
    let p = app.preview_restore(&imported.operation_id).unwrap();
    app.commit_restore(&p.preview_id).unwrap();
    assert!(app.load_library(&library).unwrap().records.is_empty());
    import(&app, &source, &library);
    assert_eq!(app.load_library(&library).unwrap().records.len(), 1);
}

#[test]
fn abandoning_first_import_preparation_keeps_library_usable() {
    let (app, source, library, target) = ready();
    let tree = skill_io::read_package_snapshot(&source, &skill_io::ScanLimits::default())
        .unwrap()
        .tree;
    let backup = app.context.env.home.join(".skill-hub/backups");
    let engine = skill_io::IoEngine {
        backup_root: backup.clone(),
        audit_path: app.context.env.home.join(".skill-hub/audit/events.jsonl"),
        write_roots: vec![library.join("packages")],
        write_files: vec![library.join("catalog.json")],
        write_file_roots: vec![],
        scan_roots: vec![target.parent().unwrap().to_owned()],
        protected_roots: vec![],
        observer: std::sync::Arc::new(skill_io::Quiet),
    };
    let plan = skill_io::MutationPlan {
        directories: vec![skill_io::DirectoryChange {
            path: library.join("packages/fixture/revisions/fixture"),
            before: None,
            after: Some(tree),
        }],
        files: vec![skill_io::FileChange {
            path: library.join("catalog.json"),
            before: None,
            after: Some(br#"{"version":1,"records":[]}"#.to_vec()),
        }],
        kind: "import".into(),
        ..Default::default()
    };
    let prepared = engine.prepare(&plan).unwrap();
    let operation = engine.list_recoveries().unwrap()[0].operation_id.clone();
    drop(prepared);
    // Reproduce a process exit after staging finished but before the prepared
    // manifest was published. No target commit has occurred.
    let path = backup.join(&operation).join("manifest.json");
    let mut journal: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    journal["state"] = serde_json::json!("preparing");
    fs::write(path, serde_json::to_vec(&journal).unwrap()).unwrap();
    let staging = library.join(".skill-hub-transactions");
    assert!(staging.exists());
    assert!(!library.join("catalog.json").exists());
    let p = app.preview_restore(&operation).unwrap();
    app.commit_restore(&p.preview_id).unwrap();
    assert!(app.load_library(&library).unwrap().records.is_empty());
    assert!(staging.exists());
    assert!(!target.exists());
    import(&app, &source, &library);
    assert_eq!(app.load_library(&library).unwrap().records.len(), 1);
}
