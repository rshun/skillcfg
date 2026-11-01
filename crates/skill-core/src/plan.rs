use crate::*;
use std::{collections::BTreeMap,path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetKey(pub String);
#[derive(Clone)]
pub struct LibraryPackage { pub id:PackageId,pub metadata:SkillMetadata,pub tree:PackageTree }
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord)]
pub struct SelectionPair { pub package_id:PackageId,pub agent:AgentId }
#[derive(Clone)]
pub struct TargetSnapshot {
    pub package_id:PackageId,pub agent:AgentId,pub key:TargetKey,pub path:PathBuf,
    pub platform:Platform,pub tree:Option<PackageTree>,pub possible_readers:Vec<ReaderExposure>,pub blocked:Option<&'static str>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ConflictDecision { Skip,Replace }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Disposition { New,Update,Unchanged,Skipped,Blocked }
#[derive(Clone)]
pub struct PlanGroup {
    pub key:TargetKey,pub path:PathBuf,pub package_ids:Vec<PackageId>,pub selected_agents:Vec<AgentId>,
    pub possible_readers:Vec<ReaderExposure>,pub before:Option<PackageTree>,pub after:PackageTree,
    pub disposition:Disposition,pub reason:Option<&'static str>,pub comparison:PackageComparison,
}
#[derive(Clone,Default)]
pub struct DistributionPlan {pub groups:Vec<PlanGroup>}
pub fn plan_distribution(_packages:&[LibraryPackage],_targets:&[TargetSnapshot],_selection:&[SelectionPair],_decisions:&BTreeMap<TargetKey,ConflictDecision>)->Result<DistributionPlan,CoreError>{Err(CoreError::new("not_implemented"))}
