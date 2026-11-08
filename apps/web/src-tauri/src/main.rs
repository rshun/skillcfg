#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(all(not(debug_assertions), not(feature = "custom-protocol")))]
compile_error!("release requires the custom-protocol feature");
fn main() {
    skill_desktop::run();
}
