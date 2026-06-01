//! Trait for pluggable lint rules.

use crate::dataset::Dataset;
use crate::diagnostic::Diagnostic;

/// A lint rule that inspects a dataset and produces diagnostics.
pub trait Rule: Send + Sync {
    /// Machine-readable rule identifier (e.g., "DQ-001").
    fn code(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Run this rule against a dataset.
    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic>;
}
