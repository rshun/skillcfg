/// Deliberately contains no parser source, original text or credential values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct CoreError {
    pub code: &'static str,
}

impl CoreError {
    pub fn new(code: &'static str) -> Self {
        Self { code }
    }
}
