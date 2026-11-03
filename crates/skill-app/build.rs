use std::{env, fs, path::PathBuf};

fn main() {
    let directory =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../profiles/builtin");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut files: Vec<_> = fs::read_dir(&directory)
        .expect("built-in profile directory")
        .map(|e| e.expect("profile entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut generated = String::from("pub const BUILTIN_DOCUMENTS: &[&str] = &[\n");
    for path in files {
        let document = fs::read_to_string(path).expect("UTF-8 built-in profile");
        generated.push_str(&format!("{document:?},\n"));
    }
    generated.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("builtin_profiles.rs"),
        generated,
    )
    .expect("generated profile data");
}
