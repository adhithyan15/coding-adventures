// Build report formatting — terminal-friendly output.
//
// # Output format
//
// The report is designed for terminal display — a fixed-width table with
// aligned columns, followed by a summary line:
//
//   Build Report
//   ============
//   Package                    Status     Duration
//   python/logic-gates         SKIPPED    -
//   python/arithmetic          BUILT      2.3s
//   python/arm-simulator       FAILED     0.5s
//   python/riscv-simulator     DEP-SKIP   - (dep failed)
//
//   Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped
//
// The report is sorted by package name for consistent output across runs.
// Status names are uppercased for visual prominence.

use std::collections::HashMap;
use std::io::Write;

use crate::executor::BuildResult;

// ---------------------------------------------------------------------------
// Status display names
// ---------------------------------------------------------------------------

/// Maps internal status strings to display names.
/// We uppercase for visual clarity in the terminal.
fn status_display(status: &str) -> &str {
    match status {
        "built" => "BUILT",
        "failed" => "FAILED",
        "skipped" => "SKIPPED",
        "dep-skipped" => "DEP-SKIP",
        "would-build" => "WOULD-BUILD",
        _ => "UNKNOWN",
    }
}

/// Converts seconds to a display string.
/// Returns "-" for negligible durations, otherwise "X.Ys".
fn format_duration(seconds: f64) -> String {
    if seconds < 0.01 {
        "-".to_string()
    } else {
        format!("{:.1}s", seconds)
    }
}

// ---------------------------------------------------------------------------
// Report formatting
// ---------------------------------------------------------------------------

/// Produces the build report as a string. This is the pure function —
/// it doesn't print anything, making it easy to test.
pub fn format_report(results: &HashMap<String, BuildResult>) -> String {
    let mut buf = String::new();

    buf.push_str("\nBuild Report\n");
    buf.push_str("============\n");

    if results.is_empty() {
        buf.push_str("No packages processed.\n");
        return buf;
    }

    // Calculate the maximum package name length for column alignment.
    let max_name_len = results
        .keys()
        .map(|n| n.len())
        .max()
        .unwrap_or(7) // "Package" header length
        .max(7);

    // Header row.
    buf.push_str(&format!(
        "{:<width$}   {:<12} {}\n",
        "Package",
        "Status",
        "Duration",
        width = max_name_len
    ));

    // Sort results by package name for consistent output.
    let mut names: Vec<&String> = results.keys().collect();
    names.sort();

    // Data rows.
    for name in &names {
        let result = &results[*name];
        let status = status_display(&result.status);
        let duration = if result.status == "dep-skipped" {
            "- (dep failed)".to_string()
        } else {
            format_duration(result.duration)
        };
        buf.push_str(&format!(
            "{:<width$}   {:<12} {}\n",
            name,
            status,
            duration,
            width = max_name_len
        ));
    }

    // Show error details for failed packages.
    for name in &names {
        let result = &results[*name];
        if result.status == "failed" && (!result.stderr.is_empty() || !result.stdout.is_empty()) {
            buf.push_str(&format!("\n--- FAILED: {} ---\n", name));
            if !result.stderr.is_empty() {
                buf.push_str(&result.stderr);
                if !result.stderr.ends_with('\n') {
                    buf.push('\n');
                }
            }
            if !result.stdout.is_empty() {
                buf.push_str(&result.stdout);
                if !result.stdout.ends_with('\n') {
                    buf.push('\n');
                }
            }
        }
    }

    // Summary line — counts of each status.
    let total = results.len();
    let mut built = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut dep_skipped = 0;
    let mut would_build = 0;

    for r in results.values() {
        match r.status.as_str() {
            "built" => built += 1,
            "skipped" => skipped += 1,
            "failed" => failed += 1,
            "dep-skipped" => dep_skipped += 1,
            "would-build" => would_build += 1,
            _ => {}
        }
    }

    buf.push_str(&format!("\nTotal: {} packages", total));
    if built > 0 {
        buf.push_str(&format!(" | {} built", built));
    }
    if skipped > 0 {
        buf.push_str(&format!(" | {} skipped", skipped));
    }
    if failed > 0 {
        buf.push_str(&format!(" | {} failed", failed));
    }
    if dep_skipped > 0 {
        buf.push_str(&format!(" | {} dep-skipped", dep_skipped));
    }
    if would_build > 0 {
        buf.push_str(&format!(" | {} would-build", would_build));
    }
    buf.push('\n');

    buf
}

/// Prints the build report to the given writer (or stdout if None).
pub fn print_report(results: &HashMap<String, BuildResult>, writer: Option<&mut dyn Write>) {
    let report = format_report(results);
    match writer {
        Some(w) => {
            let _ = w.write_all(report.as_bytes());
        }
        None => {
            print!("{}", report);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(name: &str, status: &str, duration: f64) -> BuildResult {
        BuildResult {
            package_name: name.to_string(),
            status: status.to_string(),
            duration,
            stdout: String::new(),
            stderr: String::new(),
            return_code: 0,
        }
    }

    #[test]
    fn test_format_report_empty() {
        let results = HashMap::new();
        let report = format_report(&results);
        assert!(report.contains("No packages processed"));
    }

    #[test]
    fn test_format_report_basic() {
        let mut results = HashMap::new();
        results.insert(
            "python/logic-gates".to_string(),
            make_result("python/logic-gates", "built", 2.3),
        );
        results.insert(
            "python/arithmetic".to_string(),
            make_result("python/arithmetic", "skipped", 0.0),
        );

        let report = format_report(&results);
        assert!(report.contains("Build Report"));
        assert!(report.contains("BUILT"));
        assert!(report.contains("SKIPPED"));
        assert!(report.contains("2.3s"));
        assert!(report.contains("Total: 2 packages"));
    }

    #[test]
    fn test_format_report_with_failure() {
        let mut results = HashMap::new();
        results.insert(
            "python/broken".to_string(),
            BuildResult {
                package_name: "python/broken".to_string(),
                status: "failed".to_string(),
                duration: 0.5,
                stdout: "some output\n".to_string(),
                stderr: "error: something broke\n".to_string(),
                return_code: 1,
            },
        );

        let report = format_report(&results);
        assert!(report.contains("FAILED"));
        assert!(report.contains("--- FAILED: python/broken ---"));
        assert!(report.contains("error: something broke"));
        assert!(report.contains("1 failed"));
    }

    #[test]
    fn test_format_report_dep_skipped() {
        let mut results = HashMap::new();
        results.insert(
            "python/child".to_string(),
            make_result("python/child", "dep-skipped", 0.0),
        );

        let report = format_report(&results);
        assert!(report.contains("DEP-SKIP"));
        assert!(report.contains("- (dep failed)"));
        assert!(report.contains("1 dep-skipped"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "-");
        assert_eq!(format_duration(0.005), "-");
        assert_eq!(format_duration(2.345), "2.3s");
        assert_eq!(format_duration(10.0), "10.0s");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(status_display("built"), "BUILT");
        assert_eq!(status_display("failed"), "FAILED");
        assert_eq!(status_display("skipped"), "SKIPPED");
        assert_eq!(status_display("dep-skipped"), "DEP-SKIP");
        assert_eq!(status_display("would-build"), "WOULD-BUILD");
        assert_eq!(status_display("unknown-status"), "UNKNOWN");
    }

    #[test]
    fn test_print_report_to_writer() {
        let mut results = HashMap::new();
        results.insert(
            "test/pkg".to_string(),
            make_result("test/pkg", "built", 1.0),
        );

        let mut buf = Vec::new();
        print_report(&results, Some(&mut buf));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Build Report"));
        assert!(output.contains("BUILT"));
    }
}
