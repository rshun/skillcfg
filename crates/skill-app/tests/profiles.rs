mod support;
use serde_json::json;
use skill_app::registry::builtin_profiles;
use skill_app::roots::{reader_exposures, resolve_profile_roots};
use skill_app::*;
use skill_core::{
    profile::{parse_agent_profile, MAX_PROFILE_BYTES},
    AgentId, Scope,
};
use std::fs;
use support::*;

#[test]
fn embedded_profiles_use_the_same_validator_and_resolve_on_each_platform() {
    let mut env = env();
    let project = env.home.join("project");
    fs::create_dir_all(&project).unwrap();
    env.project_root = Some(fs::canonicalize(project).unwrap());
    let documents = builtin_profiles();
    let ids: std::collections::BTreeSet<_> = documents
        .iter()
        .map(|doc| parse_agent_profile(doc).unwrap().id.as_str().to_owned())
        .collect();
    for required in ["codex", "claude-code", "antigravity", "grok"] {
        assert!(ids.contains(required));
    }
    for platform in [
        skill_core::Platform::Linux,
        skill_core::Platform::Macos,
        skill_core::Platform::Windows,
    ] {
        env.platform = platform;
        let registry = load_registry(&source(&env, documents.clone()), &env).unwrap();
        assert!(registry
            .entries
            .iter()
            .all(|e| e.state == ProfileState::Builtin));
        for scope in [Scope::User, Scope::Project] {
            let context = app(env.clone(), registry.clone(), scope).context;
            for entry in &context.registry.entries {
                let profile = entry.profile.as_ref().unwrap();
                for variant in &profile.variants {
                    let mut selected = context.clone();
                    selected
                        .registry
                        .selected_variants
                        .insert(profile.id.clone(), variant.clone());
                    let roots = resolve_profile_roots(profile, &selected).unwrap();
                    assert_eq!(roots.roots.iter().filter(|r| r.writable).count(), 1);
                }
            }
        }
    }
}

#[test]
fn fifth_standard_agent_is_loaded_from_json_only() {
    let env = env();
    let source = source(&env, vec![bytes(&profile("codex"))]);
    fs::create_dir_all(&source.custom_dir).unwrap();
    let data = bytes(&profile("custom.fifth-agent"));
    fs::write(source.custom_dir.join("fifth.json"), &data).unwrap();
    approve(&source, &env, data, true);
    let registry = load_registry(&source, &env).unwrap();
    assert_eq!(registry.entries.len(), 2);
    assert!(registry
        .entries
        .iter()
        .any(|e| e.state == ProfileState::Enabled
            && e.profile.as_ref().unwrap().id.as_str() == "custom.fifth-agent"));
}

#[test]
fn profile_listing_includes_pending_agents_without_scanning_roots() {
    let env = env();
    let source = source(&env, builtin_profiles());
    fs::create_dir_all(&source.custom_dir).unwrap();
    fs::write(
        source.custom_dir.join("fifth.json"),
        bytes(&profile("custom.fifth")),
    )
    .unwrap();
    let registry = load_registry(&source, &env).unwrap();
    let views = app(env.clone(), registry, Scope::User)
        .list_agent_profiles()
        .unwrap();
    let pending = views
        .iter()
        .find(|v| {
            v.id.as_ref()
                .is_some_and(|id| id.as_str() == "custom.fifth")
        })
        .unwrap();
    assert_eq!(pending.state, ProfileState::Pending);
    assert!(pending.roots.is_empty());
    assert!(!env.home.join(".example").exists());
    assert!(views
        .iter()
        .any(|v| v.state == ProfileState::Builtin && !v.roots.is_empty()));
}
#[test]
fn external_profile_edit_revokes_approval() {
    let env = env();
    let source = source(&env, vec![]);
    fs::create_dir_all(&source.custom_dir).unwrap();
    let original = bytes(&profile("custom.fifth"));
    approve(&source, &env, original, true);
    let mut changed = profile("custom.fifth");
    changed["roots"][0]["relative"] = json!(".other/skills");
    fs::write(source.custom_dir.join("fifth.json"), bytes(&changed)).unwrap();
    assert_eq!(
        load_registry(&source, &env).unwrap().entries[0].state,
        ProfileState::Pending
    );
}
#[test]
fn unknown_rules_duplicate_ids_and_conflicting_write_roots_are_rejected() {
    let mut p = profile("custom.new");
    p["roots"][0]["discovery"] = json!("run-shell");
    assert!(parse_agent_profile(&bytes(&p)).is_err());
    let mut p = profile("custom.new");
    let mut second = p["roots"][0].clone();
    second["rootId"] = json!("second");
    p["roots"].as_array_mut().unwrap().push(second);
    assert!(parse_agent_profile(&bytes(&p)).is_err());
    let env = env();
    let same = bytes(&profile("codex"));
    let source = source(&env, vec![same.clone(), same]);
    assert!(load_registry(&source, &env)
        .unwrap()
        .entries
        .iter()
        .all(|e| e.state == ProfileState::Invalid));
}
#[test]
fn paths_sizes_namespaces_and_variant_references_are_validated() {
    assert!(parse_agent_profile(&vec![b' '; MAX_PROFILE_BYTES + 1]).is_err());
    for path in ["../escape", "/absolute", "C:/escape", ""] {
        let mut p = profile("custom.x");
        p["roots"][0]["relative"] = json!(path);
        assert!(parse_agent_profile(&bytes(&p)).is_err());
    }
    let mut p = profile("custom.x");
    p["roots"][0]["appliesToVariants"] = json!(["unknown"]);
    assert!(parse_agent_profile(&bytes(&p)).is_err());
    let env = env();
    let source = source(&env, vec![]);
    fs::create_dir_all(&source.custom_dir).unwrap();
    fs::write(
        source.custom_dir.join("spoof.json"),
        bytes(&profile("codex")),
    )
    .unwrap();
    assert_eq!(
        load_registry(&source, &env).unwrap().entries[0].state,
        ProfileState::Invalid
    );
    let too_many = support::source(&env, vec![bytes(&profile("test")); 257]);
    assert!(load_registry(&too_many, &env).is_err());
}

#[test]
fn profile_collection_limits_accept_boundary_and_reject_one_more() {
    let mut p = profile("custom.limits");
    p["variants"] = json!((0..8).map(|i| format!("v{i}")).collect::<Vec<_>>());
    p["defaultVariant"] = json!("v0");
    p["roots"] = json!((0..32)
        .map(|i| root(
            &format!("r{i}"),
            "home",
            "user",
            &format!(".fixture/r{i}"),
            false
        ))
        .collect::<Vec<_>>());
    p["probes"] = json!((0..8).map(|i| json!({"probeId":format!("p{i}"),"base":"home","relative":"config.json","format":"json","keyPath":["enabled"],"expected":true,"onMissing":"unknown"})).collect::<Vec<_>>());
    assert!(parse_agent_profile(&bytes(&p)).is_ok());
    for field in ["variants", "roots", "probes"] {
        let mut extra = p.clone();
        let first = extra[field][0].clone();
        extra[field].as_array_mut().unwrap().push(first);
        assert!(parse_agent_profile(&bytes(&extra)).is_err());
    }
    let mut exact = bytes(&profile("custom.limits"));
    exact.resize(MAX_PROFILE_BYTES, b' ');
    assert!(parse_agent_profile(&exact).is_ok());
    exact.push(b' ');
    assert_eq!(
        parse_agent_profile(&exact).err().unwrap().code,
        "profile_too_large"
    );
    let deeply_nested = format!("{}0{}", "[".repeat(17), "]".repeat(17));
    assert_eq!(
        parse_agent_profile(deeply_nested.as_bytes())
            .err()
            .unwrap()
            .code,
        "profile_too_deep"
    );
    let env = env();
    let docs = (0..256)
        .map(|i| bytes(&profile(&format!("agent-{i}"))))
        .collect();
    assert_eq!(
        load_registry(&source(&env, docs), &env)
            .unwrap()
            .entries
            .len(),
        256
    );
}
#[test]
fn variant_roots_are_static_and_other_variants_remain_possible_readers() {
    let env = env();
    let mut p = profile("codex");
    p["variants"] = json!(["app", "cli"]);
    p["defaultVariant"] = json!("app");
    p["roots"][0]["appliesToVariants"] = json!(["app"]);
    let mut other = root("cli", "home", "user", ".cli/skills", true);
    other["appliesToVariants"] = json!(["cli"]);
    p["roots"].as_array_mut().unwrap().push(other);
    let registry = load_registry(&source(&env, vec![bytes(&p)]), &env).unwrap();
    let mut app = app(env, registry, Scope::User);
    let profile = app.context.registry.entries[0]
        .profile
        .as_ref()
        .unwrap()
        .clone();
    let roots = resolve_profile_roots(&profile, &app.context).unwrap();
    assert_eq!(roots.roots.iter().filter(|r| r.writable).count(), 1);
    assert!(roots.roots[0].path.ends_with(".example/skills"));
    let readers = reader_exposures(
        &app.context.env.home.join(".cli/skills/review"),
        &app.context,
    )
    .unwrap();
    assert!(readers.iter().any(|r| r.variant == "cli"));
    app.context
        .registry
        .selected_variants
        .insert(AgentId::parse("codex").unwrap(), "cli".into());
    assert!(resolve_profile_roots(&profile, &app.context)
        .unwrap()
        .roots
        .iter()
        .any(|r| r.writable && r.path.ends_with(".cli/skills")));
}
#[test]
fn missing_appdata_disables_root_without_fallback() {
    let env = env();
    let mut p = profile("codex");
    p["roots"][0]["base"] = json!("appdata");
    let profile = parse_agent_profile(&bytes(&p)).unwrap();
    let context = app(env, RegistrySnapshot::default(), Scope::User).context;
    let roots = resolve_profile_roots(&profile, &context).unwrap();
    assert!(roots.roots.is_empty());
    assert!(roots.issues.contains(&"missing_base"));
}

#[test]
fn shared_root_keeps_disabled_custom_agent_as_a_possible_reader() {
    let env = env();
    let source = source(&env, vec![bytes(&profile("codex"))]);
    fs::create_dir_all(&source.custom_dir).unwrap();
    let document = bytes(&profile("custom.shared"));
    fs::write(source.custom_dir.join("shared.json"), &document).unwrap();
    approve(&source, &env, document, false);
    let registry = load_registry(&source, &env).unwrap();
    let app = app(env, registry, Scope::User);
    let readers = reader_exposures(
        &app.context.env.home.join(".example/skills/review"),
        &app.context,
    )
    .unwrap();
    assert!(readers.iter().any(|r| r.agent.as_str() == "codex"));
    assert!(readers.iter().any(
        |r| r.agent.as_str() == "custom.shared" && r.evidence == skill_core::ReadState::Unknown
    ));
    assert_eq!(
        app.list_agent_skills(AgentId::parse("custom.shared").unwrap())
            .err()
            .unwrap()
            .code,
        "profile_not_enabled"
    );
}

#[test]
fn incomplete_override_evidence_retains_known_builtin_reader() {
    let env = env();
    let mut p = profile("grok");
    p["probes"] = json!([{"probeId":"enabled","base":"appdata","relative":"config.json","format":"json","keyPath":["enabled"],"expected":true,"onMissing":"unknown"}]);
    p["roots"][0]["readEvidence"] = json!({"kind":"bool-config-v1","probeId":"enabled"});
    let document = bytes(&p);
    let mut registry = load_registry(&source(&env, vec![document.clone()]), &env).unwrap();
    registry.approvals.push(Approval {
        id: AgentId::parse("grok").unwrap(),
        document,
        platform: env.platform,
        home: env.home.clone(),
        appdata: None,
        xdg_config: None,
        enabled: true,
        overrides: vec![skill_app::registry::LocalOverride {
            root_id: "unknown-root".into(),
            platform: env.platform,
            scope: Scope::User,
            project: None,
            path: env.home.join("unapproved"),
        }],
    });
    let app = app(env, registry, Scope::User);
    let readers = reader_exposures(
        &app.context.env.home.join(".example/skills/review"),
        &app.context,
    )
    .unwrap();
    assert!(readers
        .iter()
        .any(|r| r.agent.as_str() == "grok" && r.evidence == skill_core::ReadState::Unknown));
    assert!(!app.context.env.home.join("unapproved").exists());
}
#[test]
fn false_and_unknown_probes_do_not_switch_write_root_or_expose_other_fields() {
    let env = env();
    let mut p = profile("grok");
    p["probes"] = json!([{"probeId":"enabled","base":"home","relative":"config.toml","format":"toml","keyPath":["compat","skills"],"expected":true,"onMissing":"unknown"}]);
    p["roots"][0]["readEvidence"] = json!({"kind":"bool-config-v1","probeId":"enabled"});
    let registry = load_registry(&source(&env, vec![bytes(&p)]), &env).unwrap();
    let app = app(env, registry, Scope::User);
    let profile = app.context.registry.entries[0].profile.as_ref().unwrap();
    for (content, expected) in [
        (
            "[compat]\nskills = false\nprivate = 'SYNTHETIC_PRIVATE_MARKER'",
            skill_core::ReadState::ConfigDisabled,
        ),
        ("invalid = [", skill_core::ReadState::Unknown),
    ] {
        fs::write(app.context.env.home.join("config.toml"), content).unwrap();
        assert_eq!(
            resolve_profile_roots(profile, &app.context)
                .unwrap()
                .roots
                .iter()
                .filter(|r| r.writable)
                .count(),
            1
        );
        let readers = reader_exposures(
            &app.context.env.home.join(".example/skills/review"),
            &app.context,
        )
        .unwrap();
        assert!(!readers.is_empty());
        assert!(readers.iter().all(|r| r.evidence == expected));
        assert!(!format!("{readers:?}").contains("SYNTHETIC_PRIVATE_MARKER"));
    }
}
