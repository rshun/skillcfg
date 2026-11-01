use crate::plan::{Disposition,DistributionPlan};

#[derive(Debug)]
pub struct SafeFileDiff {pub path:String,pub binary:bool,pub before:Option<String>,pub after:Option<String>,pub before_bytes:usize,pub after_bytes:usize}
#[derive(Debug)]
pub struct SafePlanGroup {pub path:String,pub disposition:Disposition,pub selected_agents:Vec<String>,pub possible_readers:Vec<String>,pub files:Vec<SafeFileDiff>,pub reason:Option<&'static str>}
#[derive(Debug,Default)]
pub struct SafePlanView {pub groups:Vec<SafePlanGroup>}
pub fn redacted_plan_dto(_plan:&DistributionPlan)->SafePlanView{SafePlanView::default()}
