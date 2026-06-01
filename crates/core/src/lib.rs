//! Core linting engine for dataset-linter.
//!
//! Provides dataset loading, statistical analysis, and rule evaluation
//! for detecting quality issues in ML datasets.

pub mod builtin_rules;
pub mod dataset;
pub mod diagnostic;
pub mod lint;
pub mod rules;
pub mod stats;

pub use dataset::Dataset;
pub use diagnostic::{Diagnostic, Severity, SourceSpan};
pub use lint::{LintResult, Linter};
pub use rules::Rule;
