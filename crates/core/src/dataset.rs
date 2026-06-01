//! Dataset loading and column-level statistics.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::stats::ColumnStats;

/// A loaded dataset with computed metadata.
#[derive(Debug, Clone)]
pub struct Dataset {
    pub df: DataFrame,
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

/// Metadata for a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub dtype: String,
    pub null_count: usize,
    pub null_pct: f64,
    pub unique_count: usize,
    pub stats: Option<ColumnStats>,
}

impl Dataset {
    /// Load a CSV file into a Dataset with computed statistics.
    pub fn from_csv(path: &Path) -> Result<Self, PolarsError> {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());

        let df = CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(path.to_path_buf()))?
            .finish()?;

        Self::from_dataframe(df, name)
    }

    /// Build a Dataset from an existing DataFrame.
    pub fn from_dataframe(df: DataFrame, name: String) -> Result<Self, PolarsError> {
        let nrows = df.height() as f64;
        let columns = df
            .get_columns()
            .iter()
            .map(|col| {
                let null_count = col.null_count();
                let null_pct = if nrows > 0.0 {
                    null_count as f64 / nrows * 100.0
                } else {
                    0.0
                };
                let unique_count = col.n_unique().unwrap_or(0);

                let stats = ColumnStats::compute(col);

                ColumnInfo {
                    name: col.name().to_string(),
                    dtype: format!("{}", col.dtype()),
                    null_count,
                    null_pct,
                    unique_count,
                    stats,
                }
            })
            .collect();

        Ok(Dataset { df, name, columns })
    }

    /// Number of rows.
    pub fn nrows(&self) -> usize {
        self.df.height()
    }

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        self.df.width()
    }

    /// Get column info by name.
    pub fn column_info(&self, name: &str) -> Option<&ColumnInfo> {
        self.columns.iter().find(|c| c.name == name)
    }
}
