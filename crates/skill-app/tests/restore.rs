mod service_support;
use service_support::*;
use skill_app::*;
use skill_core::plan::PackageId;
use std::fs;
#[test]
fn restore_original_absence_backs_up_later_edits_without_delete(){let (app,source,library,target)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();let receipt=app.commit_distribution(&p.preview_id).unwrap();fs::write(target.join("later"),b"later").unwrap();let p=app.preview_restore(&receipt.operation_id).unwrap();assert!(target.exists());let restored=app.commit_restore(&p.preview_id).unwrap();assert!(!target.exists());assert_eq!(fs::read(std::path::Path::new(&restored.backup_location).join("original/0/later")).unwrap(),b"later");}
#[test]
fn restore_uses_name_at_write_path_and_does_not_hide_old_location(){let (app,source,library,old)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();let first=app.commit_distribution(&p.preview_id).unwrap();let view=app.read_package(&PackageId(id.clone())).unwrap();app.save_skill_document(EditDocumentRequest{package_id:PackageId(id.clone()),edit_token:view.edit_token.unwrap(),document:view.document.unwrap().replace("name: review","name: renamed")}).unwrap();let p=app.preview_distribution(request(&id,&library)).unwrap();app.commit_distribution(&p.preview_id).unwrap();assert!(old.exists());let new=old.with_file_name("renamed");assert!(new.exists());let p=app.preview_restore(&first.operation_id).unwrap();app.commit_restore(&p.preview_id).unwrap();assert!(!old.exists());assert!(new.exists());}
