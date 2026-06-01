# dataset-linter

> **AI-powered dataset quality linter for ML pipelines.**
> Catch null rates, outliers, data leakage, schema drift, and semantic anomalies — before they silently corrupt your models.

[![CI](https://github.com/rt/dataset-linter/actions/workflows/ci.yml/badge.svg)](https://github.com/rt/dataset-linter/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)

---

## Why

ML pipelines fail silently. A column that's 80% null, a near-unique "feature" that's actually an ID, a skewed distribution that shifts at inference time — these don't cause build errors. They cause **wrong predictions in production**.

`dataset-linter` brings the rigor of `clippy` and `eslint` to your data. It runs a battery of statistical checks and optionally uses OpenAI to surface non-obvious semantic issues that numbers alone can't catch.

## Features

| Feature | Description |
|---------|-------------|
| **Statistical rules** | Null rates, constant columns, outlier detection (IQR), high cardinality, duplicate rows |
| **AI-powered analysis** | Optional OpenAI integration for semantic insights — column purpose inference, leakage detection, bias signals |
| **SARIF output** | CI-compatible output format for GitHub Code Scanning integration |
| **Fast** | Written in Rust with Polars — lints a 1M-row CSV in seconds |
| **Extensible** | Plugin trait for custom rules |

## Quick Start

```bash
# Install from source
cargo install --path crates/cli

# Lint a dataset
dataset-linter lint data/train.csv

# With AI analysis
export OPENAI_API_KEY=sk-...
dataset-linter lint data/train.csv --ai

# CI mode (fail on errors, output SARIF)
dataset-linter lint data/train.csv --strict --format sarif --report results.sarif
```

## Example Output

```
Dataset: sample_dirty (20 rows × 9 cols)
Summary: 1 errors, 2 warnings, 0 info

┌──────────┬────────┬───────────────────┬──────────────────────────────────────────────────────────┐
│ Severity │ Code   │ Column            │ Message                                                  │
├──────────┼────────┼───────────────────┼──────────────────────────────────────────────────────────┤
│ ERROR    │ DQ-001 │ notes             │ Column 'notes' is 70.0% null                             │
│          │ 💡     │                   │ Consider dropping or imputing: df['notes'].fillna(...)    │
├──────────┼────────┼───────────────────┼──────────────────────────────────────────────────────────┤
│ WARN     │ DQ-001 │ salary            │ Column 'salary' has 10.0% null values                    │
│          │ 💡     │                   │ Evaluate whether nulls are informative or imputed.       │
├──────────┼────────┼───────────────────┼──────────────────────────────────────────────────────────┤
│ WARN     │ DQ-004 │ performance_score │ 1 outlier (5.0% of rows, IQR method)                     │
│          │ 💡     │                   │ Investigate: real measurements or data entry errors?     │
└──────────┴────────┴───────────────────┴──────────────────────────────────────────────────────────┘
```

## Rules

| Code | Name | Severity | Description |
|------|------|----------|-------------|
| `DQ-001` | `high-null-rate` | error/warn | Columns with >50% (error) or >20% (warn) null values |
| `DQ-002` | `constant-column` | warn | Columns with a single unique value (zero variance) |
| `DQ-003` | `high-cardinality` | warn | Near-unique columns — likely IDs or leakage vectors |
| `DQ-004` | `outlier-detector` | warn | Statistical outlier detection using the IQR method |
| `DQ-005` | `empty-column` | error | Columns that are entirely null |
| `DQ-006` | `duplicate-rows` | warn | Fully duplicated rows in the dataset |

## AI Analysis

When `--ai` is passed, `dataset-linter` sends a statistical summary to OpenAI and receives structured insights that automated rules miss:

- **Type suggestions** — "Column `zip_code` is stored as int but should be categorical"
- **Leakage warnings** — `purchase_amount` is likely the target, not a feature
- **Bias signals** — `gender` column has 95% one value; consider rebalancing
- **Feature engineering** — `joined_at` could be decomposed into tenure buckets

Requires `OPENAI_API_KEY` environment variable. Uses `gpt-4o-mini` by default (configurable via `--model`).

## CI Integration

### GitHub Actions

```yaml
- name: Lint datasets
  run: |
    cargo run --release -- lint data/train.csv \
      --strict --format sarif --report dataset-lint.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: dataset-lint.sarif
```

This surfaces dataset quality issues directly in GitHub's Security tab.

## Architecture

```
dataset-linter/
├── crates/
│   ├── core/         # Dataset loading, statistics, rule trait, built-in rules
│   ├── openai/       # AI-powered semantic analysis
│   ├── rules/        # Extensible rule registry (future: custom rule DSL)
│   └── cli/          # Binary, argument parsing, output formatting
├── examples/         # Sample datasets with known issues
└── tests/            # Integration tests
```

### Custom Rules

Implement the `Rule` trait:

```rust
use dataset_linter_core::{Dataset, Diagnostic, Rule};

pub struct MyCustomRule;

impl Rule for MyCustomRule {
    fn code(&self) -> &str { "CUSTOM-001" }
    fn name(&self) -> &str { "my-rule" }

    fn check(&self, dataset: &Dataset) -> Vec<Diagnostic> {
        // Your logic here
        vec![]
    }
}
```

## Codex for OSS

This project is designed to be a showcase for [OpenAI's Codex for OSS program](https://openai.com/form/codex-for-oss/):

- **Uses OpenAI models** for semantic dataset analysis
- **Built with Codex-compatible workflows** — the CLI, rules engine, and test suite are structured for AI-assisted development
- **Solves a real problem** — dataset quality is the #1 silent failure mode in ML
- **Rust + Polars** — performance-critical data tooling where Codex can help with the complex iterator/statistics code
- **Extensible architecture** — the rule plugin system is a natural fit for Codex-generated custom rules

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
