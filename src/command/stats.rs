//! Stats command for counting files by extension.
//!
//! This module implements a read-only `libra stats` command that walks the
//! current (or specified) directory and counts files grouped by their file
//! extension. Files without an extension are reported as `no_extension`.
//!
//! Directories named `.libra` and `target` are excluded from the count,
//! matching the default ignore pattern used by other Libra commands.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io::{self, Write},
};

use clap::Parser;
use serde::Serialize;
use walkdir::WalkDir;

use crate::utils::{
    error::{CliError, CliResult},
    output::{OutputConfig, emit_json_data},
};

const STATS_EXAMPLES: &str = "\
EXAMPLES:
    libra stats                Count files in current directory by extension
    libra stats --json         Emit results as JSON
    libra stats /path/to/dir   Count files in a specific directory";

const IGNORED_DIR_NAMES: &[&str] = &[".libra", "target"];

#[derive(Parser, Debug)]
#[command(after_help = STATS_EXAMPLES)]
pub struct StatsArgs {
    /// Emit structured JSON output
    #[clap(long = "json")]
    pub json: bool,

    /// Directory to analyze. Defaults to the current working directory.
    pub directory: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StatsOutput {
    directory: String,
    total_files: usize,
    extensions: BTreeMap<String, usize>,
}

/// CLI entry point (standalone usage without an [`OutputConfig`]).
pub async fn execute(args: StatsArgs) {
    if let Err(e) = execute_safe(args, &OutputConfig::default()).await {
        e.print_stderr();
    }
}

/// Structured entry point called by the CLI dispatcher in [`super::cli`].
pub async fn execute_safe(args: StatsArgs, output: &OutputConfig) -> CliResult<()> {
    let dir = args.directory.unwrap_or_else(|| ".".to_string());
    let stats = count_files_by_extension(&dir)?;

    if args.json || output.is_json() {
        emit_json_data("stats", &stats, output)?;
    } else if !output.quiet {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        render_stats(&stats, &mut handle)?;
    }

    Ok(())
}

/// Recursively walk `dir` and count files grouped by extension.
///
/// Directories whose name is in [`IGNORED_DIR_NAMES`] are skipped entirely,
/// along with any dot-directories that should not be visible to the user.
fn count_files_by_extension(dir: &str) -> CliResult<StatsOutput> {
    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;

    for entry in WalkDir::new(dir).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir()
            && e.file_name()
                .to_str()
                .is_some_and(|name| IGNORED_DIR_NAMES.contains(&name)))
    }) {
        let entry =
            entry.map_err(|e| CliError::fatal(format!("failed to read directory entry: {e}")))?;

        if !entry.file_type().is_file() {
            continue;
        }

        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("no_extension")
            .to_lowercase();

        *extensions.entry(ext).or_insert(0) += 1;
        total += 1;
    }

    Ok(StatsOutput {
        directory: dir.to_string(),
        total_files: total,
        extensions,
    })
}

/// Render the stats as human-readable text.
///
/// Outputs directory, total count, and a per-extension breakdown
/// sorted alphabetically by extension name.
fn render_stats(stats: &StatsOutput, writer: &mut impl Write) -> CliResult<()> {
    let mut buf = String::new();

    let _ = writeln!(buf, "Directory: {}", stats.directory);
    let _ = writeln!(buf, "Total files: {}", stats.total_files);

    if stats.total_files == 0 {
        let _ = writeln!(buf);
        let _ = writeln!(buf, "  (no files found)");
    } else {
        let _ = writeln!(buf);
        let max_width = stats.extensions.keys().map(|k| k.len()).max().unwrap_or(0);
        for (ext, count) in &stats.extensions {
            let _ = writeln!(
                buf,
                "  {:>max_width$}  {}",
                ext,
                count,
                max_width = max_width
            );
        }
    }

    writer
        .write_all(buf.as_bytes())
        .map_err(|e| CliError::fatal(format!("stats output error: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_defaults() {
        let args = StatsArgs::parse_from(["stats"]);
        assert!(!args.json);
        assert!(args.directory.is_none());
    }

    #[test]
    fn test_parse_args_with_json_and_directory() {
        let args = StatsArgs::parse_from(["stats", "--json", "/tmp/test"]);
        assert!(args.json);
        assert_eq!(args.directory.as_deref(), Some("/tmp/test"));
    }
}
