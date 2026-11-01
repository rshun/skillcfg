use crate::{IoError,MutationReceipt};
use crate::disk::{err,resolved_path};
use std::path::{Path,PathBuf};
#[derive(Clone,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub(crate) struct Journal {
    pub version:u32,pub id:String,pub kind:String,pub state:String,pub items:Vec<Item>,pub receipt:Option<MutationReceipt>,pub metadata:std::collections::BTreeMap<String,String>,
}
#[derive(Clone,serde::Serialize,serde::Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub(crate) struct Item {
    pub path:PathBuf,pub directory:bool,pub before:bool,pub after:bool,pub state:String,pub work:PathBuf,
    pub baseline_key:Option<String>,pub activated_key:Option<String>,pub anchor:PathBuf,pub anchor_key:String,
}
pub(crate) fn anchor(path:&Path)->Result<(PathBuf,String),IoError>{let mut p=path.parent().ok_or(err("invalid_path"))?.to_owned();while !p.exists(){if !p.pop(){return Err(err("invalid_path"));}}let p=resolved_path(&p)?;let key=crate::paths::physical_key(&p)?;Ok((p,key))}
pub(crate) fn anchor_valid(item:&Item)->bool{crate::paths::physical_key(&item.anchor).is_ok_and(|k|k==item.anchor_key) && resolved_path(&item.path).is_ok_and(|p|p==item.path)}
