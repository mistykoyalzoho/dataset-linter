//! Built-in lint rules for common dataset quality issues.

use crate::dataset::Dataset;
use crate::diagnostic::{Diagnostic, SourceSpan};
use crate::rules::Rule;

/// Columns with >50% null values.
pub struct HighNullRate;

impl Rule for HighNullRate {
    fn code(&self) -> &str {
        "DQ-001"
    }
    fn name(&self) -> &str {
        "high-null-rate"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        dataset
            .columns
            .iter()
            .filter_map(|col| {
                if col.null_pct > 50.0 {
                    Some(
                        Diagnostic::error(
                            self.code(),
                            format!("Column '{}' is {:.1}% null", col.name, col.null_pct),
                        )
                        .with_detail(
                            "Columns with majority null values are often dropped or imputed before training.",
                        )
                        .with_suggestion(format!(
                            "Consider dropping this column or using imputation: df['{}'].fillna(method='ffill')",
                            col.name
                        ))
                        .with_source(SourceSpan {
                            column: Some(col.name.clone()),
                            row_start: None,
                            row_end: None,
                        }),
                    )
                } else if col.null_pct > 5.0 {
                    Some(
                        Diagnostic::warning(
                            self.code(),
                            format!("Column '{}' has {:.1}% null values", col.name, col.null_pct),
                        )
                        .with_suggestion("Evaluate whether nulls are informative or should be imputed.")
                        .with_source(SourceSpan {
                            column: Some(col.name.clone()),
                            row_start: None,
                            row_end: None,
                        }),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Columns with only one unique value (zero variance).
pub struct ConstantColumn;

impl Rule for ConstantColumn {
    fn code(&self) -> &str {
        "DQ-002"
    }
    fn name(&self) -> &str {
        "constant-column"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        dataset
            .columns
            .iter()
            .filter_map(|col| {
                if col.unique_count <= 1 && col.null_count == 0 {
                    Some(
                        Diagnostic::warning(
                            self.code(),
                            format!(
                                "Column '{}' has only {} unique value — zero variance",
                                col.name, col.unique_count
                            ),
                        )
                        .with_detail(
                            "Constant columns provide no signal to models and waste memory.",
                        )
                        .with_suggestion("Drop this column before training.")
                        .with_source(SourceSpan {
                            column: Some(col.name.clone()),
                            row_start: None,
                            row_end: None,
                        }),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Categorical columns with very high cardinality relative to row count.
pub struct HighCardinality;

impl Rule for HighCardinality {
    fn code(&self) -> &str {
        "DQ-003"
    }
    fn name(&self) -> &str {
        "high-cardinality"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        let nrows = dataset.nrows() as f64;
        if nrows == 0.0 {
            return vec![];
        }

        dataset
            .columns
            .iter()
            .filter_map(|col| {
                let ratio = col.unique_count as f64 / nrows;
                if ratio > 0.95 && col.unique_count > 100 {
                    Some(
                        Diagnostic::warning(
                            self.code(),
                            format!(
                                "Column '{}' has {} unique values ({:.0}% of rows) — possible ID/leakage column",
                                col.name, col.unique_count, ratio * 100.0
                            ),
                        )
                        .with_detail(
                            "Near-unique columns are often identifiers that cause data leakage in ML models.",
                        )
                        .with_suggestion("Verify this isn't an ID column; if so, exclude from training features.")
                        .with_source(SourceSpan {
                            column: Some(col.name.clone()),
                            row_start: None,
                            row_end: None,
                        }),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Detect outliers using IQR method.
pub struct OutlierDetector;

impl Rule for OutlierDetector {
    fn code(&self) -> &str {
        "DQ-004"
    }
    fn name(&self) -> &str {
        "outlier-detector"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        dataset
            .columns
            .iter()
            .filter_map(|col| {
                if let Some(crate::stats::ColumnStats::Numeric(ns)) = &col.stats {
                    let nrows = dataset.nrows() as f64;
                    if nrows == 0.0 {
                        return None;
                    }
                    let outlier_pct = ns.outlier_count as f64 / nrows * 100.0;
                    if outlier_pct >= 5.0 {
                        Some(
                            Diagnostic::warning(
                                self.code(),
                                format!(
                                    "Column '{}' has {} outliers ({:.1}% of rows, IQR method)",
                                    col.name, ns.outlier_count, outlier_pct
                                ),
                            )
                            .with_detail(format!(
                                "Range: [{}, {}], mean: {:.2}, std: {:.2}",
                                ns.min, ns.max, ns.mean, ns.std_dev
                            ))
                            .with_suggestion("Investigate whether outliers are real measurements or data entry errors.")
                            .with_source(SourceSpan {
                                column: Some(col.name.clone()),
                                row_start: None,
                                row_end: None,
                            }),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Columns that are entirely empty (all null).
pub struct EmptyColumn;

impl Rule for EmptyColumn {
    fn code(&self) -> &str {
        "DQ-005"
    }
    fn name(&self) -> &str {
        "empty-column"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        dataset
            .columns
            .iter()
            .filter_map(|col| {
                if col.null_count == dataset.nrows() && dataset.nrows() > 0 {
                    Some(
                        Diagnostic::error(
                            self.code(),
                            format!("Column '{}' is entirely null", col.name),
                        )
                        .with_suggestion("Drop this column — it contains no data.")
                        .with_source(SourceSpan {
                            column: Some(col.name.clone()),
                            row_start: None,
                            row_end: None,
                        }),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Detect fully duplicated rows.
pub struct DuplicateRows;

impl Rule for DuplicateRows {
    fn code(&self) -> &str {
        "DQ-006"
    }
    fn name(&self) -> &str {
        "duplicate-rows"
    }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        let dup_count = match dataset.df.is_duplicated() {
            Ok(mask) => mask.sum().unwrap_or(0) as usize,
            Err(_) => return vec![],
        };

        if dup_count > 0 {
            let pct = dup_count as f64 / dataset.nrows() as f64 * 100.0;
            vec![Diagnostic::warning(
                self.code(),
                format!("Dataset has {} duplicate rows ({:.1}%)", dup_count, pct),
            )
            .with_suggestion("Consider deduplicating: df.drop_duplicates()")]
        } else {
            vec![]
        }
    }
}
