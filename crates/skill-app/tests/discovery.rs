mod support;
use serde_json::json;
use skill_app::*;
use skill_core::{AgentId, Scope};
use std::fs;
use support::*;

#[test]
fn pending_custom_profile_does_not_scan_and_approved_profile_reads_without_writes() {
    let env = env();
    let source = source(&env, vec![]);
    fs::create_dir_all(&source.custom_dir).unwrap();
    let data = bytes(&profile("custom.fifth"));
    fs::write(source.custom_dir.join("fifth.json"), &data).unwrap();
    package(&env.home.join(".example/skills"), "review");
    let registry = load_registry(&source, &env).unwrap();
    let pending = app(env.clone(), registry, Scope::User);
    assert_eq!(
        pending
            .list_agent_skills(AgentId::parse("custom.fifth").unwrap())
            .err()
            .unwrap()
            .code,
        "profile_not_enabled"
    );
    approve(&source, &env, data, true);
    let registry = load_registry(&source, &env).unwrap();
    let ready = app(env.clone(), registry, Scope::User);
    let inventory = ready
        .list_agent_skills(AgentId::parse("custom.fifth").unwrap())
        .unwrap();
    assert_eq!(inventory.items.len(), 1);
    assert_eq!(inventory.items[0].metadata.as_ref().unwrap().name, "review");
    assert!(!env.home.join(".skill-hub/library").exists());
    assert!(!env.home.join(".skill-hub/backups").exists());
}
#[test]
fn project_parent_is_read_only_and_missing_project_is_blocked() {
    let mut env = env();
    let project = env.home.join("repo");
    fs::create_dir_all(project.join(".git")).unwrap();
    let child = project.join("child");
    fs::create_dir_all(&child).unwrap();
    env.project_root = Some(child.clone());
    let mut p = profile("codex");
    p["roots"] = json!([root(
        "project",
        "project",
        "project",
        ".agents/skills",
        true
    )]);
    package(&project.join(".agents/skills"), "review");
    let registry = load_registry(&source(&env, vec![bytes(&p)]), &env).unwrap();
    let mut app = app(env, registry, Scope::Project);
    let inventory = app
        .list_agent_skills(AgentId::parse("codex").unwrap())
        .unwrap();
    assert_eq!(inventory.items.len(), 1);
    assert!(inventory
        .roots
        .iter()
        .any(|r| r.writable && r.path == child.join(".agents/skills")));
    assert!(!inventory
        .roots
        .iter()
        .any(|r| r.writable && r.path == project.join(".agents/skills")));
    app.context.env.project_root = None;
    assert!(app
        .list_agent_skills(AgentId::parse("codex").unwrap())
        .is_err());
}
#[test]
fn invalid_root_is_an_error_not_an_empty_or_fallback_inventory() {
    let env = env();
    fs::create_dir_all(env.home.join(".example")).unwrap();
    fs::write(env.home.join(".example/skills"), b"not a directory").unwrap();
    let registry = load_registry(&source(&env, vec![bytes(&profile("codex"))]), &env).unwrap();
    let inventory = app(env, registry, Scope::User)
        .list_agent_skills(AgentId::parse("codex").unwrap())
        .unwrap();
    assert!(!inventory.errors.is_empty());
}
