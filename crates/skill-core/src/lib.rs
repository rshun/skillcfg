//! Pure Skill package validation, profile rules and distribution planning.
pub mod error;
pub mod model;
pub mod package;

pub use error::CoreError;
pub use model::*;
pub use package::{compare_packages, parse_skill_document, validate_package};
