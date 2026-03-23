//! # Integration Tests for true
//!
//! The `true` command is the simplest possible Unix tool: it does nothing
//! and exits with status 0. Despite its simplicity, we still need to
//! verify that the CLI Builder spec is valid and that the parser handles
//! `--help` and `--version` correctly.
//!
//! There is no "business logic" to test — the entire purpose of `true`
//! is to succeed. The value of these tests is confirming that the JSON
//! spec is well-formed and integrates properly with CLI Builder.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("true.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load true.json");
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

    /// The true.json spec should load without errors.
    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "true.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Default behavior (no flags)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    /// When invoked with no flags, true should return a Parse variant.
    /// The program would then simply exit with code 0.
    #[test]
    fn no_flags_returns_parse_result() {
        match parse_argv(&["true"]) {
            ParserOutput::Parse(_) => {} // expected
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    /// The parse result should have no user-defined flags set to true.
    /// (CLI Builder may include builtin flag entries with default values.)
    #[test]
    fn no_user_flags_set_to_true() {
        match parse_argv(&["true"]) {
            ParserOutput::Parse(result) => {
                // true has no user-defined flags, so none should be true
                for (key, value) in &result.flags {
                    assert_ne!(
                        value,
                        &serde_json::json!(true),
                        "flag '{}' should not be true",
                        key
                    );
                }
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --help flag
// ---------------------------------------------------------------------------

#[cfg(test)]
mod help_flag {
    use super::*;

    /// `--help` should return a Help variant.
    #[test]
    fn help_returns_help_result() {
        match parse_argv(&["true", "--help"]) {
            ParserOutput::Help(_) => {} // expected
            other => panic!("expected Help, got {:?}", other),
        }
    }

    /// The help text should mention the program name.
    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["true", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("true"),
                    "help text should contain 'true', got: {}",
                    help.text
                );
            }
            _ => panic!("expected Help"),
        }
    }

    /// The help text should describe what the program does.
    #[test]
    fn help_text_contains_description() {
        match parse_argv(&["true", "--help"]) {
            ParserOutput::Help(help) => {
                let lower = help.text.to_lowercase();
                assert!(
                    lower.contains("successfully") || lower.contains("nothing"),
                    "help text should mention 'successfully' or 'nothing', got: {}",
                    help.text
                );
            }
            _ => panic!("expected Help"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: --version flag
// ---------------------------------------------------------------------------

#[cfg(test)]
mod version_flag {
    use super::*;

    /// `--version` should return a Version variant.
    #[test]
    fn version_returns_version_result() {
        match parse_argv(&["true", "--version"]) {
            ParserOutput::Version(_) => {} // expected
            other => panic!("expected Version, got {:?}", other),
        }
    }

    /// The version string should be "1.0.0".
    #[test]
    fn version_string() {
        match parse_argv(&["true", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0", "version should be 1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: true ignores extra arguments
// ---------------------------------------------------------------------------

#[cfg(test)]
mod extra_arguments {
    use super::*;

    /// GNU true ignores any extra arguments. Our spec has no arguments
    /// defined, so extra args may produce an error depending on
    /// CLI Builder's behavior. We just verify the spec works for
    /// the no-argument case.
    #[test]
    fn no_arguments_succeeds() {
        match parse_argv(&["true"]) {
            ParserOutput::Parse(_) => {} // expected
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}
