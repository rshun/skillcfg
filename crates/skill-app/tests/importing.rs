mod service_support;
use service_support::*;
use skill_app::*;
use std::fs;
#[test]
fn changed_source_consumes_import_preview_without_creating_library() {
    let (app, source, library, _) = ready();
    let scan = app
        .scan_import(ImportRequest {
            source: source.clone(),
            library_path: library.clone(),
        })
        .unwrap();
    let preview = app
        .preview_import(
            &scan.scan_id,
            &[ImportChoice {
                item_id: scan.items[0].item_id.clone(),
                action: ImportAction::New,
            }],
        )
        .unwrap();
    fs::write(source.join("added.txt"), b"changed").unwrap();
    assert!(app.commit_import(&preview.preview_id).is_err());
    assert!(app.commit_import(&preview.preview_id).is_err());
    assert!(!library.exists());
}
#[test]
fn same_name_different_content_requires_explicit_choice() {
    let (app, source, library, _) = ready();
    import(&app, &source, &library);
    fs::write(source.join("extra"), b"different").unwrap();
    let scan = app
        .scan_import(ImportRequest {
            source,
            library_path: library.clone(),
        })
        .unwrap();
    let preview = app
        .preview_import(
            &scan.scan_id,
            &[ImportChoice {
                item_id: scan.items[0].item_id.clone(),
                action: ImportAction::Skip,
            }],
        )
        .unwrap();
    app.commit_import(&preview.preview_id).unwrap();
    assert_eq!(app.load_library(&library).unwrap().records.len(), 1);
}
#[test]
fn sensitive_source_and_library_overlap_are_rejected() {
    let (app, source, library, _) = ready();
    fs::write(
        source.join("settings.json"),
        br#"{"token":"SYNTHETIC_PRIVATE_MARKER"}"#,
    )
    .unwrap();
    let result = app.scan_import(ImportRequest {
        source: source.clone(),
        library_path: library.clone(),
    });
    assert!(result.is_err() || result.unwrap().items.iter().any(|i| !i.issues.is_empty()));
    assert!(app
        .scan_import(ImportRequest {
            source: source.clone(),
            library_path: source.join("library")
        })
        .is_err());
}

#[test]
fn replacement_retains_previous_revision_in_central_backup(){
 let (app,source,library,_)=ready();fs::write(source.join("asset.bin"),[0,255]).unwrap();let id=import(&app,&source,&library);let view=app.load_library(&library).unwrap();let old=view.records[0].current_revision.clone();
 fs::write(source.join("extra"),b"new").unwrap();let scan=app.scan_import(ImportRequest{source,library_path:library.clone()}).unwrap();let preview=app.preview_import(&scan.scan_id,&[ImportChoice{item_id:scan.items[0].item_id.clone(),action:ImportAction::Replace(skill_core::plan::PackageId(id))}]).unwrap();let receipt=app.commit_import(&preview.preview_id).unwrap();
 let backup=std::path::PathBuf::from(receipt.backup_location);assert!(backup.join("retained/0/asset.bin").exists());assert!(library.join("packages").join(&view.records[0].id).join("revisions").join(old).exists());
}
