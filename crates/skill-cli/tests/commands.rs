use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
fn home() -> PathBuf {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "skill-cli-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    fs::canonicalize(p).unwrap()
}
fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skill-hub"))
        .arg("--home")
        .arg(home)
        .args(args)
        .output()
        .unwrap()
}
fn success(home: &Path, args: &[&str]) -> serde_json::Value {
    let output = run(home, args);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn apply_without_yes_is_read_only_exit_2() {
    let p = home();
    assert_eq!(run(&p, &["apply"]).status.code(), Some(2));
    assert!(!p.join(".skill-hub").exists());
}
#[test]
fn dry_run_has_no_backups_or_audit_and_home_is_isolated() {
    let p = home();
    success(&p, &["apply", "--dry-run", "--yes"]);
    success(&p, &["agents", "list"]);
    success(&p, &["list"]);
    assert!(!p.join(".skill-hub").exists());
}
#[test]
fn restore_requires_yes() {
    let p = home();
    assert_eq!(run(&p, &["restore", "nonexistent"]).status.code(), Some(2));
    assert!(!p.join(".skill-hub").exists());
}
#[test]
fn same_binary_adds_fifth_agent_imports_applies_and_restores() {
    let p = home();
    let profile = p.join("profile.json");
    fs::write(&profile,br#"{"schemaVersion":1,"id":"custom.demo","displayName":"Demo Agent","packageFormat":"skill-md-v1","variants":["default"],"defaultVariant":"default","roots":[{"rootId":"native","platforms":["windows","macos","linux"],"scope":"user","base":"home","relative":".demo/skills","discovery":"children","role":"native","read":true,"write":true,"priority":100}]}"#).unwrap();
    let binary = fs::read(env!("CARGO_BIN_EXE_skill-hub")).unwrap();
    success(
        &p,
        &["agents", "preview", "--file", profile.to_str().unwrap()],
    );
    assert!(!p.join(".skill-hub").exists());
    success(
        &p,
        &[
            "agents",
            "save",
            "--file",
            profile.to_str().unwrap(),
            "--enable",
            "--yes",
        ],
    );
    let profiles = success(&p, &["agents", "list"]);
    assert!(profiles.to_string().contains("custom.demo"));
    let source = p.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: review\ndescription: Fixture.\n---\n",
    )
    .unwrap();
    fs::write(source.join("asset.bin"), [0, 255]).unwrap();
    success(
        &p,
        &[
            "import",
            "--source",
            source.to_str().unwrap(),
            "--new",
            "--yes",
        ],
    );
    let library = success(&p, &["list"]);
    let id = library["records"][0]["id"].as_str().unwrap();
    let pair = format!("{id}=custom.demo");
    success(&p, &["diff", "--pair", &pair]);
    assert!(!p.join(".demo/skills/review").exists());
    let applied = success(&p, &["apply", "--pair", &pair, "--yes"]);
    assert_eq!(
        fs::read(p.join(".demo/skills/review/asset.bin")).unwrap(),
        vec![0, 255]
    );
    success(
        &p,
        &["restore", applied["operationId"].as_str().unwrap(), "--yes"],
    );
    assert!(!p.join(".demo/skills/review").exists());
    assert!(source.join("asset.bin").exists());
    assert_eq!(binary, fs::read(env!("CARGO_BIN_EXE_skill-hub")).unwrap());
}

#[test]
fn claude_to_codex_grok_end_to_end_and_fifth_agent_negative_guards() {
    let p = home();
    let claude = p.join(".claude/skills");
    let review = claude.join("code-review");
    let incident = claude.join("incident-response");
    fs::create_dir_all(review.join("scripts")).unwrap();
    fs::create_dir_all(&incident).unwrap();
    let review_doc = b"---\r\nname: code-review\r\ndescription: \xe4\xb8\xad\xe6\x96\x87 CRLF \xe6\xa0\xa1\xe9\xaa\x8c\r\nx-extra: preserved\r\n---\r\nBody.\r\n";
    fs::write(review.join("SKILL.md"), review_doc).unwrap();
    fs::write(review.join("diagram.bin"), [0, 159, 255]).unwrap();
    fs::write(review.join("scripts/run.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(review.join("scripts/run.sh"))
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(review.join("scripts/run.sh"), perms).unwrap();
    }
    fs::write(
        incident.join("SKILL.md"),
        "---\nname: incident-response\ndescription: 应急排查\n---\nKeep logs.\n",
    )
    .unwrap();
    let untouched = p.join(".gemini/config/skills/untouched");
    fs::create_dir_all(&untouched).unwrap();
    fs::write(
        untouched.join("SKILL.md"),
        "---\nname: untouched\ndescription: Untouched.\n---\n",
    )
    .unwrap();

    // Portable reserved-name and Unix symlink package entries must be rejected without writing library.
    let bad_pkg = p.join("bad-entry");
    fs::create_dir_all(&bad_pkg).unwrap();
    fs::write(
        bad_pkg.join("SKILL.md"),
        "---\nname: bad-entry\ndescription: Invalid entry.\n---\n",
    )
    .unwrap();
    fs::write(bad_pkg.join("CON.txt"), "blocked").unwrap();
    assert_eq!(
        run(
            &p,
            &[
                "import",
                "--source",
                bad_pkg.to_str().unwrap(),
                "--new",
                "--yes"
            ],
        )
        .status
        .code(),
        Some(1)
    );
    #[cfg(unix)]
    {
        let bad_link = p.join("bad-link");
        fs::create_dir_all(&bad_link).unwrap();
        fs::write(
            bad_link.join("SKILL.md"),
            "---\nname: bad-link\ndescription: Symlink entry.\n---\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(bad_link.join("SKILL.md"), bad_link.join("linked.md")).unwrap();
        assert_eq!(
            run(
                &p,
                &[
                    "import",
                    "--source",
                    bad_link.to_str().unwrap(),
                    "--new",
                    "--yes"
                ],
            )
            .status
            .code(),
            Some(1)
        );
    }

    // Import valid Claude packages into Hub library.
    success(
        &p,
        &[
            "import",
            "--source",
            claude.to_str().unwrap(),
            "--new",
            "--yes",
        ],
    );
    let list = success(&p, &["list"]);
    let records = list["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let review_id = records.iter().find(|r| r["name"] == "code-review").unwrap()["id"]
        .as_str()
        .unwrap();
    let incident_id = records
        .iter()
        .find(|r| r["name"] == "incident-response")
        .unwrap()["id"]
        .as_str()
        .unwrap();

    // Exact-pair diff & apply: (review -> codex) and (incident -> grok), NOT 4 groups.
    let pair_codex = format!("{review_id}=codex");
    let pair_grok = format!("{incident_id}=grok");
    let diff = success(&p, &["diff", "--pair", &pair_codex, "--pair", &pair_grok]);
    let groups = diff["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    let applied = success(
        &p,
        &[
            "apply",
            "--pair",
            &pair_codex,
            "--pair",
            &pair_grok,
            "--yes",
        ],
    );
    assert_eq!(
        fs::read(p.join(".agents/skills/code-review/SKILL.md")).unwrap(),
        review_doc
    );
    assert_eq!(
        fs::read(p.join(".agents/skills/code-review/diagram.bin")).unwrap(),
        vec![0, 159, 255]
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            fs::metadata(p.join(".agents/skills/code-review/scripts/run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    assert!(p.join(".grok/skills/incident-response/SKILL.md").is_file());
    assert!(!p.join(".agents/skills/incident-response").exists());
    assert!(!p.join(".grok/skills/code-review").exists());
    assert!(untouched.join("SKILL.md").is_file());

    // Updating the library record without apply must never mutate applied target roots.
    fs::write(
        review.join("SKILL.md"),
        "---\nname: code-review\ndescription: 库已更新但未重新应用\n---\nUpdated library.\n",
    )
    .unwrap();
    success(
        &p,
        &[
            "import",
            "--source",
            review.to_str().unwrap(),
            "--replace-id",
            review_id,
            "--yes",
        ],
    );
    assert_eq!(
        fs::read(p.join(".agents/skills/code-review/SKILL.md")).unwrap(),
        review_doc
    );

    // Negative guards for fifth agent: invalid JSON and unapproved profile on disk.
    let invalid_profile = p.join("invalid-profile.json");
    fs::write(&invalid_profile, br#"{"schemaVersion":1,"id":"codex"}"#).unwrap();
    assert_eq!(
        run(
            &p,
            &[
                "agents",
                "validate",
                "--file",
                invalid_profile.to_str().unwrap()
            ],
        )
        .status
        .code(),
        Some(1)
    );
    let agents_dir = p.join(".skill-hub/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    let fifth_fixture = p.join("custom.fifth.json");
    fs::write(
        &fifth_fixture,
        include_str!("../../../tests/fixtures/agents/custom.fifth.json"),
    )
    .unwrap();
    fs::write(
        agents_dir.join("custom.fifth.json"),
        include_str!("../../../tests/fixtures/agents/custom.fifth.json"),
    )
    .unwrap();
    let pair_fifth = format!("{review_id}=custom.fifth");
    assert_eq!(
        run(&p, &["apply", "--pair", &pair_fifth, "--yes"])
            .status
            .code(),
        Some(1)
    );
    assert!(!p.join(".fifth/skills").exists());

    // Approve & enable fifth agent, verify dynamic shared reader on codex, apply & restore.
    fs::create_dir_all(p.join(".fifth")).unwrap();
    fs::write(
        p.join(".fifth/config.json"),
        r#"{"compat":{"shared":{"skills":true}}}"#,
    )
    .unwrap();
    success(
        &p,
        &[
            "agents",
            "save",
            "--file",
            fifth_fixture.to_str().unwrap(),
            "--enable",
            "--yes",
        ],
    );
    let shared_diff = success(&p, &["diff", "--pair", &pair_codex]);
    assert!(shared_diff["groups"][0]["possibleReaders"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["agent"] == "custom.fifth" && r["evidence"] == "configEnabled"));

    let fifth_applied = success(&p, &["apply", "--pair", &pair_fifth, "--yes"]);
    assert!(p.join(".fifth/skills/code-review/SKILL.md").is_file());
    success(
        &p,
        &[
            "restore",
            fifth_applied["operationId"].as_str().unwrap(),
            "--yes",
        ],
    );
    assert!(!p.join(".fifth/skills/code-review").exists());

    success(
        &p,
        &["restore", applied["operationId"].as_str().unwrap(), "--yes"],
    );
    assert!(!p.join(".agents/skills/code-review").exists());
    assert!(!p.join(".grok/skills/incident-response").exists());
    assert!(untouched.join("SKILL.md").is_file());
}
