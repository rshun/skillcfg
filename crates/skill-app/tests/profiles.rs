mod support;
use support::*;
use skill_app::*;
use skill_app::roots::{resolve_profile_roots,reader_exposures};
use skill_core::{profile::{parse_agent_profile,MAX_PROFILE_BYTES},AgentId,Scope};
use serde_json::json;
use std::fs;

#[test]
fn fifth_standard_agent_is_loaded_from_json_only() {
    let env=env();let source=source(&env,vec![bytes(&profile("codex"))]);fs::create_dir_all(&source.custom_dir).unwrap();
    let data=bytes(&profile("custom.fifth-agent"));fs::write(source.custom_dir.join("fifth.json"),&data).unwrap();approve(&source,&env,data,true);
    let registry=load_registry(&source,&env).unwrap();
    assert_eq!(registry.entries.len(),2);
    assert!(registry.entries.iter().any(|e|e.state==ProfileState::Enabled && e.profile.as_ref().unwrap().id.as_str()=="custom.fifth-agent"));
}
#[test]
fn external_profile_edit_revokes_approval() {
    let env=env();let source=source(&env,vec![]);fs::create_dir_all(&source.custom_dir).unwrap();
    let original=bytes(&profile("custom.fifth"));approve(&source,&env,original,true);
    let mut changed=profile("custom.fifth");changed["roots"][0]["relative"]=json!(".other/skills");fs::write(source.custom_dir.join("fifth.json"),bytes(&changed)).unwrap();
    assert_eq!(load_registry(&source,&env).unwrap().entries[0].state,ProfileState::Pending);
}
#[test]
fn unknown_rules_duplicate_ids_and_conflicting_write_roots_are_rejected() {
    let mut p=profile("custom.new");p["roots"][0]["discovery"]=json!("run-shell");assert!(parse_agent_profile(&bytes(&p)).is_err());
    let mut p=profile("custom.new");let mut second=p["roots"][0].clone();second["rootId"]=json!("second");p["roots"].as_array_mut().unwrap().push(second);assert!(parse_agent_profile(&bytes(&p)).is_err());
    let env=env();let same=bytes(&profile("codex"));let source=source(&env,vec![same.clone(),same]);
    assert!(load_registry(&source,&env).unwrap().entries.iter().all(|e|e.state==ProfileState::Invalid));
}
#[test]
fn paths_sizes_namespaces_and_variant_references_are_validated() {
    assert!(parse_agent_profile(&vec![b' ';MAX_PROFILE_BYTES+1]).is_err());
    for path in ["../escape","/absolute","C:/escape",""] {let mut p=profile("custom.x");p["roots"][0]["relative"]=json!(path);assert!(parse_agent_profile(&bytes(&p)).is_err());}
    let mut p=profile("custom.x");p["roots"][0]["appliesToVariants"]=json!(["unknown"]);assert!(parse_agent_profile(&bytes(&p)).is_err());
    let env=env();let source=source(&env,vec![]);fs::create_dir_all(&source.custom_dir).unwrap();fs::write(source.custom_dir.join("spoof.json"),bytes(&profile("codex"))).unwrap();
    assert_eq!(load_registry(&source,&env).unwrap().entries[0].state,ProfileState::Invalid);
    let too_many=support::source(&env,vec![bytes(&profile("test"));257]);assert!(load_registry(&too_many,&env).is_err());
}
#[test]
fn variant_roots_are_static_and_other_variants_remain_possible_readers() {
    let env=env();let mut p=profile("codex");p["variants"]=json!(["app","cli"]);p["defaultVariant"]=json!("app");p["roots"][0]["appliesToVariants"]=json!(["app"]);
    let mut other=root("cli","home","user",".cli/skills",true);other["appliesToVariants"]=json!(["cli"]);p["roots"].as_array_mut().unwrap().push(other);
    let registry=load_registry(&source(&env,vec![bytes(&p)]),&env).unwrap();let mut app=app(env,registry,Scope::User);
    let profile=app.context.registry.entries[0].profile.as_ref().unwrap().clone();
    let roots=resolve_profile_roots(&profile,&app.context).unwrap();assert_eq!(roots.roots.iter().filter(|r|r.writable).count(),1);assert!(roots.roots[0].path.ends_with(".example/skills"));
    let readers=reader_exposures(&app.context.env.home.join(".cli/skills/review"),&app.context).unwrap();assert!(readers.iter().any(|r|r.variant=="cli"));
    app.context.registry.selected_variants.insert(AgentId::parse("codex").unwrap(),"cli".into());
    assert!(resolve_profile_roots(&profile,&app.context).unwrap().roots.iter().any(|r|r.writable && r.path.ends_with(".cli/skills")));
}
#[test]
fn missing_appdata_disables_root_without_fallback() {
    let env=env();let mut p=profile("codex");p["roots"][0]["base"]=json!("appdata");let profile=parse_agent_profile(&bytes(&p)).unwrap();
    let context=app(env,RegistrySnapshot::default(),Scope::User).context;let roots=resolve_profile_roots(&profile,&context).unwrap();
    assert!(roots.roots.is_empty());assert!(roots.issues.contains(&"missing_base"));
}
#[test]
fn false_and_unknown_probes_do_not_switch_write_root_or_expose_other_fields() {
    let env=env();let mut p=profile("grok");p["probes"]=json!([{"probeId":"enabled","base":"home","relative":"config.toml","format":"toml","keyPath":["compat","skills"],"expected":true,"onMissing":"unknown"}]);
    p["roots"][0]["readEvidence"]=json!({"kind":"bool-config-v1","probeId":"enabled"});
    let registry=load_registry(&source(&env,vec![bytes(&p)]),&env).unwrap();let app=app(env,registry,Scope::User);let profile=app.context.registry.entries[0].profile.as_ref().unwrap();
    for content in ["[compat]\nskills = false\nprivate = 'SYNTHETIC_PRIVATE_MARKER'","invalid = ["] {
        fs::write(app.context.env.home.join("config.toml"),content).unwrap();
        assert_eq!(resolve_profile_roots(profile,&app.context).unwrap().roots.iter().filter(|r|r.writable).count(),1);
        let readers=reader_exposures(&app.context.env.home.join(".example/skills/review"),&app.context).unwrap();assert!(!readers.is_empty());assert!(!format!("{readers:?}").contains("SYNTHETIC_PRIVATE_MARKER"));
    }
}
