//! Host-independent application services. Stage one is read-only.
pub mod detect;
pub mod env;
pub mod registry;
pub mod roots;

pub use env::HostEnv;
pub use registry::{
    load_registry, Approval, ProfileEntry, ProfileState, RegistrySnapshot, RegistrySource,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct AppError {
    pub code: &'static str,
}

impl From<skill_core::CoreError> for AppError {
    fn from(error: skill_core::CoreError) -> Self {
        Self { code: error.code }
    }
}
impl From<skill_io::IoError> for AppError {
    fn from(error: skill_io::IoError) -> Self {
        Self { code: error.code }
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub env: HostEnv,
    pub scope: skill_core::Scope,
    pub registry: RegistrySnapshot,
}
pub struct App {
    pub context: AppContext,
}
