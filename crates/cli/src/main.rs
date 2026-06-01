//! dataset-linter: AI-powered dataset quality linter for ML pipelines.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use dataset_linter_core::{Dataset, Linter};
use dataset_linter_openai::{Analyzer, OpenAIConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "dataset-linter",
    version,
    about = "AI-powered dataset quality linter for ML pipelines",
    long_about = "Lints CSV/Parquet datasets for quality issues using statistical rules \
                  and optional OpenAI-powered semantic analysis.\n\n\
                  Catches null rates, outliers, data leakage, schema drift, and more — \
                  before they silently corrupt your models."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging.
    #[arg(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint a dataset file for quality issues.
    Lint {
        /// Path to the dataset file (CSV).
        path: PathBuf,

        /// Output format.
        #[arg(long, short, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Enable AI-powered semantic analysis (requires OPENAI_API_KEY).
        #[arg(long)]
        ai: bool,

        /// OpenAI model to use for AI analysis.
        #[arg(long, default_value = "gpt-4o-mini")]
        model: String,

        /// Minimum severity to display.
        #[arg(long, value_enum, default_value = "info")]
        severity: SeverityFilter,

        /// Write JSON report to file.
        #[arg(long)]
        report: Option<PathBuf>,

        /// Fail with exit code 1 if errors are found.
        #[arg(long)]
        strict: bool,
    },

    /// Show dataset schema and basic statistics without linting.
    Inspect {
        /// Path to the dataset file.
        path: PathBuf,

        /// Output format.
        #[arg(long, short, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// List all available lint rules.
    Rules {
        /// Output format.
        #[arg(long, short, value_enum, default_value = "table")]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Sarif,
}

#[derive(Clone, ValueEnum)]
enum SeverityFilter {
    Info,
    Warning,
    Error,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("dataset_linter=debug")
    } else {
        tracing_subscriber::EnvFilter::new("dataset_linter=warn")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.command {
        Commands::Lint {
            path,
            format,
            ai,
            model,
            severity,
            report,
            strict,
        } => {
            let min_severity = match severity {
                SeverityFilter::Info => dataset_linter_core::Severity::Info,
                SeverityFilter::Warning => dataset_linter_core::Severity::Warning,
                SeverityFilter::Error => dataset_linter_core::Severity::Error,
            };

            // Load dataset
            eprintln!("{} {}", "Loading".bold(), path.display());
            let dataset = Dataset::from_csv(&path)?;

            // Run linter
            let linter = Linter::with_defaults();
            let result = linter.lint(&dataset);

            // Display results
            match format {
                OutputFormat::Table => print_table(&result, min_severity),
                OutputFormat::Json => print_json(&result, min_severity)?,
                OutputFormat::Sarif => print_sarif(&result)?,
            }

            // AI analysis
            if ai {
                eprintln!("\n{}", "Running AI analysis...".bold().cyan());
                let config = OpenAIConfig {
                    model,
                    ..Default::default()
                };
                let analyzer = Analyzer::new(config);
                match analyzer.analyze(&dataset, &result).await {
                    Ok(insights) => {
                        println!("\n{}", "AI Insights".bold().cyan());
                        println!("{}", "─".repeat(60));
                        for insight in &insights {
                            let tag = match insight.category {
                                dataset_linter_openai::InsightCategory::TypeSuggestion => "TYPE",
                                dataset_linter_openai::InsightCategory::QualityIssue => "QUALITY",
                                dataset_linter_openai::InsightCategory::FeatureEngineering => "FEAT",
                                dataset_linter_openai::InsightCategory::DataLeakage => "LEAK",
                                dataset_linter_openai::InsightCategory::BiasSignal => "BIAS",
                                dataset_linter_openai::InsightCategory::General => "INFO",
                            };
                            println!(
                                "  {} {}",
                                format!("[{tag}]").bold().yellow(),
                                insight.summary
                            );
                            println!("    → {}", insight.recommendation.dimmed());
                        }
                    }
                    Err(e) => {
                        eprintln!("{} AI analysis failed: {}", "Warning:".yellow(), e);
                    }
                }
            }

            // Write report
            if let Some(report_path) = report {
                let json = serde_json::to_string_pretty(&result)?;
                std::fs::write(&report_path, json)?;
                eprintln!("{} {}", "Report written to".green(), report_path.display());
            }

            // Exit code
            if strict && result.has_errors() {
                std::process::exit(1);
            }
        }

        Commands::Inspect { path, format } => {
            eprintln!("{} {}", "Inspecting".bold(), path.display());
            let dataset = Dataset::from_csv(&path)?;
            match format {
                OutputFormat::Table => {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL);
                    table.set_header(["Column", "Type", "Null %", "Unique", "Details"]);

                    for col in &dataset.columns {
                        let details = match &col.stats {
                            Some(dataset_linter_core::stats::ColumnStats::Numeric(ns)) => {
                                format!(
                                    "min={}, max={}, mean={:.2}, outliers={}",
                                    ns.min, ns.max, ns.mean, ns.outlier_count
                                )
                            }
                            Some(dataset_linter_core::stats::ColumnStats::Categorical(cs)) => {
                                format!("top='{}', empty={}",
                                    cs.top_values.first().map(|(v,_)| v.as_str()).unwrap_or(""),
                                    cs.empty_count
                                )
                            }
                            None => "—".into(),
                        };

                        table.add_row([
                            &col.name,
                            &col.dtype,
                            &format!("{:.1}%", col.null_pct),
                            &col.unique_count.to_string(),
                            &details,
                        ]);
                    }

                    println!("\n{}: {}", "Dataset".bold(), dataset.name);
                    println!("{}: {} rows, {} columns\n", "Shape".bold(), dataset.nrows(), dataset.ncols());
                    println!("{table}");
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&dataset.columns)?);
                }
                _ => {
                    eprintln!("SARIF format not supported for inspect");
                }
            }
        }

        Commands::Rules { format } => {
            match format {
                OutputFormat::Table => {
                    println!("{}", "Available Rules".bold());
                    println!("{}", "─".repeat(50));
                    // We'd need to expose rules from the linter for this
                    // For now, list them statically
                    let rules = [
                        ("DQ-001", "high-null-rate", "Columns with >50% null values"),
                        ("DQ-002", "constant-column", "Columns with zero variance"),
                        ("DQ-003", "high-cardinality", "Possible ID/leakage columns"),
                        ("DQ-004", "outlier-detector", "Statistical outlier detection (IQR)"),
                        ("DQ-005", "empty-column", "Entirely null columns"),
                        ("DQ-006", "duplicate-rows", "Fully duplicated rows"),
                    ];
                    for (code, name, desc) in &rules {
                        println!("  {} {} — {}", code.bold().green(), name, desc);
                    }
                }
                OutputFormat::Json => {
                    let rules: Vec<_> = [
                        ("DQ-001", "high-null-rate", "Columns with >50% null values"),
                        ("DQ-002", "constant-column", "Columns with zero variance"),
                        ("DQ-003", "high-cardinality", "Possible ID/leakage columns"),
                        ("DQ-004", "outlier-detector", "Statistical outlier detection (IQR)"),
                        ("DQ-005", "empty-column", "Entirely null columns"),
                        ("DQ-006", "duplicate-rows", "Fully duplicated rows"),
                    ].iter().map(|(c, n, d)| {
                        serde_json::json!({"code": c, "name": n, "description": d})
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&rules)?);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn print_table(result: &dataset_linter_core::LintResult, min_severity: dataset_linter_core::Severity) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["Severity", "Code", "Column", "Message"]);

    for diag in &result.diagnostics {
        if diag.severity < min_severity {
            continue;
        }
        let severity_str = match diag.severity {
            dataset_linter_core::Severity::Error => "ERROR".red().bold().to_string(),
            dataset_linter_core::Severity::Warning => "WARN".yellow().to_string(),
            dataset_linter_core::Severity::Info => "INFO".blue().to_string(),
        };
        let column = diag
            .source
            .as_ref()
            .and_then(|s| s.column.as_deref())
            .unwrap_or("—");

        table.add_row([&severity_str, &diag.code, column, &diag.message]);

        if let Some(suggestion) = &diag.suggestion {
            table.add_row(["", "💡", "", suggestion]);
        }
    }

    println!(
        "\n{}: {} rows × {} cols",
        "Dataset".bold(),
        result.rows,
        result.cols
    );
    println!(
        "{}: {} errors, {} warnings, {} info\n",
        "Summary".bold(),
        result.error_count(),
        result.warning_count(),
        result.info_count()
    );

    if result.diagnostics.is_empty() {
        println!("{}", "No issues found ✓".green());
    } else {
        println!("{table}");
    }
}

fn print_json(result: &dataset_linter_core::LintResult, min_severity: dataset_linter_core::Severity) -> Result<()> {
    let filtered: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity >= min_severity)
        .collect();

    let output = serde_json::json!({
        "dataset": result.dataset_name,
        "rows": result.rows,
        "columns": result.cols,
        "summary": {
            "errors": result.error_count(),
            "warnings": result.warning_count(),
            "info": result.info_count(),
        },
        "diagnostics": filtered,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_sarif(result: &dataset_linter_core::LintResult) -> Result<()> {
    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "dataset-linter",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/rt/dataset-linter",
                    "rules": result.diagnostics.iter().map(|d| {
                        serde_json::json!({
                            "id": d.code,
                            "shortDescription": {"text": d.message},
                            "defaultConfiguration": {
                                "level": match d.severity {
                                    dataset_linter_core::Severity::Error => "error",
                                    dataset_linter_core::Severity::Warning => "warning",
                                    dataset_linter_core::Severity::Info => "note",
                                }
                            }
                        })
                    }).collect::<Vec<_>>(),
                }
            },
            "results": result.diagnostics.iter().map(|d| {
                serde_json::json!({
                    "ruleId": d.code,
                    "level": match d.severity {
                        dataset_linter_core::Severity::Error => "error",
                        dataset_linter_core::Severity::Warning => "warning",
                        dataset_linter_core::Severity::Info => "note",
                    },
                    "message": {"text": d.message},
                })
            }).collect::<Vec<_>>(),
        }]
    });

    println!("{}", serde_json::to_string_pretty(&sarif)?);
    Ok(())
}
