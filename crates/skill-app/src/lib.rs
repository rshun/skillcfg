//! Host-independent application services. Stage one is read-only.
mod binding;
pub mod detect;
mod distribution;
pub mod env;
mod history;
mod importing;
mod library;
mod preview;
mod profiles;
pub mod registry;
pub mod roots;
pub mod service;
mod settings;
mod workbench;
pub use distribution::{DistributionRecord, ProfileAtWrite};
pub use history::HistoryItem;
pub use service::*;
pub use workbench::TargetStatus;

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
    pub(crate) state: std::cell::RefCell<preview::Session>,
    pub(crate) clock: std::sync::Arc<dyn Clock>,
    pub(crate) source: RegistrySource,
}
impl App {
    pub fn new(context: AppContext) -> Self {
        Self::with_clock(context, std::sync::Arc::new(SystemClock::default()))
    }
}
