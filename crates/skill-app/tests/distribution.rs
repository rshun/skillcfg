mod service_support;
use service_support::*;
use skill_app::*;
use std::{fs,sync::{Arc,atomic::{AtomicU64,Ordering}}};
struct FakeClock(AtomicU64);
impl Clock for FakeClock{fn now(&self)->u64{self.0.load(Ordering::SeqCst)}}
#[test]
fn expired_preview_writes_nothing_and_is_consumed(){let (base,source,library,target)=ready();let clock=Arc::new(FakeClock(AtomicU64::new(100)));let app=App::with_clock(base.context,clock.clone());let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();clock.0.store(700,Ordering::SeqCst);assert!(app.commit_distribution(&p.preview_id).is_err());clock.0.store(101,Ordering::SeqCst);assert!(app.commit_distribution(&p.preview_id).is_err());assert!(!target.exists());}
#[test]
fn added_target_file_invalidates_whole_preview_and_consumes_it(){let (app,source,library,target)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();fs::create_dir_all(&target).unwrap();fs::write(target.join("external"),b"later").unwrap();assert!(app.commit_distribution(&p.preview_id).is_err());assert!(app.commit_distribution(&p.preview_id).is_err());assert_eq!(fs::read(target.join("external")).unwrap(),b"later");assert!(!target.join("SKILL.md").exists());}
#[test]
fn settings_change_and_new_custom_profile_invalidate_distribution_preview(){for setting in [true,false]{let (app,source,library,target)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();if setting{let mut v=app.load_settings().unwrap();v.settings.backup_path=app.context.env.home.join("other-backups");app.save_settings(SettingsRequest{expected_revision:v.revision,settings:v.settings}).unwrap();}else{let dir=app.context.env.home.join(".skill-hub/agents");fs::create_dir_all(&dir).unwrap();fs::write(dir.join("new.json"),bytes(&profile("custom.new"))).unwrap();}assert!(app.commit_distribution(&p.preview_id).is_err());assert!(!target.exists());}}
#[test]
fn identical_target_and_cancelled_preview_have_no_write(){let (app,source,library,target)=ready();let id=import(&app,&source,&library);let p=app.preview_distribution(request(&id,&library)).unwrap();app.cancel_preview(&p.preview_id);assert!(app.commit_distribution(&p.preview_id).is_err());assert!(!target.exists());let p=app.preview_distribution(request(&id,&library)).unwrap();app.commit_distribution(&p.preview_id).unwrap();let count=app.list_history().unwrap().len();let p=app.preview_distribution(request(&id,&library)).unwrap();let receipt=app.commit_distribution(&p.preview_id).unwrap();assert!(receipt.outcomes.iter().all(|i|i.outcome==skill_io::Outcome::Unchanged));assert_eq!(app.list_history().unwrap().len(),count);}
