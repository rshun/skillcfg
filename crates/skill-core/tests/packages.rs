use skill_core::package::{MAX_DEPTH, MAX_ENTRIES, MAX_FILE_BYTES, MAX_PACKAGE_BYTES};
use skill_core::*;

fn file(path: &str, bytes: Vec<u8>) -> TreeEntry {
    TreeEntry { relative_path: path.into(), kind: EntryKind::File { bytes, unix_executable: false } }
}

fn document() -> Vec<u8> {
    b"---\nname: code-review\ndescription: Review changes carefully.\n---\n\nInstructions.\n".to_vec()
}

fn tree() -> PackageTree {
    PackageTree { entries: vec![file("SKILL.md", document())] }
}

#[test]
fn keeps_unknown_frontmatter_and_crlf_bytes() {
    let bytes = "\u{feff}---\r\nname: code-review\r\ndescription: 审查代码。\r\ncustom: [a, b]\r\n---\r\n正文。\r\n".as_bytes().to_vec();
    let original = bytes.clone();
    let meta = parse_skill_document(&bytes).unwrap();
    assert_eq!(meta.name, "code-review");
    assert_eq!(meta.description, "审查代码。");
    assert_eq!(bytes, original);
}

#[test]
fn rejects_duplicate_yaml_keys() {
    assert!(parse_skill_document(b"---\nname: first\nname: second\ndescription: Review.\n---\n").is_err());
}

#[test]
fn rejects_non_string_name() {
    assert!(parse_skill_document(b"---\nname: 123\ndescription: Review.\n---\n").is_err());
}

#[test]
fn rejects_65_char_name_but_accepts_64() {
    for (len, accepted) in [(1, true), (64, true), (65, false)] {
        let text = format!("---\nname: {}\ndescription: Review.\n---\n", "a".repeat(len));
        assert_eq!(parse_skill_document(text.as_bytes()).is_ok(), accepted);
    }
}

#[test]
fn errors_do_not_include_private_input() {
    let marker = "SYNTHETIC_PRIVATE_MARKER";
    let bytes = format!("---\nname: [bad\n{marker}\n---\n");
    let error = parse_skill_document(bytes.as_bytes()).unwrap_err();
    assert!(!format!("{error:?} {error}").contains(marker));
}

#[test]
fn detects_binary_and_executable_change() {
    let left = PackageTree { entries: vec![file("image.bin", vec![0, 255]), file("run.sh", vec![1])] };
    let right = PackageTree { entries: vec![file("image.bin", vec![0, 254]), TreeEntry { relative_path: "run.sh".into(), kind: EntryKind::File { bytes: vec![1], unix_executable: true } }] };
    assert_eq!(compare_packages(&left, &right).changed, vec!["image.bin", "run.sh"]);
}

#[test]
fn comparison_is_order_independent_and_includes_empty_directories() {
    let a = file("a", vec![1]);
    let b = file("b", vec![2]);
    assert!(compare_packages(&PackageTree { entries: vec![a.clone(), b.clone()] }, &PackageTree { entries: vec![b, a] }).is_equal());
    let result = compare_packages(&PackageTree::default(), &PackageTree { entries: vec![TreeEntry { relative_path: "empty".into(), kind: EntryKind::Directory }] });
    assert_eq!(result.added, vec!["empty"]);
}

#[test]
fn rejects_windows_case_collision_and_reserved_names() {
    let mut package = tree();
    package.entries.extend([file("readme", vec![]), file("README", vec![]), file("NUL.txt", vec![])]);
    let issues = validate_package(&package, Platform::Windows);
    assert!(issues.iter().any(|i| i.code == "case_collision"));
    assert!(issues.iter().any(|i| i.code == "invalid_path"));
    assert!(validate_package(&tree(), Platform::Windows).is_empty());
}

#[test]
fn rejects_escape_duplicate_and_file_directory_collisions() {
    for bad in ["../escape", "/absolute", "C:/escape", "a\\b", "a//b", "a/./b"] {
        let mut package = tree(); package.entries.push(file(bad, vec![]));
        assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "invalid_path"));
    }
    let mut package = tree(); package.entries.extend([file("a", vec![]), file("a/b", vec![])]);
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "path_collision"));
    package.entries.push(file("SKILL.md", document()));
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "duplicate_path"));
}

#[test]
fn enforces_file_package_entry_and_depth_boundaries() {
    let mut package = tree();
    package.entries.push(file("blob", vec![0; MAX_FILE_BYTES]));
    assert!(validate_package(&package, Platform::Linux).is_empty());
    package.entries.push(file("too-big", vec![0; MAX_FILE_BYTES + 1]));
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "file_too_large"));
    let mut package = tree();
    for n in 0..MAX_ENTRIES - 1 { package.entries.push(file(&format!("f{n}"), vec![])); }
    assert!(validate_package(&package, Platform::Linux).is_empty());
    package.entries.push(file("one-extra", vec![]));
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "too_many_entries"));
    let mut package = tree();
    package.entries.push(file(&vec!["d"; MAX_DEPTH].join("/"), vec![]));
    assert!(validate_package(&package, Platform::Linux).is_empty());
    package.entries.push(file(&vec!["d"; MAX_DEPTH + 1].join("/"), vec![]));
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "too_deep"));
    let mut package = tree(); let header = document().len();
    for n in 0..3 { package.entries.push(file(&format!("b{n}"), vec![0; MAX_FILE_BYTES])); }
    package.entries.push(file("last", vec![0; MAX_PACKAGE_BYTES - header - 3 * MAX_FILE_BYTES]));
    assert!(validate_package(&package, Platform::Linux).is_empty());
    package.entries.push(file("extra", vec![0]));
    assert!(validate_package(&package, Platform::Linux).iter().any(|i| i.code == "package_too_large"));
}

#[test]
fn agent_ids_are_lexical_not_a_four_agent_enum() {
    for id in ["codex", "custom.fifth-agent", "some-future-agent"] { assert!(AgentId::parse(id).is_ok()); }
    for id in ["", "../escape", "Bad", "a..b", "custom."] { assert!(AgentId::parse(id).is_err()); }
}
