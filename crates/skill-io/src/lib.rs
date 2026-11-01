//! Read-only filesystem boundaries. Stage one never writes Agent files.
pub mod paths;
pub mod snapshot;
pub use snapshot::{read_package_snapshot, IoError, PackageSnapshot, ScanLimits};
