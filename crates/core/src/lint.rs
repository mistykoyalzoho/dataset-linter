//! Linter orchestration — runs all rules and collects results.

use crate::dataset::Dataset;
use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::Rule;

/// Aggregated result from a lint run.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LintResult {
    pub dataset_name: String,
    pub rows: usize,
    pub cols: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl LintResult {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Info)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// The main linter that runs a set of rules against a dataset.
pub struct Linter {
    rules: Vec<Box<dyn Rule>>,
}

impl Linter {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn with_defaults() -> Self {
        let mut linter = Self::new();
        // Built-in rules are registered here
        linter.register(Box::new(crate::builtin_rules::HighNullRate));
        linter.register(Box::new(crate::builtin_rules::ConstantColumn));
        linter.register(Box::new(crate::builtin_rules::HighCardinality));
        linter.register(Box::new(crate::builtin_rules::OutlierDetector));
        linter.register(Box::new(crate::builtin_rules::EmptyColumn));
        linter.register(Box::new(crate::builtin_rules::DuplicateRows));
        linter
    }

    pub fn register(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    /// Run all rules against the dataset.
    pub fn lint(&self, dataset: &Dataset) -> LintResult {
        let diagnostics: Vec<Diagnostic> = self
            .rules
            .iter()
            .flat_map(|rule| {
                tracing::debug!("Running rule {}", rule.code());
                rule.check(dataset)
            })
            .collect();

        LintResult {
            dataset_name: dataset.name.clone(),
            rows: dataset.nrows(),
            cols: dataset.ncols(),
            diagnostics,
        }
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::with_defaults()
    }
}
