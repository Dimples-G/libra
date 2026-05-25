//! Integration tests for the `libra stats` command.

use std::fs;

use serde::Deserialize;
use serde_json::Value;
use tempfile::tempdir;

use crate::assert_cli_success;
use crate::run_libra_command;

#[derive(Debug, Deserialize)]
struct StatsOutput {
    directory: String,
    total_files: u64,
    extensions: std::collections::BTreeMap<String, u64>,
}

/// Parse JSON stdout into a StatsOutput struct.
fn parse_stats_output(stdout: &[u8]) -> StatsOutput {
    let root: Value = serde_json::from_slice(stdout).expect("expected valid JSON");
    let data = root
        .get("data")
        .expect("expected 'data' key in JSON envelope");
    serde_json::from_value(data.clone()).expect("expected valid StatsOutput")
}

/// Create a directory tree with assorted extensions, plus .libra/ and target/
/// directories that must be excluded from counts.
fn setup_stats_workspace(root: &std::path::Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join(".libra")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::create_dir_all(root.join("target").join("debug")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("empty_dir")).unwrap();

    fs::write(root.join("README.md"), "# readme").unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("src").join("lib.rs"), "// lib").unwrap();
    fs::write(root.join("Cargo.toml"), "[package]").unwrap();
    fs::write(root.join("Cargo.lock"), "").unwrap();
    fs::write(root.join("build.rs"), "fn main() {}").unwrap();
    fs::write(root.join("docs").join("guide.md"), "# guide").unwrap();
    fs::write(root.join("LICENSE"), "MIT").unwrap();
    fs::write(root.join("Makefile"), "all:").unwrap();
    fs::write(root.join(".libra").join("config"), "config").unwrap();
    fs::write(root.join("target").join("debug").join("binary.o"), "obj").unwrap();
}

#[test]
fn stats_text_output_counts_extensions_and_skips_ignored_dirs() {
    let tmp = tempdir().unwrap();
    setup_stats_workspace(tmp.path());

    let output = run_libra_command(&["stats"], tmp.path());
    assert_cli_success(&output, "stats should succeed on a directory");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Total files: 7"),
        "expected 7 files (9 created minus 2 in ignored dirs), got:\n{stdout}"
    );
    assert!(
        stdout.contains("md: 2"),
        "expected 2 .md files, got:\n{stdout}"
    );
    assert!(
        stdout.contains("rs: 3"),
        "expected 3 .rs files, got:\n{stdout}"
    );
    assert!(
        stdout.contains("toml: 1"),
        "expected 1 .toml file, got:\n{stdout}"
    );
    assert!(
        stdout.contains("lock: 1"),
        "expected 1 .lock file, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("no_extension"),
        "no_extension category should not appear when there are no extensionless files, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Directory:"),
        "expected directory header, got:\n{stdout}"
    );
}

#[test]
fn stats_json_output_is_structured() {
    let tmp = tempdir().unwrap();
    setup_stats_workspace(tmp.path());

    let output = run_libra_command(&["stats", "--json"], tmp.path());
    assert_cli_success(&output, "stats --json should succeed");

    let stats = parse_stats_output(&output.stdout);
    assert_eq!(stats.total_files, 7);
    assert_eq!(stats.extensions.get("md"), Some(&2));
    assert_eq!(stats.extensions.get("rs"), Some(&3));
    assert_eq!(stats.extensions.get("toml"), Some(&1));
    assert_eq!(stats.extensions.get("lock"), Some(&1));
    // .libra/config and target/debug/binary.o must be excluded
    assert!(!stats.extensions.contains_key("o"));
    assert!(!stats.extensions.contains_key("config"));
}

#[test]
fn stats_counts_extensionless_files_as_no_extension() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("README"), "readme").unwrap();
    fs::write(tmp.path().join("LICENSE"), "MIT").unwrap();
    fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

    let output = run_libra_command(&["stats"], tmp.path());
    assert_cli_success(&output, "stats should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no_extension: 2"),
        "expected 2 extensionless files, got:\n{stdout}"
    );
    assert!(
        stdout.contains("rs: 1"),
        "expected 1 .rs file, got:\n{stdout}"
    );
}

#[test]
fn stats_on_specified_directory() {
    let tmp = tempdir().unwrap();
    let sub = tmp.path().join("subdir");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("a.txt"), "a").unwrap();
    fs::write(sub.join("b.txt"), "b").unwrap();
    fs::write(sub.join("c.png"), "c").unwrap();

    let output = run_libra_command(&["stats", "subdir"], tmp.path());
    assert_cli_success(&output, "stats subdir should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("txt: 2"),
        "expected 2 .txt files in subdir, got:\n{stdout}"
    );
    assert!(
        stdout.contains("png: 1"),
        "expected 1 .png file in subdir, got:\n{stdout}"
    );
}

#[test]
fn stats_on_empty_directory() {
    let tmp = tempdir().unwrap();

    let output = run_libra_command(&["stats"], tmp.path());
    assert_cli_success(&output, "stats on empty dir should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Total files: 0"),
        "expected 0 files in empty directory, got:\n{stdout}"
    );
}
