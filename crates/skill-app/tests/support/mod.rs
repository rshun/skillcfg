#![allow(dead_code)]
use skill_app::*;
use skill_core::{AgentId, Platform, Scope};
use serde_json::{json,Value};
use std::{path::{Path,PathBuf},fs,sync::atomic::{AtomicU64,Ordering}};

pub fn temp() -> PathBuf {
    static SERIAL:AtomicU64=AtomicU64::new(0);
    let p=std::env::temp_dir().join(format!("skillhub-app-{}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),SERIAL.fetch_add(1,Ordering::Relaxed)));
    fs::create_dir_all(&p).unwrap();p
}
pub fn env() -> HostEnv {HostEnv{home:temp(),appdata:None,xdg_config:None,project_root:None,search_path:vec![],platform:Platform::Linux}}
pub fn profile(id:&str)->Value {json!({"schemaVersion":1,"id":id,"displayName":"Example Agent","packageFormat":"skill-md-v1","variants":["default"],"defaultVariant":"default","roots":[root("native","home","user",".example/skills",true)]})}
pub fn root(id:&str,base:&str,scope:&str,relative:&str,write:bool)->Value {json!({"rootId":id,"platforms":["windows","macos","linux"],"scope":scope,"base":base,"relative":relative,"discovery":if scope=="project"{"ancestors-to-repo"}else{"children"},"role":"native","read":true,"write":write,"priority":100})}
pub fn bytes(v:&Value)->Vec<u8>{serde_json::to_vec(v).unwrap()}
pub fn source(env:&HostEnv,builtins:Vec<Vec<u8>>)->RegistrySource {RegistrySource{builtins,custom_dir:env.home.join(".skill-hub/agents"),approvals_path:Some(env.home.join(".skill-hub/agent-approvals.json"))}}
pub fn approve(source:&RegistrySource,env:&HostEnv,document:Vec<u8>,enabled:bool) {
    let id=AgentId::parse(serde_json::from_slice::<Value>(&document).unwrap()["id"].as_str().unwrap()).unwrap();
    let approval=Approval{id,document,platform:env.platform,home:env.home.clone(),appdata:env.appdata.clone(),xdg_config:env.xdg_config.clone(),enabled,overrides:Default::default()};
    let path=source.approvals_path.as_ref().unwrap();fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path,serde_json::to_vec(&vec![approval]).unwrap()).unwrap();
}
pub fn package(root:&Path,name:&str) {fs::create_dir_all(root.join(name)).unwrap();fs::write(root.join(name).join("SKILL.md"),format!("---\nname: {name}\ndescription: Review fixture.\n---\n")).unwrap();}
pub fn app(env:HostEnv,registry:RegistrySnapshot,scope:Scope)->App {App{context:AppContext{env,registry,scope}}}
