use skill_core::{plan::*,diff::redacted_plan_dto,*};
use std::{collections::BTreeMap,path::PathBuf};
fn package(id:&str,name:&str,body:&str)->LibraryPackage {
    let text=format!("---\nname: {name}\ndescription: Fixture.\n---\n{body}");
    LibraryPackage{id:PackageId(id.into()),metadata:SkillMetadata{name:name.into(),description:"Fixture.".into()},tree:PackageTree{entries:vec![TreeEntry{relative_path:"SKILL.md".into(),kind:EntryKind::File{bytes:text.into_bytes(),unix_executable:false}}]}}
}
fn target(p:&LibraryPackage,agent:&str,key:&str,before:Option<PackageTree>)->TargetSnapshot {TargetSnapshot{package_id:p.id.clone(),agent:AgentId::parse(agent).unwrap(),key:TargetKey(key.into()),path:PathBuf::from(format!("fixture/{key}/{}",p.metadata.name)),platform:Platform::Linux,tree:before,possible_readers:vec![],blocked:None}}
fn pair(p:&LibraryPackage,agent:&str)->SelectionPair{SelectionPair{package_id:p.id.clone(),agent:AgentId::parse(agent).unwrap()}}
#[test]
fn two_pairs_do_not_expand_to_four() {
    let a=package("a","review","one");let b=package("b","debug","two");
    let targets=vec![target(&a,"codex","a",None),target(&b,"custom.fifth","b",None)];
    let result=plan_distribution(&[a.clone(),b.clone()],&targets,&[pair(&a,"codex"),pair(&b,"custom.fifth")],&BTreeMap::new()).unwrap();
    assert_eq!(result.groups.len(),2);assert_eq!(result.groups[0].selected_agents.len(),1);assert_eq!(result.groups[1].selected_agents.len(),1);
}
#[test]
fn shared_target_has_one_change_and_all_possible_readers() {
    let p=package("p","review","new");let mut a=target(&p,"codex","shared",None);let mut b=target(&p,"custom.fifth","shared",None);
    let reader=ReaderExposure{agent:AgentId::parse("grok").unwrap(),variant:"cli".into(),root_id:"compat".into(),evidence:ReadState::Unknown};a.possible_readers.push(reader.clone());b.possible_readers.push(reader);
    let result=plan_distribution(&[p.clone()],&[a,b],&[pair(&p,"codex"),pair(&p,"custom.fifth")],&BTreeMap::new()).unwrap();
    assert_eq!(result.groups.len(),1);assert_eq!(result.groups[0].selected_agents.len(),2);assert_eq!(result.groups[0].possible_readers.len(),1);assert_eq!(result.groups[0].disposition,Disposition::New);
}
#[test]
fn two_versions_for_same_target_block() {
    let a=package("a","review","old");let b=package("b","review","different");let targets=[target(&a,"codex","shared",None),target(&b,"grok","shared",None)];
    let result=plan_distribution(&[a.clone(),b.clone()],&targets,&[pair(&a,"codex"),pair(&b,"grok")],&BTreeMap::new()).unwrap();assert_eq!(result.groups[0].disposition,Disposition::Blocked);
}
#[test]
fn unmanaged_difference_defaults_to_skip_and_replace_lists_extra_old_files() {
    let p=package("p","review","new");let mut old=package("old","review","old").tree;old.entries.push(TreeEntry{relative_path:"user-notes.txt".into(),kind:EntryKind::File{bytes:b"preserve in archive".to_vec(),unix_executable:false}});
    let targets=[target(&p,"codex","one",Some(old))];let selection=[pair(&p,"codex")];
    let skipped=plan_distribution(&[p.clone()],&targets,&selection,&BTreeMap::new()).unwrap();assert_eq!(skipped.groups[0].disposition,Disposition::Skipped);
    let decisions=BTreeMap::from([(TargetKey("one".into()),ConflictDecision::Replace)]);
    let replace=plan_distribution(&[p],&targets,&selection,&decisions).unwrap();assert_eq!(replace.groups[0].disposition,Disposition::Update);assert_eq!(replace.groups[0].comparison.removed,vec!["user-notes.txt"]);
}
#[test]
fn identical_and_empty_selection_have_no_writes() {
    let p=package("p","review","same");let targets=[target(&p,"codex","one",Some(p.tree.clone()))];
    let result=plan_distribution(&[p.clone()],&targets,&[pair(&p,"codex")],&BTreeMap::new()).unwrap();assert_eq!(result.groups[0].disposition,Disposition::Unchanged);
    assert!(plan_distribution(&[],&[],&[],&BTreeMap::new()).unwrap().groups.is_empty());
}
#[test]
fn stale_shared_baselines_and_link_targets_block() {
    let p=package("p","review","new");let a=target(&p,"codex","shared",None);let b=target(&p,"grok","shared",Some(p.tree.clone()));
    let result=plan_distribution(&[p.clone()],&[a,b],&[pair(&p,"codex"),pair(&p,"grok")],&BTreeMap::new()).unwrap();assert_eq!(result.groups[0].disposition,Disposition::Blocked);
    let mut linked=target(&p,"codex","link",None);linked.blocked=Some("target_link");
    let result=plan_distribution(&[p.clone()],&[linked],&[pair(&p,"codex")],&BTreeMap::new()).unwrap();assert_eq!(result.groups[0].reason,Some("target_link"));
}
#[test]
fn unsafe_assignment_is_blocked_and_public_diff_does_not_reveal_it() {
    let p=package("p","review","password: 'SYNTHETIC_PRIVATE_MARKER'");let targets=[target(&p,"codex","one",None)];
    let result=plan_distribution(&[p.clone()],&targets,&[pair(&p,"codex")],&BTreeMap::new()).unwrap();assert_eq!(result.groups[0].disposition,Disposition::Blocked);assert!(!format!("{:?}",redacted_plan_dto(&result)).contains("SYNTHETIC_PRIVATE_MARKER"));
}
#[test]
fn binary_changes_are_summaries_and_metadata_cache_mismatch_is_rejected() {
    let mut p=package("p","review","safe");p.tree.entries.push(TreeEntry{relative_path:"image.bin".into(),kind:EntryKind::File{bytes:vec![0,255],unix_executable:false}});
    let targets=[target(&p,"codex","one",None)];let result=plan_distribution(&[p.clone()],&targets,&[pair(&p,"codex")],&BTreeMap::new()).unwrap();let view=redacted_plan_dto(&result);assert!(view.groups[0].files.iter().any(|f|f.path=="image.bin" && f.binary && f.after.is_none()));
    p.metadata.name="different".into();assert!(plan_distribution(&[p.clone()],&targets,&[pair(&p,"codex")],&BTreeMap::new()).is_err());
}
