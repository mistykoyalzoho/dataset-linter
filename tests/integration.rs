use dataset_linter_core::{Dataset, Linter, Severity};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn test_fixture(name: &str) -> PathBuf {
    // Use the examples directory for test fixtures
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join(name)
}

#[test]
fn test_lint_finds_high_null_rate() {
    let path = test_fixture("sample_dirty.csv");
    if !path.exists() {
        eprintln!("Skipping: test fixture not found at {}", path.display());
        return;
    }
    let dataset = Dataset::from_csv(&path).expect("Failed to load dataset");
    let linter = Linter::with_defaults();
    let result = linter.lint(&dataset);

    // We expect at least some diagnostics
    assert!(
        !result.diagnostics.is_empty(),
        "Expected diagnostics but found none"
    );

    // Check that DQ-001 fires for columns with nulls
    let null_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "DQ-001")
        .collect();
    assert!(
        !null_diags.is_empty(),
        "Expected DQ-001 (high-null-rate) diagnostics"
    );
}

#[test]
fn test_lint_finds_duplicates() {
    let path = test_fixture("sample_dirty.csv");
    if !path.exists() {
        eprintln!("Skipping: test fixture not found");
        return;
    }
    let dataset = Dataset::from_csv(&path).expect("Failed to load dataset");
    let linter = Linter::with_defaults();
    let result = linter.lint(&dataset);

    let dup_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "DQ-006")
        .collect();
    assert!(
        !dup_diags.is_empty(),
        "Expected DQ-006 (duplicate-rows) diagnostics"
    );
}

#[test]
fn test_severity_counts() {
    let path = test_fixture("sample_dirty.csv");
    if !path.exists() {
        eprintln!("Skipping: test fixture not found");
        return;
    }
    let dataset = Dataset::from_csv(&path).expect("Failed to load dataset");
    let linter = Linter::with_defaults();
    let result = linter.lint(&dataset);

    let total = result.error_count() + result.warning_count() + result.info_count();
    assert_eq!(total, result.diagnostics.len());
}
