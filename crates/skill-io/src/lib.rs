//! Read-only filesystem boundaries. Stage one never writes Agent files.
pub mod disk;
mod journal;
mod lock;
pub mod paths;
pub mod snapshot;
pub mod transaction;
pub use snapshot::{read_package_snapshot, IoError, PackageSnapshot, ScanLimits};
pub use transaction::*;
