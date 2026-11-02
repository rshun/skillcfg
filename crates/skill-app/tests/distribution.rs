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
