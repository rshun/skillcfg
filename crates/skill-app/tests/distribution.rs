mod service_support;
use service_support::*;
use skill_app::*;
use std::{
    fs,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
struct FakeClock(AtomicU64);
impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[test]
fn expired_preview_writes_nothing_and_is_consumed() {
    let (base, source, library, target) = ready();
    let clock = Arc::new(FakeClock(AtomicU64::new(100)));
    let app = App::with_clock(base.context, clock.clone());
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    clock.0.store(700, Ordering::SeqCst);
    assert!(app.commit_distribution(&p.preview_id).is_err());
    clock.0.store(101, Ordering::SeqCst);
    assert!(app.commit_distribution(&p.preview_id).is_err());
    assert!(!target.exists());
}
#[test]
fn added_target_file_invalidates_whole_preview_and_consumes_it() {
    let (app, source, library, target) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("external"), b"later").unwrap();
    assert!(app.commit_distribution(&p.preview_id).is_err());
    assert!(app.commit_distribution(&p.preview_id).is_err());
    assert_eq!(fs::read(target.join("external")).unwrap(), b"later");
    assert!(!target.join("SKILL.md").exists());
}
#[test]
fn settings_change_and_new_custom_profile_invalidate_distribution_preview() {
    for setting in [true, false] {
        let (app, source, library, target) = ready();
        let id = import(&app, &source, &library);
        let p = app.preview_distribution(request(&id, &library)).unwrap();
        if setting {
            let mut v = app.load_settings().unwrap();
            v.settings.backup_path = app.context.env.home.join("other-backups");
            app.save_settings(SettingsRequest {
                expected_revision: v.revision,
                settings: v.settings,
            })
            .unwrap();
        } else {
            let dir = app.context.env.home.join(".skill-hub/agents");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("new.json"), bytes(&profile("custom.new"))).unwrap();
        }
        assert!(app.commit_distribution(&p.preview_id).is_err());
        assert!(!target.exists());
    }
}
#[test]
fn identical_target_and_cancelled_preview_have_no_write() {
    let (app, source, library, target) = ready();
    let id = import(&app, &source, &library);
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    app.cancel_preview(&p.preview_id);
    assert!(app.commit_distribution(&p.preview_id).is_err());
    assert!(!target.exists());
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    app.commit_distribution(&p.preview_id).unwrap();
    let count = app.list_history().unwrap().len();
    let p = app.preview_distribution(request(&id, &library)).unwrap();
    let receipt = app.commit_distribution(&p.preview_id).unwrap();
    assert!(receipt
        .outcomes
        .iter()
        .all(|i| i.outcome == skill_io::Outcome::Unchanged));
    assert_eq!(app.list_history().unwrap().len(), count);
}

#[test]
fn binary_source_change_and_probe_change_invalidate_whole_preview(){
 for binary in [true,false]{
 let (base,source,library,target)=ready();let mut p=profile("codex");p["probes"]=serde_json::json!([{"probeId":"enabled","base":"home","relative":"probe.json","format":"json","keyPath":["enabled"],"expected":true,"onMissing":"unknown"}]);let env=base.context.env;fs::write(env.home.join("probe.json"),br#"{"enabled":true}"#).unwrap();let registry=load_registry(&service_support::source(&env,vec![bytes(&p)]),&env).unwrap();let app=service_support::app(env,registry,skill_core::Scope::User);
 fs::write(source.join("asset.bin"),[0,255]).unwrap();let id=import(&app,&source,&library);let preview=app.preview_distribution(request(&id,&library)).unwrap();
 if binary{let records=app.load_library(&library).unwrap();let revision=&records.records[0].current_revision;fs::write(library.join("packages").join(&id).join("revisions").join(revision).join("asset.bin"),[0,254]).unwrap();}else{fs::write(app.context.env.home.join("probe.json"),br#"{"enabled":false}"#).unwrap();}
 assert!(app.commit_distribution(&preview.preview_id).is_err());assert!(!target.exists());
 }
}
#[test]
fn redirected_target_parent_invalidates_preview(){
 let (app,source,library,target)=ready();let id=import(&app,&source,&library);let preview=app.preview_distribution(request(&id,&library)).unwrap();let parent=target.parent().unwrap();fs::create_dir_all(parent.parent().unwrap()).unwrap();let elsewhere=app.context.env.home.join("elsewhere");fs::create_dir_all(&elsewhere).unwrap();
 #[cfg(unix)] std::os::unix::fs::symlink(&elsewhere,parent).unwrap();
 #[cfg(windows)] assert!(std::process::Command::new("cmd").args(["/C","mklink","/J"]).arg(parent).arg(&elsewhere).output().unwrap().status.success());
 assert!(app.commit_distribution(&preview.preview_id).is_err());assert!(!elsewhere.join("review").exists());
}

#[test]
fn switching_variant_and_editing_profile_require_repreview(){for variant in [true,false]{let(base,source,library,target)=ready();let env=base.context.env;let mut p=profile("codex");p["variants"]=serde_json::json!(["app","cli"]);p["defaultVariant"]=serde_json::json!("app");let registry=load_registry(&service_support::source(&env,vec![bytes(&p)]),&env).unwrap();let app=service_support::app(env,registry,skill_core::Scope::User);let id=import(&app,&source,&library);let preview=app.preview_distribution(request(&id,&library)).unwrap();if variant{let mut s=app.load_settings().unwrap();s.settings.agent_variants.insert(skill_core::AgentId::parse("codex").unwrap(),"cli".into());app.save_settings(SettingsRequest{expected_revision:s.revision,settings:s.settings}).unwrap();}else{let dir=app.context.env.home.join(".skill-hub/agents");fs::create_dir_all(&dir).unwrap();fs::write(dir.join("new.json"),bytes(&profile("custom.changed"))).unwrap();}assert!(app.commit_distribution(&preview.preview_id).is_err());assert!(!target.exists());}}

#[test]
fn changed_skipped_or_unchanged_target_invalidates_other_writes(){
 for skipped in [true,false]{let(app,source,library,target)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();app.commit_distribution(&p.preview_id).unwrap();if skipped{fs::write(target.join("unmanaged"),b"keep").unwrap();}
 package(source.parent().unwrap(),"other");let second=import(&app,&source.parent().unwrap().join("other"),&library);let mut req=request(&id,&library);req.selection.push(skill_core::plan::SelectionPair{package_id:skill_core::plan::PackageId(second),agent:skill_core::AgentId::parse("codex").unwrap()});let preview=app.preview_distribution(req).unwrap();let history=app.list_history().unwrap().len();fs::write(target.join("later.bin"),[0,255]).unwrap();assert!(app.commit_distribution(&preview.preview_id).is_err());assert!(app.commit_distribution(&preview.preview_id).is_err());assert!(!target.with_file_name("other").exists());assert_eq!(app.list_history().unwrap().len(),history);
 }
}

#[test]
fn builtin_alias_cannot_overwrite_historical_external_library(){
 let(app,source,_,target)=ready();let old=app.context.env.home.join("external-old");let mut settings=app.load_settings().unwrap();settings.settings.library_path=old.clone();let settings=app.save_settings(SettingsRequest{expected_revision:settings.revision,settings:settings.settings}).unwrap();let id=import(&app,&source,&old);let records=app.load_library(&old).unwrap();let revision=records.records[0].current_revision.clone();let revisions=old.join("packages").join(id).join("revisions");let original=fs::read(revisions.join(&revision).join("SKILL.md")).unwrap();let library=app.context.env.home.join("external-new");let mut data=settings.settings;data.library_path=library.clone();app.save_settings(SettingsRequest{expected_revision:settings.revision,settings:data}).unwrap();fs::write(source.join("SKILL.md"),format!("---\nname: {revision}\ndescription: Replacement fixture.\n---\n").replace("\\n","\n")).unwrap();let id=import(&app,&source,&library);let root=target.parent().unwrap();fs::create_dir_all(root.parent().unwrap()).unwrap();
 #[cfg(unix)] std::os::unix::fs::symlink(&revisions,root).unwrap();
 #[cfg(windows)] assert!(std::process::Command::new("cmd").args(["/C","mklink","/J"]).arg(root).arg(&revisions).output().unwrap().status.success());
 let mut req=request(&id,&library);let path=revisions.join(&revision);req.conflict_decisions.insert(skill_core::plan::TargetKey(skill_io::paths::physical_key(&path).unwrap()),skill_core::plan::ConflictDecision::Replace);if let Ok(preview)=app.preview_distribution(req){let result=app.commit_distribution(&preview.preview_id);assert!(result.is_err() || result.unwrap().outcomes.iter().any(|o|o.outcome==skill_io::Outcome::Blocked));}assert_eq!(fs::read(path.join("SKILL.md")).unwrap(),original);
}
#[test]
fn unapproved_project_probe_does_not_affect_shared_reader_evidence(){
 let(mut app,source,library,_)=ready();let mut p=profile("custom.shared");p["probes"]=serde_json::json!([{"probeId":"enabled","base":"project","relative":"private.json","format":"json","keyPath":["enabled"],"expected":true,"onMissing":"unknown"}]);p["roots"][0]["readEvidence"]=serde_json::json!({"kind":"bool-config-v1","probeId":"enabled"});let preview=app.preview_agent_profile(ProfileChangeRequest{profile_json:bytes(&p),expected_revision:None,local_root_overrides:vec![],enable_after_save:true}).unwrap();app.commit_agent_profile(&preview.preview_id).unwrap();let project=app.context.env.home.join("new-project");fs::create_dir(&project).unwrap();fs::write(project.join("private.json"),br#"{"enabled":true}"#).unwrap();app.context.env.project_root=Some(project.clone());let id=import(&app,&source,&library);let mut req=request(&id,&library);req.project_root=Some(project);let preview=app.preview_distribution(req).unwrap();let readers=&preview.groups[0].possible_readers;assert!(readers.iter().any(|r|r.agent.as_str()=="custom.shared"));assert!(readers.iter().filter(|r|r.agent.as_str()=="custom.shared").all(|r|r.evidence==skill_core::ReadState::Unknown));
}
