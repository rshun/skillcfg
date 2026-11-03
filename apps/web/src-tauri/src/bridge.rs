use std::path::PathBuf;
#[derive(Debug, serde::Serialize)]
pub struct SafeError { pub code: String, pub message: String }
pub struct DesktopSession;
impl DesktopSession {
    pub fn new(_home: PathBuf) -> Result<Self, SafeError> { Ok(Self) }
    pub fn execute(&mut self, _command: &str, _request: serde_json::Value) -> Result<serde_json::Value, SafeError> {
        Err(SafeError { code: "not_implemented".into(), message: "尚未实现".into() })
    }
}
