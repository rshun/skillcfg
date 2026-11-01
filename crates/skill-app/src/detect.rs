use crate::{roots::ResolvedRoot, App, AppError};
use skill_core::{AgentId, SkillMetadata};
use std::path::PathBuf;

pub struct InventoryItem {
    pub physical_key: String,
    pub metadata: Option<SkillMetadata>,
    pub sources: Vec<PathBuf>,
    pub files: Vec<String>,
    pub issues: Vec<&'static str>,
}
pub struct AgentInventory {
    pub agent: AgentId,
    pub installed: bool,
    pub roots: Vec<ResolvedRoot>,
    pub items: Vec<InventoryItem>,
    pub errors: Vec<&'static str>,
}
impl App {
    pub fn list_agent_skills(&self, agent: AgentId) -> Result<AgentInventory, AppError> {
        use std::{fs,collections::BTreeMap};
        use skill_core::{parse_skill_document,validate_package,EntryKind};
        let entry=self.context.registry.entries.iter().find(|e|e.profile.as_ref().is_some_and(|p|p.id==agent)).ok_or(AppError{code:"unknown_agent"})?;
        if !matches!(entry.state,crate::ProfileState::Builtin|crate::ProfileState::Enabled){return Err(AppError{code:"profile_not_enabled"});}
        let profile=entry.profile.as_ref().unwrap();let resolved=crate::roots::resolve_profile_roots(profile,&self.context)?;
        let installed=self.context.env.search_path.iter().any(|dir|profile.binary_names.iter().any(|name|{
            let suffixes=if self.context.env.platform==skill_core::Platform::Windows{vec![".exe",".cmd",".bat",""]}else{vec![""]};suffixes.iter().any(|suffix|dir.join(format!("{name}{suffix}")).is_file())
        }));
        let mut result=AgentInventory{agent,installed,roots:resolved.roots,items:vec![],errors:resolved.issues};let mut indices=BTreeMap::new();
        for root in &result.roots {
            let children=match fs::read_dir(&root.path){Ok(c)=>c,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>continue,Err(_)=>{result.errors.push("root_read_failed");continue;}};
            for (number,child) in children.enumerate(){
                if number>=1024{result.errors.push("inventory_too_large");break;}
                let child=match child{Ok(c)=>c,Err(_)=>{result.errors.push("root_read_failed");continue;}};let path=child.path();
                match fs::symlink_metadata(path.join("SKILL.md")){Ok(_)=>{},Err(e) if e.kind()==std::io::ErrorKind::NotFound=>continue,Err(_)=>{result.errors.push("package_read_failed");continue;}}
                match skill_io::read_package_snapshot(&path,&skill_io::ScanLimits::default()){
                    Ok(snapshot)=>{
                        if let Some(index)=indices.get(&snapshot.physical_key){let item:&mut InventoryItem=&mut result.items[*index];item.sources.push(path);continue;}
                        let validation=validate_package(&snapshot.tree,self.context.env.platform);
                        let metadata=if validation.is_empty(){snapshot.tree.entries.iter().find_map(|e|if e.relative_path=="SKILL.md"{if let EntryKind::File{bytes,..}=&e.kind{parse_skill_document(bytes).ok()}else{None}}else{None})}else{None};
                        let files=snapshot.tree.entries.iter().map(|e|skill_core::compatibility::safe_text(&e.relative_path)).collect();
                        indices.insert(snapshot.physical_key.clone(),result.items.len());
                        result.items.push(InventoryItem{physical_key:snapshot.physical_key,metadata,sources:vec![path],files,issues:validation.into_iter().map(|i|i.code).collect()});
                    }
                    Err(e)=>result.errors.push(e.code),
                }
                if result.items.len()>=1024{result.errors.push("inventory_too_large");break;}
            }
        }
        Ok(result)
    }
}
