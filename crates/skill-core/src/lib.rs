//! Pure Skill package validation, profile rules and distribution planning.
pub mod error;
pub mod compatibility;
pub mod model;
pub mod package;
pub mod profile;
pub mod plan;
pub mod diff;

pub use error::CoreError;
pub use model::*;
pub use package::{compare_packages, parse_skill_document, validate_package};
