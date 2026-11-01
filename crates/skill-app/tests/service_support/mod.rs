#![allow(dead_code)]
#[path="../support/mod.rs"] mod support;
pub use support::*;
use skill_app::*;
use skill_core::{plan::*,Scope,AgentId};
use std::{path::PathBuf,collections::BTreeMap};
pub fn ready()->(App,PathBuf,PathBuf,PathBuf){let env=env();let source_dir=env.home.join("source");package(&source_dir,"review");let source_dir=source_dir.join("review");let library=env.home.join(".skill-hub/library");let target=env.home.join(".example/skills/review");let registry=load_registry(&source(&env,vec![bytes(&profile("codex"))]),&env).unwrap();(app(env,registry,Scope::User),source_dir,library,target)}
pub fn import(app:&App,source:&std::path::Path,library:&std::path::Path)->String{let scan=app.scan_import(ImportRequest{source:source.into(),library_path:library.into()}).unwrap();let preview=app.preview_import(&scan.scan_id,&[ImportChoice{item_id:scan.items[0].item_id.clone(),action:ImportAction::New}]).unwrap();app.commit_import(&preview.preview_id).unwrap();app.load_library(library).unwrap().records[0].id.clone()}
pub fn request(id:&str,library:&std::path::Path)->DistributionRequest{DistributionRequest{library_path:library.into(),scope:Scope::User,project_root:None,selection:vec![SelectionPair{package_id:PackageId(id.into()),agent:AgentId::parse("codex").unwrap()}],conflict_decisions:BTreeMap::new()}}
