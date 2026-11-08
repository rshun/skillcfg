use serde_json::{json, Value};
use skill_desktop::bridge::DesktopSession;
use std::fs;
use std::path::PathBuf;

fn isolated_workspace() -> (DesktopSession, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(skill_io::disk::operation_id());
    let home = root.join("home");
    let project = root.join("project-中文验证");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(project.join(".git")).unwrap();
    let home = fs::canonicalize(home).unwrap();
    let project = fs::canonicalize(project).unwrap();
    (DesktopSession::new(home.clone()).unwrap(), home, project)
}

#[cfg(unix)]
fn mark_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    (fs::metadata(path).unwrap().permissions().mode() & 0o111) != 0
}

#[test]
fn claude_import_exact_pairs_shared_visibility_edit_and_restore_end_to_end() {
    let (mut session, home, project) = isolated_workspace();

    // Seed two real Claude Code Skill packages with UTF-8 Chinese, unknown frontmatter,
    // CRLF line endings, binary assets, nested directories, and Unix execute bits.
    let claude_root = home.join(".claude/skills");
    let review_src = claude_root.join("code-review");
    let incident_src = claude_root.join("incident-response");
    fs::create_dir_all(review_src.join("assets")).unwrap();
    fs::create_dir_all(review_src.join("scripts")).unwrap();
    fs::create_dir_all(incident_src.join("references")).unwrap();

    let review_doc = b"---\r\nname: code-review\r\ndescription: \xe5\xae\xa1\xe9\x98\x85\xe4\xbb\xa3\xe7\xa0\x81\xe5\xb9\xb6\xe4\xbf\x9d\xe7\x95\x99 CRLF\r\nx-owner: qa-team\r\nallowed-tools: Read\r\n---\r\n\xe9\x80\x90\xe8\xa1\x8c\xe6\xa0\xb8\xe5\xaf\xb9\xe5\x8f\x98\xe6\x9b\xb4\xe3\x80\x82\r\n";
    let review_bin = [0x00_u8, 0xff, 0x89, 0x50, 0x4e, 0x47];
    let review_script = b"#!/bin/sh\necho review-ok\n";
    fs::write(review_src.join("SKILL.md"), review_doc).unwrap();
    fs::write(review_src.join("assets/diagram.bin"), review_bin).unwrap();
    fs::write(review_src.join("scripts/check.sh"), review_script).unwrap();
    #[cfg(unix)]
    mark_executable(&review_src.join("scripts/check.sh"));

    let incident_doc = "---\nname: incident-response\ndescription: 故障现场保留与排查指南\n---\n先保留现场，再逐步核对。\n";
    let incident_ref = "# 应急手册\n不要删除备份或锁记录。\n";
    fs::write(incident_src.join("SKILL.md"), incident_doc).unwrap();
    fs::write(incident_src.join("references/runbook.md"), incident_ref).unwrap();

    // Seed an independent Antigravity user package that is never selected.
    let antigravity_untouched = home.join(".gemini/config/skills/untouched-skill");
    fs::create_dir_all(&antigravity_untouched).unwrap();
    let untouched_bytes = b"---\nname: untouched-skill\ndescription: Independent skill.\n---\nMust stay byte-identical.\n";
    fs::write(antigravity_untouched.join("SKILL.md"), untouched_bytes).unwrap();

    // Startup / read-only discovery creates no .skill-hub state.
    let discovered = session
        .execute("list_agent_skills", json!({ "agentId": "claude-code" }))
        .unwrap();
    assert_eq!(discovered["items"].as_array().unwrap().len(), 2);
    assert!(!home.join(".skill-hub").exists());

    // 1. Scan & preview import from ~/.claude/skills -> commit to Hub library.
    let scan = session
        .execute("scan_import", json!({ "source": claude_root }))
        .unwrap();
    let items = scan["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let choices: Vec<Value> = items
        .iter()
        .map(|item| {
            json!({
                "itemId": item["itemId"],
                "action": "new",
                "replaceId": null
            })
        })
        .collect();
    let import_preview = session
        .execute(
            "preview_import",
            json!({ "scanId": scan["scanId"], "choices": choices }),
        )
        .unwrap();
    assert!(!import_preview["blocked"].as_bool().unwrap());
    assert!(!home.join(".skill-hub/library/catalog.json").exists());

    session
        .execute(
            "commit_import",
            json!({ "previewId": import_preview["previewId"] }),
        )
        .unwrap();

    // Source Claude directory, unselected Antigravity directory, and Codex/Grok targets stay untouched.
    assert_eq!(fs::read(review_src.join("SKILL.md")).unwrap(), review_doc);
    assert_eq!(
        fs::read(antigravity_untouched.join("SKILL.md")).unwrap(),
        untouched_bytes
    );
    assert!(!home.join(".agents/skills").exists());
    assert!(!home.join(".grok/skills").exists());

    // 2. Load library and construct exact non-Cartesian pairs:
    //    code-review -> codex, incident-response -> grok.
    let library = session.execute("load_library", json!({})).unwrap();
    let records = library["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let review_id = records.iter().find(|r| r["name"] == "code-review").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let incident_id = records
        .iter()
        .find(|r| r["name"] == "incident-response")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Verify project-scope shared root visibility first (.agents/skills is shared by codex, antigravity, grok).
    session
        .execute(
            "set_context",
            json!({ "scope": "project", "projectRoot": project }),
        )
        .unwrap();
    let project_preview = session
        .execute(
            "preview_distribution",
            json!({
                "selection": [{ "packageId": review_id, "agent": "codex" }],
                "conflictDecisions": {}
            }),
        )
        .unwrap();
    let project_readers = project_preview["groups"][0]["possibleReaders"]
        .as_array()
        .unwrap();
    assert!(project_readers.iter().any(|r| r["agent"] == "antigravity"));
    assert!(project_readers.iter().any(|r| r["agent"] == "grok"));
    assert!(!project.join(".agents/skills").exists());

    // Switch back to user scope and preview exact pairs: (review -> codex) + (incident -> grok).
    session
        .execute(
            "set_context",
            json!({ "scope": "user", "projectRoot": null }),
        )
        .unwrap();
    let dist_preview = session
        .execute(
            "preview_distribution",
            json!({
                "selection": [
                    { "packageId": review_id, "agent": "codex" },
                    { "packageId": incident_id, "agent": "grok" }
                ],
                "conflictDecisions": {}
            }),
        )
        .unwrap();
    assert!(!dist_preview["blocked"].as_bool().unwrap());
    let groups = dist_preview["groups"].as_array().unwrap();
    // Exact pairs: 2 physical target groups, NOT 4 Cartesian groups.
    assert_eq!(groups.len(), 2);

    let codex_group = groups
        .iter()
        .find(|g| g["selectedAgents"] == json!(["codex"]))
        .unwrap();
    assert!(codex_group["path"]
        .as_str()
        .unwrap()
        .replace('\\', "/")
        .ends_with("/.agents/skills/code-review"));
    let codex_readers = codex_group["possibleReaders"].as_array().unwrap();
    assert!(codex_readers.iter().any(|r| r["agent"] == "grok"
        && r["rootId"] == "user-shared"
        && r["evidence"] == "declared"));
    assert!(!codex_readers.iter().any(|r| r["agent"] == "antigravity"));

    let grok_group = groups
        .iter()
        .find(|g| g["selectedAgents"] == json!(["grok"]))
        .unwrap();
    assert!(grok_group["path"]
        .as_str()
        .unwrap()
        .replace('\\', "/")
        .ends_with("/.grok/skills/incident-response"));
    let grok_readers = grok_group["possibleReaders"].as_array().unwrap();
    assert!(!grok_readers.is_empty());
    assert!(grok_readers.iter().all(|r| r["agent"] == "grok"));

    // 3. Apply manual distribution and verify written bytes, CRLF, binary, execute bit, and post-write statuses.
    let dist_receipt = session
        .execute(
            "commit_distribution",
            json!({ "previewId": dist_preview["previewId"] }),
        )
        .unwrap();
    let dist_op_id = dist_receipt["operationId"].as_str().unwrap().to_owned();
    assert_eq!(dist_receipt["outcomes"].as_array().unwrap().len(), 2);

    let codex_written = home.join(".agents/skills/code-review");
    let grok_written = home.join(".grok/skills/incident-response");
    assert_eq!(
        fs::read(codex_written.join("SKILL.md")).unwrap(),
        review_doc
    );
    assert_eq!(
        fs::read(codex_written.join("assets/diagram.bin")).unwrap(),
        review_bin
    );
    #[cfg(unix)]
    assert!(is_executable(&codex_written.join("scripts/check.sh")));
    assert_eq!(
        fs::read_to_string(grok_written.join("references/runbook.md")).unwrap(),
        incident_ref
    );
    assert!(!home.join(".agents/skills/incident-response").exists());
    assert!(!home.join(".grok/skills/code-review").exists());
    assert_eq!(
        fs::read(antigravity_untouched.join("SKILL.md")).unwrap(),
        untouched_bytes
    );

    let statuses_after_write = session.execute("library_statuses", json!({})).unwrap();
    let status_of = |items: &Value, pkg: &str, agent: &str| -> String {
        items
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["packageId"] == pkg && s["agentId"] == agent)
            .unwrap()["state"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        status_of(&statuses_after_write, &review_id, "codex"),
        "equal"
    );
    assert_eq!(
        status_of(&statuses_after_write, &incident_id, "grok"),
        "equal"
    );
    assert_eq!(
        status_of(&statuses_after_write, &review_id, "grok"),
        "missing"
    );
    assert_eq!(
        status_of(&statuses_after_write, &incident_id, "codex"),
        "missing"
    );
    assert_eq!(
        status_of(&statuses_after_write, &review_id, "antigravity"),
        "missing"
    );

    // 4. Edit library documents WITHOUT applying:
    //    - Modify code-review body -> codex target stays untouched on disk, status becomes "different".
    //    - Rename incident-response in library -> restore still targets original physical path!
    let pkg_view = session
        .execute("read_package", json!({ "packageId": review_id }))
        .unwrap();
    session
        .execute(
            "save_skill_document",
            json!({
                "packageId": review_id,
                "editToken": pkg_view["editToken"],
                "document": "---\r\nname: code-review\r\ndescription: 审阅代码（库内已更新但未分发）\r\nx-owner: qa-team\r\n---\r\n仅保存在配置库的新版本。\r\n"
            }),
        )
        .unwrap();
    let incident_view = session
        .execute("read_package", json!({ "packageId": incident_id }))
        .unwrap();
    session
        .execute(
            "save_skill_document",
            json!({
                "packageId": incident_id,
                "editToken": incident_view["editToken"],
                "document": "---\nname: incident-response-v2\ndescription: 已在库内改名但未重新分发\n---\n恢复历史仍按原写入路径定位。\n"
            }),
        )
        .unwrap();

    // Agent directories remain byte-for-byte identical to the pre-edit applied version!
    assert_eq!(
        fs::read(codex_written.join("SKILL.md")).unwrap(),
        review_doc
    );
    let statuses_after_edit = session.execute("library_statuses", json!({})).unwrap();
    assert_eq!(
        status_of(&statuses_after_edit, &review_id, "codex"),
        "different"
    );

    // 5. Preview and commit restore of the distribution operation.
    let history = session.execute("list_history", json!({})).unwrap();
    assert!(history
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["operationId"] == dist_op_id && h["kind"] == "distribution"));

    let restore_preview = session
        .execute("preview_restore", json!({ "operationId": dist_op_id }))
        .unwrap();
    assert!(!restore_preview["blocked"].as_bool().unwrap());
    assert_eq!(restore_preview["groups"].as_array().unwrap().len(), 2);

    let restore_receipt = session
        .execute(
            "commit_restore",
            json!({ "previewId": restore_preview["previewId"] }),
        )
        .unwrap();
    assert_eq!(restore_receipt["outcomes"].as_array().unwrap().len(), 2);

    // Previously absent target directories are archived out of load roots (not permanently deleted).
    assert!(!codex_written.exists());
    assert!(!grok_written.exists());
    assert!(PathBuf::from(restore_receipt["backupLocation"].as_str().unwrap()).is_dir());

    // Source Claude packages, unselected Antigravity package, and library edits remain intact.
    assert_eq!(fs::read(review_src.join("SKILL.md")).unwrap(), review_doc);
    assert_eq!(
        fs::read(antigravity_untouched.join("SKILL.md")).unwrap(),
        untouched_bytes
    );
    let current_review = session
        .execute("read_package", json!({ "packageId": review_id }))
        .unwrap();
    assert!(current_review["document"]
        .as_str()
        .unwrap()
        .contains("仅保存在配置库的新版本"));
}

#[test]
fn fifth_agent_json_negative_guards_probe_invalidation_and_restore() {
    let (mut session, home, _) = isolated_workspace();
    let agents_dir = home.join(".skill-hub/agents");
    fs::create_dir_all(&agents_dir).unwrap();

    // 1. Negative guard: invalid custom profile on disk is isolated as "invalid" while builtins stay usable.
    fs::write(
        agents_dir.join("custom.broken.json"),
        br#"{"schemaVersion":1,"id":"../escape","displayName":"Broken"}"#,
    )
    .unwrap();
    let profiles = session.execute("list_agent_profiles", json!({})).unwrap();
    assert!(profiles
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["state"] == "invalid"));

    // 2. Negative guard: valid fifth agent JSON placed on disk without approval stays "pending" and cannot scan or write.
    let fifth_raw = include_str!("../../../../tests/fixtures/agents/custom.fifth.json");
    fs::write(agents_dir.join("custom.fifth.json"), fifth_raw).unwrap();
    let profiles = session.execute("list_agent_profiles", json!({})).unwrap();
    let pending = profiles
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "custom.fifth")
        .unwrap();
    assert_eq!(pending["state"], "pending");
    assert_eq!(
        session
            .execute("list_agent_skills", json!({ "agentId": "custom.fifth" }))
            .unwrap_err()
            .code,
        "profile_not_enabled"
    );

    // Import one valid skill into library for distribution tests.
    let src = home.join("source-skill");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("SKILL.md"),
        "---\nname: fifth-check\ndescription: Runtime fifth agent test.\n---\nBody.\n",
    )
    .unwrap();
    let scan = session
        .execute("scan_import", json!({ "source": src }))
        .unwrap();
    let item_id = scan["items"][0]["itemId"].as_str().unwrap();
    let imp = session
        .execute(
            "preview_import",
            json!({ "scanId": scan["scanId"], "choices": [{ "itemId": item_id, "action": "new", "replaceId": null }] }),
        )
        .unwrap();
    session
        .execute("commit_import", json!({ "previewId": imp["previewId"] }))
        .unwrap();
    let pkg_id = session.execute("load_library", json!({})).unwrap()["records"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Unapproved custom.fifth cannot be selected for distribution.
    assert!(session
        .execute(
            "preview_distribution",
            json!({ "selection": [{ "packageId": pkg_id, "agent": "custom.fifth" }] }),
        )
        .is_err());
    assert!(!home.join(".fifth").exists());

    // 3. Approve and enable custom.fifth via standard IPC using its read revision.
    fs::create_dir_all(home.join(".fifth")).unwrap();
    fs::write(
        home.join(".fifth/config.json"),
        r#"{"compat":{"shared":{"skills":true}}}"#,
    )
    .unwrap();
    let read_doc = session
        .execute("read_agent_profile", json!({ "agentId": "custom.fifth" }))
        .unwrap();
    let prof_preview = session
        .execute(
            "preview_agent_profile",
            json!({
                "profileJson": read_doc["profileJson"],
                "expectedRevision": read_doc["revision"],
                "localRootOverrides": [],
                "enableAfterSave": true
            }),
        )
        .unwrap();
    assert!(!prof_preview["blocked"].as_bool().unwrap());
    session
        .execute(
            "commit_agent_profile",
            json!({ "previewId": prof_preview["previewId"] }),
        )
        .unwrap();

    // 4. Shared root reader detection is dynamic (not hardcoded to 4 built-in IDs).
    let codex_preview = session
        .execute(
            "preview_distribution",
            json!({ "selection": [{ "packageId": pkg_id, "agent": "codex" }] }),
        )
        .unwrap();
    let readers = codex_preview["groups"][0]["possibleReaders"]
        .as_array()
        .unwrap();
    assert!(readers
        .iter()
        .any(|r| r["agent"] == "custom.fifth" && r["evidence"] == "configEnabled"));

    // 5. Negative guard: changing custom.fifth's boolean probe file invalidates an existing preview before write!
    let stale_preview = session
        .execute(
            "preview_distribution",
            json!({ "selection": [{ "packageId": pkg_id, "agent": "custom.fifth" }] }),
        )
        .unwrap();
    fs::write(
        home.join(".fifth/config.json"),
        r#"{"compat":{"shared":{"skills":false}}}"#,
    )
    .unwrap();
    assert!(session
        .execute(
            "commit_distribution",
            json!({ "previewId": stale_preview["previewId"] }),
        )
        .is_err());
    assert!(!home.join(".fifth/skills/fifth-check").exists());

    // 6. Fresh preview -> commit -> restore works cleanly on custom.fifth.
    let fresh_preview = session
        .execute(
            "preview_distribution",
            json!({ "selection": [{ "packageId": pkg_id, "agent": "custom.fifth" }] }),
        )
        .unwrap();
    let applied = session
        .execute(
            "commit_distribution",
            json!({ "previewId": fresh_preview["previewId"] }),
        )
        .unwrap();
    assert!(home.join(".fifth/skills/fifth-check/SKILL.md").is_file());

    let restore_preview = session
        .execute(
            "preview_restore",
            json!({ "operationId": applied["operationId"] }),
        )
        .unwrap();
    session
        .execute(
            "commit_restore",
            json!({ "previewId": restore_preview["previewId"] }),
        )
        .unwrap();
    assert!(!home.join(".fifth/skills/fifth-check").exists());
}
