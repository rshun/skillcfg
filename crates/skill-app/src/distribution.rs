use crate::*;
use crate::{preview::{error,directory_change,tree_bytes},binding::{ContextOverride,FileBinding},library::metadata};
use skill_core::{plan::*,Scope};
use std::{collections::{BTreeMap,BTreeSet},path::PathBuf};
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct DistributionRecord{pub package_id:String,pub revision_id:String,pub name_at_write:String,pub scope:Scope,pub project:Option<PathBuf>,pub physical_path:PathBuf,pub selected_profiles:Vec<ProfileAtWrite>}
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct ProfileAtWrite{pub agent_id:skill_core::AgentId,pub variant_id:String,pub root_id:String}
impl App {
    pub fn preview_distribution(&self,req:DistributionRequest)->Result<PreviewDto,AppError>{
        self.state.borrow_mut().preview=None;
        self.state.borrow_mut().library=Some(req.library_path.clone());
        let project=if req.scope==Scope::Project{let p=req.project_root.as_ref().filter(|p|p.is_absolute()&&p.is_dir()).ok_or(error("invalid_project"))?;Some(skill_io::disk::resolved_path(p)?)}else{req.project_root.clone()};
        let mut over=ContextOverride{scope:req.scope,project:project.clone(),recovery:false,extra_paths:vec![]};let stamp=self.snapshot(Some(&over))?;
        let loaded=self.checked_library(&req.library_path)?;let catalog_binding=FileBinding::read(loaded.path.join("catalog.json"))?;if catalog_binding.data!=loaded.raw{return Err(error("library_changed"));}let auxiliary=vec![catalog_binding];let ctx=self.context_for(Some(&over))?;
        let selected:BTreeSet<_>=req.selection.iter().cloned().collect();if selected.len()>4096{return Err(error("too_many_pairs"));}
        let mut budget=stamp.bytes().saturating_add(loaded.bindings.iter().map(|b|tree_bytes(&b.tree)).sum::<usize>());if budget>64*1024*1024{return Err(error("preview_too_large"));}
        let mut targets=Vec::new();let mut roots=Vec::new();let mut profile_at=BTreeMap::new();let mut wanted=BTreeSet::new();
        for pair in &selected{
            let index=loaded.catalog.records.iter().position(|r|r.id==pair.package_id.0).ok_or(error("unknown_package"))?;let record=&loaded.catalog.records[index];wanted.insert(index);
            let entry=ctx.registry.entries.iter().find(|e|e.profile.as_ref().is_some_and(|p|p.id==pair.agent)).ok_or(error("unknown_agent"))?;
            if !matches!(entry.state,ProfileState::Builtin|ProfileState::Enabled){return Err(error("profile_not_enabled"));}let p=entry.profile.as_ref().ok_or(error("invalid_profile"))?;self.profile_authorized(p,&ctx,true)?;
            let resolved=crate::roots::resolve_profile_roots(p,&ctx)?;let write:Vec<_>=resolved.roots.into_iter().filter(|r|r.writable).collect();if write.len()!=1{return Err(error("write_root_unavailable"));}let root=&write[0];let actual_root=skill_io::disk::resolved_path(&root.path)?;let target=actual_root.join(&record.name);let key=TargetKey(skill_io::paths::physical_key(&target)?);
            let (tree,blocked)=match skill_io::disk::optional_tree(&target){Ok(tree)=>(tree,None),Err(e)=>(None,Some(e.code))};budget=budget.saturating_add(tree.as_ref().map_or(0,tree_bytes));if budget>64*1024*1024{return Err(error("preview_too_large"));}
            let readers=crate::roots::reader_exposures(&target,&ctx)?;
            profile_at.insert(pair.clone(),ProfileAtWrite{agent_id:pair.agent.clone(),variant_id:root.variant.clone(),root_id:root.root_id.clone()});roots.push(actual_root);
            targets.push(TargetSnapshot{package_id:pair.package_id.clone(),agent:pair.agent.clone(),key,path:target,platform:ctx.env.platform,tree,possible_readers:readers,blocked});
        }
        let packages:Vec<_>=wanted.iter().map(|&i|Ok(LibraryPackage{id:PackageId(loaded.catalog.records[i].id.clone()),metadata:metadata(&loaded.bindings[i].tree)?,tree:loaded.bindings[i].tree.clone()})).collect::<Result<_,AppError>>()?;
        let core=plan_distribution(&packages,&targets,&req.selection,&req.conflict_decisions)?;
        if req.conflict_decisions.keys().any(|key|!core.groups.iter().any(|g|&g.key==key)){return Err(error("unknown_conflict_target"));}
        let materialized=stamp.bytes().saturating_add(loaded.bindings.iter().map(|b|tree_bytes(&b.tree)).sum::<usize>()).saturating_add(core.groups.iter().filter(|g|matches!(g.disposition,Disposition::New|Disposition::Update)).map(|g|g.before.as_ref().map_or(0,tree_bytes).saturating_add(tree_bytes(&g.after))).sum::<usize>());
        if materialized>64*1024*1024{return Err(error("preview_too_large"));}
        let blocked=core.groups.iter().any(|g|g.disposition==Disposition::Blocked);let mut plan=skill_io::MutationPlan{kind:"distribution".into(),..Default::default()};let mut outcomes=Vec::new();let mut records=Vec::new();
        for group in &core.groups{if matches!(group.disposition,Disposition::New|Disposition::Update){plan.directories.push(directory_change(group.path.clone(),group.before.clone(),Some(group.after.as_ref().clone())));
            for id in &group.package_ids{let record=loaded.catalog.records.iter().find(|r|&r.id==&id.0).ok_or(error("unknown_package"))?;let selected_profiles=group.selected_agents.iter().filter_map(|a|profile_at.get(&SelectionPair{package_id:id.clone(),agent:a.clone()}).cloned()).collect();records.push(DistributionRecord{package_id:id.0.clone(),revision_id:record.current_revision.clone(),name_at_write:record.name.clone(),scope:req.scope,project:project.clone(),physical_path:group.path.clone(),selected_profiles});}
        }else{outcomes.push(SafeOutcome{path:skill_core::compatibility::safe_text(&group.path.to_string_lossy()),outcome:match group.disposition{Disposition::Unchanged=>skill_io::Outcome::Unchanged,Disposition::Skipped=>skill_io::Outcome::Skipped,_=>skill_io::Outcome::Blocked},reason:group.reason.map(str::to_owned)});}}
        plan.metadata.insert("records".into(),serde_json::to_string(&records).map_err(|_|error("invalid_distribution_record"))?);plan.metadata.insert("createdAt".into(),self.clock.now().to_string());
        let mut warnings=vec!["可能读取者仅涵盖已登记档案；复制成功不代表 Agent 已加载。".into()];
        if !selected.is_empty(){for history in self.list_history()?{for old in history.records{if let Some(current)=loaded.catalog.records.iter().find(|r|r.id==old.package_id){if current.name!=old.name_at_write && old.physical_path.exists(){warnings.push(format!("历史名称位置仍存在：{}",skill_core::compatibility::safe_text(&old.physical_path.to_string_lossy())));}}}}}
        roots.sort();roots.dedup();let mut engine=self.engine(roots,vec![],Some(&loaded.path))?;engine.scan_roots=self.known_roots(&ctx)?;let backup_location=skill_core::compatibility::safe_text(&engine.backup_root.to_string_lossy());let groups=skill_core::diff::redacted_plan_dto(&core).groups;
        if stamp!=self.snapshot(Some(&over))? || targets.iter().any(|t|skill_io::paths::physical_key(&t.path).is_ok_and(|k|k!=t.key.0)){return Err(error("preview_changed"));}
        over.extra_paths=targets.iter().map(|t|t.path.clone()).collect();over.extra_paths.push(req.library_path.clone());let bound_stamp=self.snapshot(Some(&over))?;
        let (preview_id,expires_at)=self.install_bound("distribution",plan,loaded.bindings,engine,outcomes,blocked,bound_stamp,Some(over),auxiliary)?;
        Ok(PreviewDto{preview_id,expires_at,groups,warnings,blocked,backup_location})
    }
    pub fn commit_distribution(&self,id:&str)->Result<ReceiptDto,AppError>{self.commit(id,"distribution")}
}
