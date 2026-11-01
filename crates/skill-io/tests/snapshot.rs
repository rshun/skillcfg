use skill_io::{paths, read_package_snapshot, ScanLimits};
use skill_core::EntryKind;
use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

fn temp() -> PathBuf {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!("skillhub-snapshot-{}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),SERIAL.fetch_add(1,Ordering::Relaxed)));
    fs::create_dir_all(&path).unwrap(); path
}
fn package() -> PathBuf {
    let root = temp().join("技能包"); fs::create_dir_all(root.join("empty")).unwrap();
    fs::write(root.join("SKILL.md"), b"---\nname: review\ndescription: Review.\n---\n").unwrap();
    fs::write(root.join("image.bin"), [0,255,42]).unwrap(); root
}
fn alias(target: &std::path::Path, link: &std::path::Path) {
    #[cfg(unix)] std::os::unix::fs::symlink(target,link).unwrap();
    #[cfg(windows)] {
        let result = std::process::Command::new("cmd").args(["/C","mklink","/J"]).arg(link).arg(target).output().unwrap();
        assert!(result.status.success(),"fixture junction creation failed");
    }
}
#[test]
fn reads_binary_unicode_and_empty_directory_without_mutation() {
    let root = package(); let snapshot = read_package_snapshot(&root,&ScanLimits::default()).unwrap();
    assert!(snapshot.tree.entries.iter().any(|e| e.relative_path=="empty" && matches!(e.kind,EntryKind::Directory)));
    assert!(snapshot.tree.entries.iter().any(|e| e.relative_path=="image.bin" && matches!(&e.kind,EntryKind::File{bytes,..} if bytes==&[0,255,42])));
    assert_eq!(fs::read(root.join("image.bin")).unwrap(),vec![0,255,42]);
}
#[test]
fn root_alias_is_same_physical_package_but_remains_read_only() {
    let root = package(); let link = temp().join("alias"); alias(&root,&link);
    let a = read_package_snapshot(&root,&ScanLimits::default()).unwrap();
    let b = read_package_snapshot(&link,&ScanLimits::default()).unwrap();
    assert_eq!(a.physical_key,b.physical_key); assert!(b.root_was_link);
    assert!(paths::is_link(&link).unwrap());
}
#[test]
fn internal_link_is_blocked() {
    let root = package(); let other = temp(); alias(&other,&root.join("outside"));
    let error = read_package_snapshot(&root,&ScanLimits::default()).err().unwrap();
    assert_eq!(error.code,"internal_link");
}
#[test]
fn oversized_or_deep_inputs_fail_instead_of_truncating() {
    let root = package();
    let limits = ScanLimits { file_bytes: 2, ..ScanLimits::default() };
    assert_eq!(read_package_snapshot(&root,&limits).err().unwrap().code,"file_too_large");
    let deep = root.join("a/b/c");fs::create_dir_all(&deep).unwrap();fs::write(deep.join("f"),b"x").unwrap();
    let limits = ScanLimits { depth: 2, ..ScanLimits::default() };
    assert_eq!(read_package_snapshot(&root,&limits).err().unwrap().code,"too_deep");
}
#[cfg(unix)]
#[test]
fn retains_unix_executable_bit() {
    use std::os::unix::fs::PermissionsExt;
    let root = package();fs::write(root.join("run.sh"),b"echo fixture").unwrap();
    fs::set_permissions(root.join("run.sh"),fs::Permissions::from_mode(0o700)).unwrap();
    let result = read_package_snapshot(&root,&ScanLimits::default()).unwrap();
    assert!(result.tree.entries.iter().any(|e|e.relative_path=="run.sh" && matches!(e.kind,EntryKind::File{unix_executable:true,..})));
}
