use crate::*;
use crate::preview::{error,file_change};
use skill_io::{MutationPlan,disk::{optional_file,resolved_path}};
use std::path::{Path,PathBuf};
#[derive(serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub(crate) struct SettingsDocument{version:u32,settings:GuiSettings,backup_locations:Vec<PathBuf>}
#[derive(Default,Clone,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub(crate) struct Authority{pub version:u32,pub libraries:Vec<PathBuf>,pub backups:Vec<PathBuf>}
impl App {
    pub(crate) fn settings_path(&self)->PathBuf{self.context.env.home.join(".skill-hub/settings.json")}
    pub(crate) fn authority_path(&self)->PathBuf{self.context.env.home.join(".skill-hub/state-roots.json")}
    pub(crate) fn settings_data(&self)->Result<(GuiSettings,Vec<PathBuf>,Option<Vec<u8>>),AppError>{
        let raw=optional_file(&self.settings_path())?;
        if let Some(bytes)=&raw{let doc:SettingsDocument=serde_json::from_slice(bytes).map_err(|_|error("invalid_settings"))?;if doc.version!=1||doc.settings.version!=1||doc.backup_locations.len()>64{return Err(error("invalid_settings"));}self.basic_location(&doc.settings.library_path)?;self.basic_location(&doc.settings.backup_path)?;for p in &doc.backup_locations{self.basic_location(p)?;}Ok((doc.settings,doc.backup_locations,raw))}
        else{let settings=GuiSettings{version:1,library_path:self.context.env.home.join(".skill-hub/library"),backup_path:self.context.env.home.join(".skill-hub/backups"),default_scope:skill_core::Scope::User,last_project_dir:None,agent_variants:Default::default()};let roots=vec![settings.backup_path.clone()];Ok((settings,roots,None))}
    }
    pub(crate) fn basic_location(&self,path:&Path)->Result<PathBuf,AppError>{let actual=resolved_path(path)?;let home=resolved_path(&self.context.env.home)?;let hub=home.join(".skill-hub");if actual.parent().is_none()||home.starts_with(&actual)||actual==hub||[hub.join("agents"),hub.join("audit")].iter().any(|p|overlap(&actual,p)){return Err(error("protected_location"));}Ok(actual)}
    pub(crate) fn check_library_location(&self,path:&Path)->Result<PathBuf,AppError>{let actual=self.basic_location(path)?;let (settings,backups,_)=self.settings_data()?;for root in backups.iter().chain(std::iter::once(&settings.backup_path)){if overlap(&actual,&resolved_path(root)?){return Err(error("library_backup_overlap"));}}let ctx=self.runtime_context()?;if self.known_roots(&ctx)?.iter().any(|p|overlap(&actual,p)){return Err(error("library_agent_overlap"));}Ok(actual)}
    pub(crate) fn authority(&self)->Result<(Authority,Option<Vec<u8>>),AppError>{let raw=optional_file(&self.authority_path())?;let mut a=if let Some(bytes)=&raw{let a:Authority=serde_json::from_slice(bytes).map_err(|_|error("invalid_state_roots"))?;if a.version!=1||a.libraries.len()>64||a.backups.len()>64{return Err(error("invalid_state_roots"));}a}else{Authority{version:1,..Default::default()}};let (settings,backups,_)=self.settings_data()?;for p in a.libraries.iter().chain(a.backups.iter()){self.basic_location(p)?;}a.libraries.push(settings.library_path);a.backups.extend(backups);a.backups.push(settings.backup_path);if let Some(p)=&self.state.borrow().library{a.libraries.push(p.clone());}a.libraries.sort();a.libraries.dedup();a.backups.sort();a.backups.dedup();Ok((a,raw))}
    pub(crate) fn authority_change(&self,library:Option<&Path>,backup:Option<&Path>)->Result<skill_io::FileChange,AppError>{let (mut a,before)=self.authority()?;if let Some(p)=library{a.libraries.push(self.basic_location(p)?);}if let Some(p)=backup{a.backups.push(self.basic_location(p)?);}a.libraries.sort();a.libraries.dedup();a.backups.sort();a.backups.dedup();if a.libraries.len()>64||a.backups.len()>64{return Err(error("too_many_state_locations"));}let after=serde_json::to_vec(&a).map_err(|_|error("invalid_state_roots"))?;Ok(file_change(self.authority_path(),before,Some(after)))}
    pub fn load_settings(&self)->Result<SettingsView,AppError>{let (settings,_,raw)=self.settings_data()?;let revision=skill_io::disk::operation_id();let mut state=self.state.borrow_mut();if state.settings_tokens.len()>=16{state.settings_tokens.clear();}state.settings_tokens.insert(revision.clone(),raw);Ok(SettingsView{settings,revision,warnings:vec![]})}
    pub fn save_settings(&self,req:SettingsRequest)->Result<SettingsView,AppError>{
        self.state.borrow_mut().preview=None;let expected=self.state.borrow_mut().settings_tokens.remove(&req.expected_revision).ok_or(error("settings_revision_unknown"))?;let (_,mut backups,current)=self.settings_data()?;if current!=expected{return Err(error("settings_changed"));}
        let settings=req.settings;if settings.version!=1{return Err(error("invalid_settings"));}let library=self.basic_location(&settings.library_path)?;let backup=self.basic_location(&settings.backup_path)?;if overlap(&library,&backup){return Err(error("library_backup_overlap"));}let ctx=self.runtime_context()?;for root in self.known_roots(&ctx)?{if overlap(&root,&library)||overlap(&root,&backup){return Err(error("state_agent_overlap"));}}
        for (id,variant) in &settings.agent_variants{let profile=ctx.registry.entries.iter().find_map(|e|e.profile.as_ref().filter(|p|&p.id==id)).ok_or(error("unknown_agent"))?;if !profile.variants.contains(variant){return Err(error("invalid_variant"));}}
        if let Some(project)=&settings.last_project_dir{if !project.is_absolute()||!project.is_dir(){return Err(error("invalid_project"));}}
        backups.push(settings.backup_path.clone());backups.sort();backups.dedup();let authority=self.authority_change(Some(&library),Some(&backup))?;
        let after=serde_json::to_vec(&SettingsDocument{version:1,settings:settings.clone(),backup_locations:backups}).map_err(|_|error("invalid_settings"))?;
        let plan=MutationPlan{files:vec![authority,file_change(self.settings_path(),current,Some(after))],kind:"settings".into(),..Default::default()};let engine=self.engine(vec![],vec![self.authority_path(),self.settings_path()],None)?;let receipt=engine.apply(engine.prepare(&plan)?);if receipt.outcomes.iter().any(|i|!matches!(i.outcome,skill_io::Outcome::Written|skill_io::Outcome::Unchanged)){return Err(error("settings_save_incomplete"));}self.load_settings()
    }
}
pub(crate) fn overlap(a:&Path,b:&Path)->bool{a.starts_with(b)||b.starts_with(a)}
