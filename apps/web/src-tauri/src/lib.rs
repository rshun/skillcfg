pub mod bridge;
mod commands;
pub mod directory;
use std::{collections::BTreeMap, sync::{Arc, Mutex}};
use bridge::{DesktopSession, SafeError};
use skill_app::HostEnv;

pub(crate) struct DesktopState { env: HostEnv, sessions: Mutex<BTreeMap<String,DesktopSession>> }
impl DesktopState {
    fn execute(&self,label:&str,command:&str,request:serde_json::Value)->Result<serde_json::Value,SafeError>{
        let mut sessions=self.sessions.lock().map_err(|_|SafeError::new("session_unavailable"))?;
        if !sessions.contains_key(label) {
            if sessions.len()>=8{return Err(SafeError::new("too_many_windows"));}
            sessions.insert(label.to_owned(),DesktopSession::with_env(self.env.clone())?);
        }
        sessions.get_mut(label).ok_or(SafeError::new("session_unavailable"))?.execute(command,request)
    }
}
fn host_env()->Result<HostEnv,SafeError>{
    let args:Vec<_>=std::env::args().collect();
    if let Some(index)=args.iter().position(|a|a=="--home") {
        let home=args.get(index+1).ok_or(SafeError::new("invalid_home"))?;
        let env=bridge::isolated_env(home.into());
        if !env.home.is_absolute(){return Err(SafeError::new("invalid_home"));}
        return Ok(env);
    }
    let home=std::env::var_os(if cfg!(windows){"USERPROFILE"}else{"HOME"}).ok_or(SafeError::new("home_unavailable"))?;
    let mut env=bridge::isolated_env(home.into());
    env.appdata=std::env::var_os("APPDATA").map(Into::into).or(env.appdata);
    env.xdg_config=std::env::var_os("XDG_CONFIG_HOME").map(Into::into).or(env.xdg_config);
    env.search_path=std::env::var_os("PATH").map_or_else(Vec::new,|p|std::env::split_paths(&p).collect());
    Ok(env)
}
pub fn run() {
    let env=match host_env(){Ok(e)=>e,Err(_)=>{eprintln!("desktop_environment_unavailable");std::process::exit(1)}};
    let state=Arc::new(DesktopState{env,sessions:Mutex::new(BTreeMap::new())}); let closed=state.clone();
    let result=tauri::Builder::default().plugin(tauri_plugin_dialog::init()).manage(state)
        .invoke_handler(tauri::generate_handler![commands::execute])
        .on_window_event(move|window,event|{
            if matches!(event,tauri::WindowEvent::Destroyed){
                let state=closed.clone();let label=window.label().to_owned();
                tauri::async_runtime::spawn_blocking(move||{if let Ok(mut sessions)=state.sessions.lock(){sessions.remove(&label);}});
            }
        }).run(tauri::generate_context!());
    if result.is_err(){eprintln!("desktop_runtime_failed");std::process::exit(1)}
}
