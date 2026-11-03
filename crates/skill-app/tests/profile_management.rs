mod service_support;
use service_support::*;
use skill_app::*;
use skill_core::AgentId;
use std::fs;
fn change() -> ProfileChangeRequest {
    ProfileChangeRequest {
        profile_json: bytes(&profile("custom.fifth")),
        expected_revision: None,
        local_root_overrides: vec![],
        enable_after_save: true,
    }
}
#[test]
fn alias_cannot_authorize_a_private_hub_directory_as_agent_root() {
    let (app, _, _, _) = ready();
    let private = app.context.env.home.join(".skill-hub/internal");
    fs::create_dir_all(&private).unwrap();
    let alias = app.context.env.home.join(".alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&private, &alias).unwrap();
    #[cfg(windows)]
    assert!(std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(&private)
        .output()
        .unwrap()
        .status
        .success());
    let mut p = profile("custom.fifth");
    p["roots"][0]["relative"] = serde_json::json!(".alias");
    let mut req = change();
    req.profile_json = bytes(&p);
    assert!(app.preview_agent_profile(req).is_err());
}
#[test]
fn profile_save_alone_never_distributes_skills() {
    let (app, _, library, target) = ready();
    let preview = app.preview_agent_profile(change()).unwrap();
    assert!(!preview.resolved_roots.is_empty());
    assert!(!app.context.env.home.join(".skill-hub").exists());
    app.commit_agent_profile(&preview.preview_id).unwrap();
    assert!(!library.exists());
    assert!(!target.exists());
    assert!(app.list_agent_profiles().unwrap().iter().any(|v| v
        .id
        .as_ref()
        .is_some_and(|i| i.as_str() == "custom.fifth")
        && v.state == ProfileState::Enabled));
}
#[test]
fn external_profile_edit_revokes_previous_approval_and_enable_baseline() {
    let (app, _, _, _) = ready();
    let preview = app.preview_agent_profile(change()).unwrap();
    app.commit_agent_profile(&preview.preview_id).unwrap();
    let views = app.list_agent_profiles().unwrap();
    let v = views
        .iter()
        .find(|v| v.id.as_ref().is_some_and(|i| i.as_str() == "custom.fifth"))
        .unwrap();
    let revision = v.revision.clone().unwrap();
    let path = app
        .context
        .env
        .home
        .join(".skill-hub/agents/custom.fifth.json");
    let mut changed = profile("custom.fifth");
    changed["roots"][0]["relative"] = serde_json::json!(".other/skills");
    fs::write(path, bytes(&changed)).unwrap();
    assert!(app
        .set_agent_profile_enabled(ProfileEnableRequest {
            agent_id: AgentId::parse("custom.fifth").unwrap(),
            enabled: true,
            expected_revision: revision
        })
        .is_err());
    assert!(app.list_agent_profiles().unwrap().iter().any(|v| v
        .id
        .as_ref()
        .is_some_and(|i| i.as_str() == "custom.fifth")
        && v.state == ProfileState::Pending));
}
#[test]
fn approval_preview_lists_all_local_variants_without_reading_probe_contents() {
    let (app, _, _, _) = ready();
    let mut p = profile("custom.fifth");
    p["variants"] = serde_json::json!(["app", "cli"]);
    p["defaultVariant"] = serde_json::json!("app");
    p["probes"] = serde_json::json!([{"probeId":"enabled","base":"home","relative":"probe.json","format":"json","keyPath":["enabled"],"expected":true,"onMissing":"unknown"}]);
    fs::write(
        app.context.env.home.join("probe.json"),
        br#"{"token":"SYNTHETIC_PRIVATE_MARKER","enabled":true}"#,
    )
    .unwrap();
    let mut req = change();
    req.profile_json = bytes(&p);
    let preview = app.preview_agent_profile(req).unwrap();
    assert_eq!(preview.variants, vec!["app", "cli"]);
    assert!(!preview.probe_paths.is_empty());
    assert!(!serde_json::to_string(&preview)
        .unwrap()
        .contains("SYNTHETIC_PRIVATE_MARKER"));
    assert!(!app.context.env.home.join(".skill-hub").exists());
}

#[test]
fn historical_library_cannot_become_an_agent_root() {
    let (app, _, _, _) = ready();
    let mut settings = app.load_settings().unwrap();
    settings.settings.library_path = app.context.env.home.join("old-library");
    let mut settings = app
        .save_settings(SettingsRequest {
            expected_revision: settings.revision,
            settings: settings.settings,
        })
        .unwrap();
    settings.settings.library_path = app.context.env.home.join("new-library");
    app.save_settings(SettingsRequest {
        expected_revision: settings.revision,
        settings: settings.settings,
    })
    .unwrap();
    let mut p = profile("custom.fifth");
    p["roots"][0]["relative"] = serde_json::json!("old-library");
    let mut req = change();
    req.profile_json = bytes(&p);
    assert!(app.preview_agent_profile(req).is_err());
}
