//! Statistical analysis for dataset columns.

use polars::prelude::*;
use serde::{Deserialize, Serialize};

/// Computed statistics for a numeric column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub q25: f64,
    pub q75: f64,
    pub skewness: Option<f64>,
    pub kurtosis: Option<f64>,
    pub zero_count: usize,
    pub negative_count: usize,
    pub outlier_count: usize,
}

/// Computed statistics for a string/categorical column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoricalStats {
    pub top_values: Vec<(String, usize)>,
    pub avg_length: f64,
    pub empty_count: usize,
    pub pattern_mismatch_count: usize,
}

/// Column-level statistics, typed by column kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColumnStats {
    Numeric(NumericStats),
    Categorical(CategoricalStats),
}

impl ColumnStats {
    /// Compute statistics for a single column based on its dtype.
    pub fn compute(col: &Column) -> Option<Self> {
        let dt = col.dtype();
        if dt.is_primitive_numeric() {
            let stats = compute_numeric(col);
            stats.map(ColumnStats::Numeric)
        } else if dt.is_categorical() || dt.is_string() || dt.is_enum() {
            let stats = compute_categorical(col);
            stats.map(ColumnStats::Categorical)
        } else {
            None
        }
    }
}

fn compute_numeric(col: &Column) -> Option<NumericStats> {
    let vals: Vec<f64> = col.f64().ok()?.into_no_null_iter().collect();

    if vals.is_empty() {
        return None;
    }

    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    let mut sorted = vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = percentile(&sorted, 50.0);
    let q25 = percentile(&sorted, 25.0);
    let q75 = percentile(&sorted, 75.0);
    let iqr = q75 - q25;
    let lower_fence = q25 - 1.5 * iqr;
    let upper_fence = q75 + 1.5 * iqr;

    let outlier_count = vals
        .iter()
        .filter(|v| **v < lower_fence || **v > upper_fence)
        .count();

    let zero_count = vals.iter().filter(|v| **v == 0.0).count();
    let negative_count = vals.iter().filter(|v| **v < 0.0).count();

    let skewness = if std_dev > 0.0 {
        let m3 = vals.iter().map(|v| (v - mean).powi(3)).sum::<f64>() / n;
        Some(m3 / std_dev.powi(3))
    } else {
        None
    };

    let kurtosis = if std_dev > 0.0 {
        let m4 = vals.iter().map(|v| (v - mean).powi(4)).sum::<f64>() / n;
        Some(m4 / std_dev.powi(4) - 3.0)
    } else {
        None
    };

    Some(NumericStats {
        min: sorted[0],
        max: sorted[sorted.len() - 1],
        mean,
        median,
        std_dev,
        q25,
        q75,
        skewness,
        kurtosis,
        zero_count,
        negative_count,
        outlier_count,
    })
}

fn compute_categorical(col: &Column) -> Option<CategoricalStats> {
    let utf8 = col.str().ok()?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_len = 0usize;
    let mut empty_count = 0usize;
    let mut total = 0usize;

    for val in utf8.into_no_null_iter() {
        total += 1;
        total_len += val.len();
        if val.is_empty() {
            empty_count += 1;
        }
        *counts.entry(val.to_string()).or_insert(0) += 1;
    }

    let avg_length = if total > 0 {
        total_len as f64 / total as f64
    } else {
        0.0
    };

    let mut top_values: Vec<(String, usize)> = counts.into_iter().collect();
    top_values.sort_by(|a, b| b.1.cmp(&a.1));
    top_values.truncate(10);

    Some(CategoricalStats {
        top_values,
        avg_length,
        empty_count,
        pattern_mismatch_count: 0,
    })
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0) * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = idx - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}
