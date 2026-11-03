mod service_support;
use service_support::*;
use skill_app::*;
use skill_core::AgentId;
use std::fs;
#[test]
fn target_statuses_compare_full_bytes_without_consuming_ready_preview() {
    let (app, source, library, target) = ready(); let id=import(&app,&source,&library);
    assert_eq!(app.library_statuses(&library).unwrap()[0].state,"missing");
    let p=app.preview_distribution(request(&id,&library)).unwrap();
    app.library_statuses(&library).unwrap(); app.commit_distribution(&p.preview_id).unwrap();
    assert_eq!(app.library_statuses(&library).unwrap()[0].state,"equal");
    fs::write(target.join("extra.txt"),"A target-side edit.").unwrap();
    assert_eq!(app.library_statuses(&library).unwrap()[0].state,"different");
}
#[test]
fn profile_editor_token_binds_the_document_shown_to_user() {
    let (app,_,_,_)=ready(); let id=AgentId::parse("codex").unwrap();
    let (document,revision)=app.read_agent_profile_document(&id).unwrap();
    let mut p:serde_json::Value=serde_json::from_str(&document).unwrap(); p["displayName"]=serde_json::json!("Edited label");
    let preview=app.preview_agent_profile(ProfileChangeRequest{profile_json:serde_json::to_vec(&p).unwrap(),expected_revision:Some(revision),local_root_overrides:vec![],enable_after_save:true}).unwrap();
    assert!(!preview.blocked);
}
#[test]
fn late_cancel_of_old_preview_cannot_consume_new_preview() {
    let (app,source,library,target)=ready(); let id=import(&app,&source,&library);
    let old=app.preview_distribution(request(&id,&library)).unwrap(); let current=app.preview_distribution(request(&id,&library)).unwrap();
    app.cancel_preview(&old.preview_id); app.commit_distribution(&current.preview_id).unwrap(); assert!(target.join("SKILL.md").is_file());
}
