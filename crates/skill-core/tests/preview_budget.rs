use skill_core::{plan::*, *};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::BTreeMap,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct CountingAllocator;
static COUNT: AtomicBool = AtomicBool::new(false);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT.load(Ordering::Relaxed) {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, new: usize) -> *mut u8 {
        if COUNT.load(Ordering::Relaxed) {
            ALLOCATED.fetch_add(new, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, old, new) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn over_budget_preview_is_rejected_before_copying_source_bytes() {
    let mut packages = Vec::new();
    let mut targets = Vec::new();
    let mut selection = Vec::new();
    for index in 0..5 {
        let id = PackageId(format!("p{index}"));
        let agent = AgentId::parse("codex").unwrap();
        let mut entries = vec![TreeEntry {
            relative_path: "SKILL.md".into(),
            kind: EntryKind::File {
                bytes: b"---\nname: review\ndescription: Fixture.\n---\n".to_vec(),
                unix_executable: false,
            },
        }];
        for file in 0..4 {
            entries.push(TreeEntry {
                relative_path: format!("asset{file}.bin"),
                kind: EntryKind::File {
                    bytes: vec![0xff; 4 * 1024 * 1024 - 1024],
                    unix_executable: false,
                },
            });
        }
        packages.push(LibraryPackage {
            id: id.clone(),
            metadata: SkillMetadata {
                name: "review".into(),
                description: "Fixture.".into(),
            },
            tree: PackageTree { entries },
        });
        targets.push(TargetSnapshot {
            package_id: id.clone(),
            agent: agent.clone(),
            key: TargetKey(format!("t{index}")),
            path: format!("target/{index}").into(),
            platform: Platform::Linux,
            tree: None,
            possible_readers: vec![],
            blocked: None,
        });
        selection.push(SelectionPair {
            package_id: id,
            agent,
        });
    }
    COUNT.store(true, Ordering::SeqCst);
    let result = plan_distribution(&packages, &targets, &selection, &BTreeMap::new());
    COUNT.store(false, Ordering::SeqCst);
    assert_eq!(result.err().unwrap().code, "preview_too_large");
    assert!(
        ALLOCATED.load(Ordering::Relaxed) < 1024 * 1024,
        "over-budget requests must not copy package data"
    );
}
