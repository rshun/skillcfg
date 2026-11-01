//! Pure Skill package validation, profile rules and distribution planning.
pub mod compatibility;
pub mod diff;
pub mod error;
pub mod model;
pub mod package;
pub mod plan;
pub mod profile;

pub use error::CoreError;
pub use model::*;
pub use package::{compare_packages, parse_skill_document, validate_package};
