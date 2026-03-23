//! # Integration Tests for sleep
//!
//! These tests verify that the `sleep` JSON spec integrates correctly
//! with CLI Builder, and that the duration parsing logic handles all
//! valid formats and rejects invalid input.
//!
//! ## Note on Testing
//!
//! We do NOT actually sleep in tests — we only test the parsing logic.
//! The `parse_duration` and `parse_durations` functions convert strings
//! into `Duration` values, which is the interesting part. The actual
//! sleeping is a trivial `std::thread::sleep` call.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::sleep_tool::{parse_duration, parse_durations};
use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("sleep.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load sleep.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads successfully
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "sleep.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Default behavior
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    #[test]
    fn with_argument_returns_parse_result() {
        match parse_argv(&["sleep", "1"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn multiple_arguments_returns_parse_result() {
        match parse_argv(&["sleep", "1m", "30s"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --help and --version
// ---------------------------------------------------------------------------

#[cfg(test)]
mod builtins {
    use super::*;

    #[test]
    fn help_returns_help_result() {
        match parse_argv(&["sleep", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["sleep", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("sleep"),
                    "help text should contain 'sleep'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["sleep", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["sleep", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — parse_duration
// ---------------------------------------------------------------------------

#[cfg(test)]
mod duration_parsing {
    use super::*;

    #[test]
    fn plain_seconds() {
        assert_eq!(parse_duration("5").unwrap(), Duration::from_secs(5));
    }

    #[test]
    fn seconds_suffix() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
    }

    #[test]
    fn minutes() {
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    }

    #[test]
    fn hours() {
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
    }

    #[test]
    fn days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn fractional_seconds() {
        assert_eq!(parse_duration("0.5").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn fractional_minutes() {
        assert_eq!(parse_duration("1.5m").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn zero() {
        assert_eq!(parse_duration("0").unwrap(), Duration::ZERO);
    }

    #[test]
    fn zero_with_suffix() {
        assert_eq!(parse_duration("0s").unwrap(), Duration::ZERO);
    }
}

// ---------------------------------------------------------------------------
// Test: Multiple durations
// ---------------------------------------------------------------------------

#[cfg(test)]
mod multiple_durations {
    use super::*;

    #[test]
    fn sum_minutes_and_seconds() {
        let args: Vec<String> = vec!["1m".into(), "30s".into()];
        assert_eq!(parse_durations(&args).unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn sum_mixed_units() {
        let args: Vec<String> = vec!["1h".into(), "30m".into(), "15s".into()];
        assert_eq!(
            parse_durations(&args).unwrap(),
            Duration::from_secs(3600 + 1800 + 15)
        );
    }

    #[test]
    fn single_argument() {
        let args: Vec<String> = vec!["10".into()];
        assert_eq!(parse_durations(&args).unwrap(), Duration::from_secs(10));
    }
}

// ---------------------------------------------------------------------------
// Test: Error cases
// ---------------------------------------------------------------------------

#[cfg(test)]
mod error_cases {
    use super::*;

    #[test]
    fn empty_args() {
        let args: Vec<String> = vec![];
        assert!(parse_durations(&args).is_err());
    }

    #[test]
    fn invalid_string() {
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn invalid_suffix() {
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn empty_string() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn negative_value() {
        assert!(parse_duration("-5").is_err());
    }

    #[test]
    fn one_bad_in_list_fails_all() {
        let args: Vec<String> = vec!["1m".into(), "bad".into()];
        assert!(parse_durations(&args).is_err());
    }
}
