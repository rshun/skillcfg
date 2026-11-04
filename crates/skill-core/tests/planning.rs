use skill_core::{diff::redacted_plan_dto, plan::*, *};
use std::{collections::BTreeMap, path::PathBuf};
fn package(id: &str, name: &str, body: &str) -> LibraryPackage {
    let text = format!("---\nname: {name}\ndescription: Fixture.\n---\n{body}");
    LibraryPackage {
        id: PackageId(id.into()),
        metadata: SkillMetadata {
            name: name.into(),
            description: "Fixture.".into(),
        },
        tree: PackageTree {
            entries: vec![TreeEntry {
                relative_path: "SKILL.md".into(),
                kind: EntryKind::File {
                    bytes: text.into_bytes(),
                    unix_executable: false,
                },
            }],
        },
    }
}
fn target(
    p: &LibraryPackage,
    agent: &str,
    key: &str,
    before: Option<PackageTree>,
) -> TargetSnapshot {
    TargetSnapshot {
        package_id: p.id.clone(),
        agent: AgentId::parse(agent).unwrap(),
        key: TargetKey(key.into()),
        path: PathBuf::from(format!("fixture/{key}/{}", p.metadata.name)),
        platform: Platform::Linux,
        tree: before,
        possible_readers: vec![],
        blocked: None,
    }
}
fn pair(p: &LibraryPackage, agent: &str) -> SelectionPair {
    SelectionPair {
        package_id: p.id.clone(),
        agent: AgentId::parse(agent).unwrap(),
    }
}
#[test]
fn two_pairs_do_not_expand_to_four() {
    let a = package("a", "review", "one");
    let b = package("b", "debug", "two");
    let targets = vec![
        target(&a, "codex", "a", None),
        target(&b, "custom.fifth", "b", None),
    ];
    let result = plan_distribution(
        &[a.clone(), b.clone()],
        &targets,
        &[pair(&a, "codex"), pair(&b, "custom.fifth")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups.len(), 2);
    assert_eq!(result.groups[0].selected_agents.len(), 1);
    assert_eq!(result.groups[1].selected_agents.len(), 1);
}
#[test]
fn shared_target_has_one_change_and_all_possible_readers() {
    let p = package("p", "review", "new");
    let mut a = target(&p, "codex", "shared", None);
    let mut b = target(&p, "custom.fifth", "shared", None);
    let reader = ReaderExposure {
        agent: AgentId::parse("grok").unwrap(),
        variant: "cli".into(),
        root_id: "compat".into(),
        evidence: ReadState::Unknown,
    };
    a.possible_readers.push(reader.clone());
    b.possible_readers.push(reader);
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &[a, b],
        &[pair(&p, "codex"), pair(&p, "custom.fifth")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.groups[0].selected_agents.len(), 2);
    assert_eq!(result.groups[0].possible_readers.len(), 1);
    assert_eq!(result.groups[0].disposition, Disposition::New);
}
#[test]
fn two_versions_for_same_target_block() {
    let a = package("a", "review", "old");
    let b = package("b", "review", "different");
    let targets = [
        target(&a, "codex", "shared", None),
        target(&b, "grok", "shared", None),
    ];
    let result = plan_distribution(
        &[a.clone(), b.clone()],
        &targets,
        &[pair(&a, "codex"), pair(&b, "grok")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups[0].disposition, Disposition::Blocked);
}
#[test]
fn unmanaged_difference_defaults_to_skip_and_replace_lists_extra_old_files() {
    let p = package("p", "review", "new");
    let mut old = package("old", "review", "old").tree;
    old.entries.push(TreeEntry {
        relative_path: "user-notes.txt".into(),
        kind: EntryKind::File {
            bytes: b"preserve in archive".to_vec(),
            unix_executable: false,
        },
    });
    let targets = [target(&p, "codex", "one", Some(old))];
    let selection = [pair(&p, "codex")];
    let skipped = plan_distribution(
        std::slice::from_ref(&p),
        &targets,
        &selection,
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(skipped.groups[0].disposition, Disposition::Skipped);
    let decisions = BTreeMap::from([(TargetKey("one".into()), ConflictDecision::Replace)]);
    let replace = plan_distribution(&[p], &targets, &selection, &decisions).unwrap();
    assert_eq!(replace.groups[0].disposition, Disposition::Update);
    assert_eq!(replace.groups[0].comparison.removed, vec!["user-notes.txt"]);
}
#[test]
fn identical_and_empty_selection_have_no_writes() {
    let p = package("p", "review", "same");
    let targets = [target(&p, "codex", "one", Some(p.tree.clone()))];
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &targets,
        &[pair(&p, "codex")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups[0].disposition, Disposition::Unchanged);
    assert!(plan_distribution(&[], &[], &[], &BTreeMap::new())
        .unwrap()
        .groups
        .is_empty());
}
#[test]
fn stale_shared_baselines_and_link_targets_block() {
    let p = package("p", "review", "new");
    let a = target(&p, "codex", "shared", None);
    let b = target(&p, "grok", "shared", Some(p.tree.clone()));
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &[a, b],
        &[pair(&p, "codex"), pair(&p, "grok")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups[0].disposition, Disposition::Blocked);
    let mut linked = target(&p, "codex", "link", None);
    linked.blocked = Some("target_link");
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &[linked],
        &[pair(&p, "codex")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups[0].reason, Some("target_link"));
}
#[test]
fn unsafe_assignment_is_blocked_and_public_diff_does_not_reveal_it() {
    let p = package("p", "review", "password: 'SYNTHETIC_PRIVATE_MARKER'");
    let targets = [target(&p, "codex", "one", None)];
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &targets,
        &[pair(&p, "codex")],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.groups[0].disposition, Disposition::Blocked);
    assert!(!format!("{:?}", redacted_plan_dto(&result)).contains("SYNTHETIC_PRIVATE_MARKER"));
}
#[test]
fn binary_changes_are_summaries_and_metadata_cache_mismatch_is_rejected() {
    let mut p = package("p", "review", "safe");
    p.tree.entries.push(TreeEntry {
        relative_path: "image.bin".into(),
        kind: EntryKind::File {
            bytes: vec![0, 255],
            unix_executable: false,
        },
    });
    let targets = [target(&p, "codex", "one", None)];
    let result = plan_distribution(
        std::slice::from_ref(&p),
        &targets,
        &[pair(&p, "codex")],
        &BTreeMap::new(),
    )
    .unwrap();
    let view = redacted_plan_dto(&result);
    assert!(view.groups[0]
        .files
        .iter()
        .any(|f| f.path == "image.bin" && f.binary && f.after.is_none()));
    p.metadata.name = "different".into();
    assert!(plan_distribution(
        std::slice::from_ref(&p),
        &targets,
        &[pair(&p, "codex")],
        &BTreeMap::new()
    )
    .is_err());
}

#[test]
fn structured_sensitive_fields_are_blocked_in_source_and_existing_target() {
    for content in [
        r#"{"token":"SYNTHETIC_PRIVATE_MARKER"}"#,
        r#"{"ordinary":true,"nested":{"password":"SYNTHETIC_PRIVATE_MARKER"}}"#,
        r#"{"headers":{"Authorization":"Bearer SYNTHETIC_PRIVATE_MARKER"}}"#,
        "nested:\n  token: SYNTHETIC_PRIVATE_MARKER",
    ] {
        let mut private = package("private", "review", "safe");
        private.tree.entries.push(TreeEntry {
            relative_path: "settings.json".into(),
            kind: EntryKind::File {
                bytes: content.as_bytes().to_vec(),
                unix_executable: false,
            },
        });
        let clean = package("clean", "review", "safe");
        for (source, before) in [(&private, None), (&clean, Some(private.tree.clone()))] {
            let result = plan_distribution(
                std::slice::from_ref(source),
                &[target(source, "codex", "one", before)],
                &[pair(source, "codex")],
                &BTreeMap::new(),
            )
            .unwrap();
            assert_eq!(result.groups[0].disposition, Disposition::Blocked);
            assert!(
                !format!("{:?}", redacted_plan_dto(&result)).contains("SYNTHETIC_PRIVATE_MARKER")
            );
        }
    }
}

#[test]
fn safe_diff_distinguishes_binary_removal_directory_and_permission_changes() {
    let mut p = package("p", "review", "safe");
    p.tree.entries.push(TreeEntry { relative_path: "run.sh".into(), kind: EntryKind::File { bytes: b"same".to_vec(), unix_executable: true } });
    p.tree.entries.push(TreeEntry { relative_path: "new-empty".into(), kind: EntryKind::Directory });
    let mut before = p.tree.clone();
    before.entries.retain(|e| e.relative_path != "new-empty");
    before.entries.iter_mut().find(|e| e.relative_path == "run.sh").unwrap().kind = EntryKind::File { bytes: b"same".to_vec(), unix_executable: false };
    before.entries.push(TreeEntry { relative_path: "old-image.bin".into(), kind: EntryKind::File { bytes: vec![0,255], unix_executable: false } });
    before.entries.push(TreeEntry { relative_path: "old-empty".into(), kind: EntryKind::Directory });
    let key = TargetKey("one".into());
    let plan = plan_distribution(std::slice::from_ref(&p), &[target(&p,"codex","one",Some(before))], &[pair(&p,"codex")], &BTreeMap::from([(key,ConflictDecision::Replace)])).unwrap();
    let dto = serde_json::to_value(redacted_plan_dto(&plan)).unwrap();
    let files = dto["groups"][0]["files"].as_array().unwrap();
    let item = |path: &str| files.iter().find(|f| f["path"] == path).unwrap();
    assert_eq!(item("old-image.bin")["change"], "removed");
    assert_eq!(item("old-empty")["beforeKind"], "directory");
    assert_eq!(item("new-empty")["change"], "added");
    assert_eq!(item("run.sh")["change"], "permissions");
    assert_eq!(item("run.sh")["afterExecutable"], true);
}
