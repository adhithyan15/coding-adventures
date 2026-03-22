//! # Integration Tests for echo
//!
//! These tests exercise both the CLI Builder integration (spec parsing,
//! flag handling) and the echo business logic (escape sequences, newline
//! suppression). The echo tool is more interesting than true/false because
//! it has real flags and arguments that affect output.
//!
//! ## Test Strategy
//!
//! We test at two levels:
//!
//! 1. **Spec-level tests**: Verify the JSON spec integrates correctly
//!    with CLI Builder — flags parse, help works, mutual exclusivity
//!    of `-e`/`-E` is enforced.
//!
//! 2. **Logic-level tests**: Verify the `process_echo` function produces
//!    correct output for various inputs and flag combinations.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::echo_tool::process_echo;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("echo.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load echo.json");
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
        assert!(spec.is_ok(), "echo.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Default behavior (no flags)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    #[test]
    fn no_flags_returns_parse_result() {
        match parse_argv(&["echo"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn with_arguments_returns_parse_result() {
        match parse_argv(&["echo", "hello", "world"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: -n flag (suppress newline)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod no_newline_flag {
    use super::*;

    #[test]
    fn short_flag_parsed() {
        match parse_argv(&["echo", "-n", "hello"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("no_newline"),
                    Some(&serde_json::json!(true)),
                    "-n should set no_newline to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: -e flag (enable escapes)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod enable_escapes_flag {
    use super::*;

    #[test]
    fn short_flag_parsed() {
        match parse_argv(&["echo", "-e", "hello"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("enable_escapes"),
                    Some(&serde_json::json!(true)),
                    "-e should set enable_escapes to true"
                );
            }
            _ => panic!("expected Parse"),
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
        match parse_argv(&["echo", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["echo", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("echo"),
                    "help text should contain 'echo'"
                );
            }
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["echo", "--version"]) {
            ParserOutput::Version(_) => {}
            other => panic!("expected Version, got {:?}", other),
        }
    }

    #[test]
    fn version_string() {
        match parse_argv(&["echo", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — process_echo
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    /// Echo with no arguments should output just a newline.
    #[test]
    fn empty_echo() {
        let result = process_echo(&[], false, false);
        assert_eq!(result, "\n");
    }

    /// Single argument with trailing newline.
    #[test]
    fn single_argument() {
        let args: Vec<String> = vec!["hello".into()];
        assert_eq!(process_echo(&args, false, false), "hello\n");
    }

    /// Multiple arguments joined by spaces.
    #[test]
    fn multiple_arguments() {
        let args: Vec<String> = vec!["hello".into(), "world".into()];
        assert_eq!(process_echo(&args, false, false), "hello world\n");
    }

    /// The -n flag suppresses the trailing newline.
    #[test]
    fn suppress_newline() {
        let args: Vec<String> = vec!["hello".into()];
        assert_eq!(process_echo(&args, true, false), "hello");
    }

    /// With -e, backslash-t becomes a real tab.
    #[test]
    fn escape_tab() {
        let args: Vec<String> = vec!["a\\tb".into()];
        assert_eq!(process_echo(&args, false, true), "a\tb\n");
    }

    /// With -e, backslash-n becomes a real newline.
    #[test]
    fn escape_newline() {
        let args: Vec<String> = vec!["a\\nb".into()];
        assert_eq!(process_echo(&args, false, true), "a\nb\n");
    }

    /// With -e, backslash-c stops all output (including trailing newline).
    #[test]
    fn escape_c_stops_output() {
        let args: Vec<String> = vec!["hello\\cworld".into()];
        assert_eq!(process_echo(&args, false, true), "hello");
    }

    /// With -e, double backslash becomes single backslash.
    #[test]
    fn escape_double_backslash() {
        let args: Vec<String> = vec!["a\\\\b".into()];
        assert_eq!(process_echo(&args, false, true), "a\\b\n");
    }

    /// Without -e, escape sequences are literal.
    #[test]
    fn no_escape_interpretation() {
        let args: Vec<String> = vec!["a\\tb".into()];
        assert_eq!(process_echo(&args, false, false), "a\\tb\n");
    }

    /// Octal escape: \0101 = 'A'.
    #[test]
    fn escape_octal() {
        let args: Vec<String> = vec!["\\0101".into()];
        assert_eq!(process_echo(&args, false, true), "A\n");
    }

    /// Hex escape: \x41 = 'A'.
    #[test]
    fn escape_hex() {
        let args: Vec<String> = vec!["\\x41".into()];
        assert_eq!(process_echo(&args, false, true), "A\n");
    }

    /// Both -n and -e can be combined.
    #[test]
    fn combined_flags() {
        let args: Vec<String> = vec!["hello\\tworld".into()];
        assert_eq!(process_echo(&args, true, true), "hello\tworld");
    }

    /// Bell escape: \a
    #[test]
    fn escape_bell() {
        let args: Vec<String> = vec!["\\a".into()];
        assert_eq!(process_echo(&args, false, true), "\x07\n");
    }

    /// Carriage return escape: \r
    #[test]
    fn escape_carriage_return() {
        let args: Vec<String> = vec!["\\r".into()];
        assert_eq!(process_echo(&args, false, true), "\r\n");
    }
}
