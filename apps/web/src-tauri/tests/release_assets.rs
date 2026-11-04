#![cfg(not(debug_assertions))]
#[test]
fn release_uses_embedded_frontend_without_a_development_server() {
    assert!(!tauri::is_dev(), "release must use the custom protocol");
    let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let index = context.assets().get(&"index.html".into()).expect("embedded index");
    let text = std::str::from_utf8(index.as_ref()).expect("UTF-8 frontend");
    assert!(text.contains("/assets/"), "release must embed the built frontend");
}
