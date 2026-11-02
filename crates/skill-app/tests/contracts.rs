mod service_support;
use service_support::*;
use skill_app::*;
use serde_json::Value;
fn shape(actual:&Value,fixture:&Value){match (actual,fixture){(Value::Object(a),Value::Object(b))=>{assert_eq!(a.keys().collect::<Vec<_>>(),b.keys().collect::<Vec<_>>());for(k,v)in b{shape(&a[k],v);}},(Value::Array(a),Value::Array(b))=>{if let(Some(a),Some(b))=(a.first(),b.first()){shape(a,b);}},(Value::Null,_)|(_,Value::Null)=>{},(a,b)=>assert_eq!(std::mem::discriminant(a),std::mem::discriminant(b))}}
#[test]
fn public_contract_fixtures_match_real_service_responses(){
 let(app,source,library,_)=ready();let scan=app.scan_import(ImportRequest{source:source.clone(),library_path:library.clone()}).unwrap();shape(&serde_json::to_value(&scan).unwrap(),&serde_json::from_str(include_str!("../../../tests/fixtures/contracts/import.json")).unwrap());
 let p=app.preview_agent_profile(ProfileChangeRequest{profile_json:bytes(&profile("custom.demo")),expected_revision:None,local_root_overrides:vec![],enable_after_save:true}).unwrap();shape(&serde_json::to_value(p).unwrap(),&serde_json::from_str(include_str!("../../../tests/fixtures/contracts/profiles.json")).unwrap());
 let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();shape(&serde_json::to_value(&p).unwrap(),&serde_json::from_str(include_str!("../../../tests/fixtures/contracts/distribution.json")).unwrap());let receipt=app.commit_distribution(&p.preview_id).unwrap();shape(&serde_json::to_value(receipt).unwrap(),&serde_json::from_str(include_str!("../../../tests/fixtures/contracts/receipt.json")).unwrap());
}
