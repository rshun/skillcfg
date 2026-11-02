#![allow(dead_code)]
use skill_core::{EntryKind, PackageTree, TreeEntry};
use skill_io::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
pub fn document(body: &str) -> Vec<u8> {
    format!("---\nname: review\ndescription: Fixture.\n---\n{body}").into_bytes()
}
pub fn engine_at(root: &Path) -> IoEngine {
    IoEngine {
        backup_root: root.join("hub/backups"),
        audit_path: root.join("hub/audit/events.jsonl"),
        write_roots: vec![root.join("agent/skills")],
        write_files: vec![],
        write_file_roots: vec![],
        scan_roots: vec![root.join("agent/skills")],
        protected_roots: vec![root.join("hub")],
        observer: Arc::new(Quiet),
    }
}
pub fn tree(body: &str) -> PackageTree {
    PackageTree {
        entries: vec![TreeEntry {
            relative_path: "SKILL.md".into(),
            kind: EntryKind::File {
                bytes: document(body),
                unix_executable: false,
            },
        }],
    }
}
pub fn fixture() -> (IoEngine, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "skill-tx-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = fs::canonicalize(root).unwrap();
    fs::create_dir_all(root.join("hub")).unwrap();
    let target = root.join("agent/skills/review");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), document("old")).unwrap();
    (
        IoEngine {
            backup_root: root.join("hub/backups"),
            audit_path: root.join("hub/audit/events.jsonl"),
            write_roots: vec![root.join("agent/skills")],
            write_files: vec![],
            write_file_roots: vec![],
            scan_roots: vec![root.join("agent/skills")],
            protected_roots: vec![root.join("hub")],
            observer: Arc::new(Quiet),
        },
        target,
    )
}
pub fn plan(path: &Path, before: Option<PackageTree>, after: Option<PackageTree>) -> MutationPlan {
    MutationPlan {
        directories: vec![DirectoryChange {
            path: path.to_owned(),
            before,
            after,
        }],
        files: vec![],
        kind: "distribution".into(),
        ..Default::default()
    }
}
