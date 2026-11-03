use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn home() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "skill-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
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
