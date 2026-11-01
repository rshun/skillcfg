mod service_support;
use service_support::*;
use skill_app::*;
use skill_core::plan::PackageId;
use std::fs;
#[test]
fn missing_library_is_first_use_not_created() {
    let (app, _, library, _) = ready();
    assert!(app.load_library(&library).unwrap().first_use);
    assert!(!library.exists());
}
#[test]
fn broken_library_is_not_empty() {
    let (app, _, library, _) = ready();
    fs::create_dir_all(&library).unwrap();
    fs::write(library.join("catalog.json"), b"not json").unwrap();
    assert!(app.load_library(&library).is_err());
}
#[test]
fn import_preserves_source_and_full_tree_and_edit_never_distributes() {
    let (app, source, library, target) = ready();
    fs::write(source.join("asset.bin"), [0, 255, 1]).unwrap();
    fs::create_dir(source.join("empty")).unwrap();
    let id = import(&app, &source, &library);
    assert!(!target.exists());
    let view = app.read_package(&PackageId(id.clone())).unwrap();
    assert!(view.files.contains(&"asset.bin".into()));
    assert!(view.files.contains(&"empty".into()));
    let old_revision = view.revision_id.clone();
    let text = view
        .document
        .unwrap()
        .replace("name: review", "name: renamed");
    app.save_skill_document(EditDocumentRequest {
        package_id: PackageId(id.clone()),
        edit_token: view.edit_token.unwrap(),
        document: text,
    })
    .unwrap();
    let current = app.load_library(&library).unwrap();
    assert_eq!(current.records[0].id, id);
    assert_eq!(current.records[0].name, "renamed");
    assert_ne!(current.records[0].current_revision, old_revision);
    assert!(library
        .join("packages")
        .join(&id)
        .join("revisions")
        .join(old_revision)
        .join("asset.bin")
        .exists());
    assert_eq!(fs::read(source.join("asset.bin")).unwrap(), vec![0, 255, 1]);
    assert!(!target.exists());
}
#[test]
fn catalog_metadata_mismatch_blocks_distribution() {
    let (app, source, library, _) = ready();
    let id = import(&app, &source, &library);
    let path = library.join("catalog.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    doc["records"][0]["name"] = serde_json::json!("wrong");
    fs::write(path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(app.preview_distribution(request(&id, &library)).is_err());
}
#[test]
fn external_edit_invalidates_edit_baseline() {
    let (app, source, library, _) = ready();
    let id = import(&app, &source, &library);
    let view = app.read_package(&PackageId(id.clone())).unwrap();
    let path = library
        .join("packages")
        .join(&id)
        .join("revisions")
        .join(&view.revision_id)
        .join("SKILL.md");
    fs::write(&path, b"external").unwrap();
    assert!(app
        .save_skill_document(EditDocumentRequest {
            package_id: PackageId(id),
            edit_token: view.edit_token.unwrap(),
            document: view.document.unwrap()
        })
        .is_err());
    assert_eq!(fs::read(path).unwrap(), b"external");
}
#[test]
fn broken_settings_cannot_be_saved_as_defaults() {
    let (app, _, _, _) = ready();
    let first = app.load_settings().unwrap();
    assert!(!app
        .context
        .env
        .home
        .join(".skill-hub/settings.json")
        .exists());
    let path = app.context.env.home.join(".skill-hub/settings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"broken").unwrap();
    assert!(app.load_settings().is_err());
    assert!(app
        .save_settings(SettingsRequest {
            expected_revision: first.revision,
            settings: first.settings
        })
        .is_err());
    assert_eq!(fs::read(path).unwrap(), b"broken");
}
