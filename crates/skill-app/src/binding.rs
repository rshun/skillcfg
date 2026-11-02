use crate::*;
use crate::{preview::{Stamp,error},settings::Authority};
use std::{path::{Path,PathBuf},collections::BTreeSet};
#[derive(Clone)]
pub(crate) struct ContextOverride{pub scope:skill_core::Scope,pub project:Option<PathBuf>,pub recovery:bool,pub extra_paths:Vec<PathBuf>}
pub(crate) struct FileBinding{pub path:PathBuf,pub data:Option<Vec<u8>>,pub key:Result<String,String>}
pub(crate) fn read_optional(path:&Path,limit:usize)->Result<Option<Vec<u8>>,AppError>{match skill_io::snapshot::read_bounded_file(path,limit){Ok(b)=>Ok(Some(b)),Err(e) if e.code=="not_found"=>Ok(None),Err(e)=>Err(e.into())}}
impl FileBinding{pub fn read(path:PathBuf)->Result<Self,AppError>{let data=read_optional(&path,4*1024*1024)?;let key=skill_io::paths::physical_key(&path).map_err(|e|e.code.to_owned());Ok(Self{path,data,key})}pub fn matches(&self)->bool{Self::read(self.path.clone()).is_ok_and(|b|b.data==self.data&&b.key==self.key)}}
struct Budget(usize);
impl Budget{fn add(&mut self,n:usize)->Result<(),AppError>{self.0=self.0.saturating_add(n);if self.0>64*1024*1024{Err(error("preview_too_large"))}else{Ok(())}}}
impl Stamp{pub(crate) fn bytes(&self)->usize{self.settings.as_ref().map_or(0,Vec::len)+self.authority.as_ref().map_or(0,Vec::len)+self.approvals.as_ref().map_or(0,Vec::len)+self.registry.iter().map(|(_,b)|b.as_ref().map_or(0,Vec::len)).sum::<usize>()+self.probes.iter().filter_map(|(_,v)|v.as_ref().ok()).filter_map(Option::as_ref).map(Vec::len).sum::<usize>()}}
impl App {
    pub(crate) fn context_for(&self,over:Option<&ContextOverride>)->Result<AppContext,AppError>{
        let mut ctx=match self.runtime_context(){Ok(c)=>c,Err(e) if over.is_some_and(|o|o.recovery)=>{let raw=read_optional(&self.authority_path(),4*1024*1024)?.ok_or(error("recovery_authority_missing"))?;let a:Authority=serde_json::from_slice(&raw).map_err(|_|error("invalid_state_roots"))?;if a.version!=1||a.libraries.len()>64||a.backups.len()>64{return Err(error("invalid_state_roots"));}let mut c=self.context.clone();c.registry=load_registry(&self.source,&self.context.env)?;c.registry.selected_variants=self.context.registry.selected_variants.clone();let _=e;c},Err(e)=>return Err(e)};
        if let Some(o)=over{ctx.scope=o.scope;if o.scope==skill_core::Scope::Project||o.project.is_some(){ctx.env.project_root=o.project.clone();}}
        Ok(ctx)
    }
    pub(crate) fn snapshot(&self,over:Option<&ContextOverride>)->Result<Stamp,AppError>{
        let ctx=self.context_for(over)?;let mut budget=Budget(0);let settings=read_optional(&self.settings_path(),4*1024*1024)?;budget.add(settings.as_ref().map_or(0,Vec::len))?;let authority=read_optional(&self.authority_path(),4*1024*1024)?;budget.add(authority.as_ref().map_or(0,Vec::len))?;
        let approvals=read_optional(self.source.approvals_path.as_ref().ok_or(error("missing_approvals_path"))?,64*1024*1024)?;budget.add(approvals.as_ref().map_or(0,Vec::len))?;
        let mut registry=Vec::new();match std::fs::read_dir(&self.source.custom_dir){Ok(entries)=>{for (n,entry) in entries.enumerate(){if n>=4096{return Err(error("too_many_profile_files"));}let entry=entry.map_err(|_|error("registry_read_failed"))?;if entry.path().extension().is_some_and(|e|e=="json"){if registry.len()>=256{return Err(error("too_many_profiles"));}let name=entry.file_name().into_string().map_err(|_|error("invalid_profile_path"))?;let data=skill_io::snapshot::read_bounded_file(&entry.path(),256*1024).ok();budget.add(data.as_ref().map_or(0,Vec::len))?;registry.push((name,data));}}},Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},Err(_)=>return Err(error("registry_read_failed"))}registry.sort();
        let mut paths=BTreeSet::new();paths.insert(self.context.env.home.join(".skill-hub"));paths.insert(self.source.custom_dir.clone());
        if let Ok((s,_,_))=self.settings_data(){paths.insert(s.library_path);paths.insert(s.backup_path);}
        if let Some(raw)=&authority{let a:Authority=serde_json::from_slice(raw).map_err(|_|error("invalid_state_roots"))?;if a.version!=1||a.libraries.len()>64||a.backups.len()>64{return Err(error("invalid_state_roots"));}for p in a.libraries.into_iter().chain(a.backups){self.basic_location(&p)?;paths.insert(p);}}
        if let Some(path)=&self.state.borrow().library{paths.insert(path.clone());}
        if let Some(over)=over{paths.extend(over.extra_paths.iter().cloned());}
        let mut probe_paths=BTreeSet::new();let mut unauthorized_probes=BTreeSet::new();
        for entry in &ctx.registry.entries{if !matches!(entry.state,ProfileState::Builtin|ProfileState::Enabled|ProfileState::Disabled){continue;}if let Some(p)=&entry.profile{
            for variant in &p.variants{for scope in [skill_core::Scope::User,skill_core::Scope::Project]{if scope==skill_core::Scope::Project&&ctx.env.project_root.is_none(){continue;}let mut local=ctx.clone();local.scope=scope;local.registry.selected_variants.insert(p.id.clone(),variant.clone());for root in crate::roots::resolve_profile_roots(p,&local)?.roots{paths.insert(root.path);if paths.len()>16384{return Err(error("too_many_resolved_roots"));}}}}
            for probe in &p.probes{if let Some(base)=ctx.env.base(probe.base){let path=base.join(&probe.relative);let mut local=ctx.clone();if probe.base==skill_core::profile::Base::Project{local.scope=skill_core::Scope::Project;}if self.profile_authorized(p,&local,false).is_err(){unauthorized_probes.insert(path.clone());}probe_paths.insert(path);}}
        }}
        let mut probes=Vec::new();for path in probe_paths{let value=if unauthorized_probes.contains(&path){Err("not_authorized".into())}else{match read_optional(&path,4*1024*1024){Ok(v)=>Ok(v),Err(e)=>Err(e.code.into())}};if let Ok(Some(bytes))=&value{budget.add(bytes.len())?;}paths.insert(path.clone());probes.push((path,value));}
        let mut locations=Vec::new();for path in paths{budget.add(path.as_os_str().len())?;let value=skill_io::disk::resolved_path(&path).and_then(|actual|skill_io::paths::physical_key(&path).map(|key|(actual,key))).map_err(|e|e.code.to_owned());locations.push((path,value));}
        let context=format!("{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",ctx.env.home,ctx.env.appdata,ctx.env.xdg_config,ctx.env.platform,ctx.env.project_root,ctx.scope,ctx.registry.selected_variants,self.context.env.project_root,self.context.scope,self.context.registry.selected_variants);
        Ok(Stamp{settings,authority,registry,approvals,context,probes,locations})
    }
}
