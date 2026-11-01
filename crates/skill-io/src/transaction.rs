use crate::IoError;
use skill_core::PackageTree;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct DirectoryChange { pub path: PathBuf, pub before: Option<PackageTree>, pub after: Option<PackageTree> }
#[derive(Clone)]
pub struct FileChange { pub path: PathBuf, pub before: Option<Vec<u8>>, pub after: Option<Vec<u8>> }
#[derive(Clone, Default)]
pub struct MutationPlan { pub directories: Vec<DirectoryChange>, pub files: Vec<FileChange>, pub kind: String }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Outcome { Written, Unchanged, Skipped, Blocked, FailedRestored, RecoveryRequired, AuditFailed }
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct ItemReceipt { pub path: PathBuf, pub outcome: Outcome, pub reason: Option<String> }
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct MutationReceipt { pub operation_id: String, pub backup_location: PathBuf, pub outcomes: Vec<ItemReceipt> }
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct RecoveryRecord { pub operation_id: String, pub kind: String, pub state: String, pub paths: Vec<PathBuf>, pub reason: Option<String> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step { BackedUp, Archived, Activated, BeforeVerify, Verified, Audit }
pub trait Observer: Send + Sync { fn event(&self, _step: Step, _path: &std::path::Path) -> Result<(), IoError> { Ok(()) } }
pub struct Quiet;
impl Observer for Quiet {}
#[derive(Clone)]
pub struct IoEngine { pub backup_root: PathBuf, pub audit_path: PathBuf, pub write_roots: Vec<PathBuf>, pub write_files: Vec<PathBuf>, pub scan_roots: Vec<PathBuf>, pub protected_roots: Vec<PathBuf>, pub observer: Arc<dyn Observer> }
pub struct PreparedMutation { _private: () }
impl IoEngine {
    pub fn prepare(&self, _plan:&MutationPlan)->Result<PreparedMutation,IoError>{Err(IoError{code:"not_implemented"})}
    pub fn apply(&self, _prepared:PreparedMutation)->MutationReceipt{MutationReceipt{operation_id:String::new(),backup_location:PathBuf::new(),outcomes:vec![]}}
    pub fn list_history(&self)->Result<Vec<MutationReceipt>,IoError>{Err(IoError{code:"not_implemented"})}
    pub fn list_recoveries(&self)->Result<Vec<RecoveryRecord>,IoError>{Err(IoError{code:"not_implemented"})}
    pub fn plan_restore(&self,_id:&str)->Result<MutationPlan,IoError>{Err(IoError{code:"not_implemented"})}
}
