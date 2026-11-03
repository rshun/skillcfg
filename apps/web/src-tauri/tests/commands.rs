use serde_json::{json, Value};
use skill_desktop::{
    bridge::DesktopSession,
    directory::{choose_directory, DirectoryPicker},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn fixture() -> (DesktopSession, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(skill_io::disk::operation_id());
    let home = root.join("home");
    let source = root.join("source");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: native-fixture\ndescription: Native fixture.\n---\nKeep this source.\n",
    )
    .unwrap();
    (DesktopSession::new(home.clone()).unwrap(), home, source)
}
#[test]
fn forged_target_path_is_rejected() {
    let (mut session, home, _) = fixture();
    let e = session
        .execute(
            "commit_distribution",
            json!({"previewId":"forged", "targetPath":home.join("unexpected")}),
        )
        .unwrap_err();
    assert_eq!(e.code, "invalid_request");
    assert!(!home.join("unexpected").exists());
}
#[test]
fn unknown_package_id_cannot_read_arbitrary_file() {
    let (mut session, home, _) = fixture();
    session.execute("load_library", json!({})).unwrap();
    for package in ["../../private", "unknown-package"] {
        let e = session
            .execute("read_package", json!({"packageId":package}))
            .unwrap_err();
        assert!(matches!(
            e.code.as_str(),
            "invalid_request" | "unknown_package"
        ));
    }
    assert!(!home.join(".skill-hub/library").exists());
}
#[test]
fn window_preview_is_not_shared() {
    let (mut first, home, source) = fixture();
    let mut second = DesktopSession::new(home).unwrap();
    let scan = first
        .execute("scan_import", json!({"source":source}))
        .unwrap();
    let item = scan["items"][0]["itemId"].as_str().unwrap();
    let p=first.execute("preview_import",json!({"scanId":scan["scanId"],"choices":[{"itemId":item,"action":"new","replaceId":null}]})).unwrap();
    let e = second
        .execute("commit_import", json!({"previewId":p["previewId"]}))
        .unwrap_err();
    assert_ne!(e.code, "not_implemented");
    first
        .execute("commit_import", json!({"previewId":p["previewId"]}))
        .unwrap();
    let library = second.execute("load_library", json!({})).unwrap();
    assert_eq!(library["records"].as_array().unwrap().len(), 1);
}
#[test]
fn cancel_native_picker_returns_none() {
    struct Cancel;
    impl DirectoryPicker for Cancel {
        fn pick(
            &self,
            _: Option<&Path>,
        ) -> Result<Option<PathBuf>, skill_desktop::bridge::SafeError> {
            Ok(None)
        }
    }
    assert_eq!(choose_directory(&Cancel, None).unwrap(), None);
}
#[test]
fn unapproved_profile_cannot_register_native_write_root() {
    let (mut session, home, _) = fixture();
    fs::create_dir_all(home.join(".skill-hub/agents")).unwrap();
    fs::write(
        home.join(".skill-hub/agents/custom.demo.json"),
        include_str!("../../../../tests/fixtures/agents/custom.demo.json"),
    )
    .unwrap();
    let profiles = session.execute("list_agent_profiles", json!({})).unwrap();
    let custom = profiles
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "custom.demo")
        .unwrap();
    assert_eq!(custom["state"], "pending");
    let error = session
        .execute("list_agent_skills", json!({"agentId":"custom.demo"}))
        .unwrap_err();
    assert_eq!(error.code, "profile_not_enabled");
    assert!(!home.join(".example").exists());
}
#[test]
fn custom_profile_uses_same_ipc_without_code_plugin() {
    let (mut session, home, _) = fixture();
    let raw: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/agents/custom.demo.json"
    ))
    .unwrap();
    let p=session.execute("preview_agent_profile",json!({"profileJson":raw.to_string(),"expectedRevision":null,"localRootOverrides":[],"enableAfterSave":true})).unwrap();
    assert!(!p["blocked"].as_bool().unwrap());
    assert!(p["resolvedRoots"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str().unwrap().contains(".example")));
    session
        .execute("commit_agent_profile", json!({"previewId":p["previewId"]}))
        .unwrap();
    let profiles = session.execute("list_agent_profiles", json!({})).unwrap();
    assert_eq!(profiles.as_array().unwrap().len(), 5);
    let inventory = session
        .execute("list_agent_skills", json!({"agentId":"custom.demo"}))
        .unwrap();
    assert_eq!(inventory["agentId"], "custom.demo");
    assert!(!home.join(".example").exists());
}
