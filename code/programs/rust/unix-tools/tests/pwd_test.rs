//! # Integration Tests for pwd
//!
//! These tests exercise the full CLI Builder integration. We construct a
//! `Parser` with our `pwd.json` spec and various argv values, then verify
//! that the parser returns the correct result type and that the business
//! logic produces the expected output.
//!
//! ## Why We Test Through CLI Builder
//!
//! The point of CLI Builder is that developers don't write parsing code.
//! So our tests verify the *integration*: does our JSON spec, combined with
//! CLI Builder's parser, produce the right behavior? This catches spec
//! errors (wrong flag names, missing fields) as well as logic errors.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------
// During `cargo test`, the working directory is the crate root (where
// Cargo.toml lives), so pwd.json is right there.

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("pwd.json");
    path.to_string_lossy().into_owned()
}

/// Create a parser from the pwd spec file.
fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load pwd.json");
    Parser::new(spec)
}

/// Parse an argv list against the pwd spec and return the output.
fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Default behavior (no flags) returns ParseResult
// ---------------------------------------------------------------------------

#[cfg(test)]
mod default_behavior {
    use super::*;

    /// When invoked with no flags, pwd should return a Parse variant
    /// with neither "physical" nor "logical" set to true.
    #[test]
    fn no_flags_returns_parse_result() {
        match parse_argv(&["pwd"]) {
            ParserOutput::Parse(_) => {} // expected
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    /// With no flags, the "physical" flag should not be true.
    #[test]
    fn no_flags_physical_is_not_true() {
        match parse_argv(&["pwd"]) {
            ParserOutput::Parse(result) => {
                let physical = result.flags.get("physical");
                assert!(
                    physical != Some(&serde_json::json!(true)),
                    "physical should not be true when no flags given"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    /// With no flags, the "logical" flag should not be true either.
    #[test]
    fn no_flags_logical_is_not_true() {
        match parse_argv(&["pwd"]) {
            ParserOutput::Parse(result) => {
                let logical = result.flags.get("logical");
                assert!(
                    logical != Some(&serde_json::json!(true)),
                    "logical should not be true when no flags given"
                );
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: -P flag (physical path)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod physical_flag {
    use super::*;

    /// The short `-P` flag should set "physical" to true.
    #[test]
    fn short_flag() {
        match parse_argv(&["pwd", "-P"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("physical"),
                    Some(&serde_json::json!(true)),
                    "-P should set physical to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    /// The long `--physical` flag should also set "physical" to true.
    #[test]
    fn long_flag() {
        match parse_argv(&["pwd", "--physical"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("physical"),
                    Some(&serde_json::json!(true)),
                    "--physical should set physical to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: -L flag (logical path)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod logical_flag {
    use super::*;

    /// The short `-L` flag should set "logical" to true.
    #[test]
    fn short_flag() {
        match parse_argv(&["pwd", "-L"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("logical"),
                    Some(&serde_json::json!(true)),
                    "-L should set logical to true"
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    /// The long `--logical` flag should also set "logical" to true.
    #[test]
    fn long_flag() {
        match parse_argv(&["pwd", "--logical"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("logical"),
                    Some(&serde_json::json!(true)),
                    "--logical should set logical to true"
                );
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
        match parse_argv(&["pwd", "--help"]) {
            ParserOutput::Help(_) => {} // expected
            other => panic!("expected Help, got {:?}", other),
        }
    }

    /// The help text should mention the program name.
    #[test]
    fn help_text_contains_program_name() {
        match parse_argv(&["pwd", "--help"]) {
            ParserOutput::Help(help) => {
                assert!(
                    help.text.contains("pwd"),
                    "help text should contain 'pwd', got: {}",
                    help.text
                );
            }
            _ => panic!("expected Help"),
        }
    }

    /// The help text should describe what the program does.
    #[test]
    fn help_text_contains_description() {
        match parse_argv(&["pwd", "--help"]) {
            ParserOutput::Help(help) => {
                let lower = help.text.to_lowercase();
                assert!(
                    lower.contains("working directory"),
                    "help text should mention 'working directory', got: {}",
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
        match parse_argv(&["pwd", "--version"]) {
            ParserOutput::Version(_) => {} // expected
            other => panic!("expected Version, got {:?}", other),
        }
    }

    /// The version string should be "1.0.0".
    #[test]
    fn version_string() {
        match parse_argv(&["pwd", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0", "version should be 1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Unknown flags produce errors
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unknown_flags {
    use super::*;

    /// An unknown long flag should cause a parse error.
    #[test]
    fn unknown_long_flag_errors() {
        let parser = make_parser();
        let args: Vec<String> = vec!["pwd".into(), "--unknown".into()];
        assert!(
            parser.parse(&args).is_err(),
            "unknown flag --unknown should produce an error"
        );
    }

    /// An unknown short flag should cause a parse error.
    #[test]
    fn unknown_short_flag_errors() {
        let parser = make_parser();
        let args: Vec<String> = vec!["pwd".into(), "-x".into()];
        assert!(
            parser.parse(&args).is_err(),
            "unknown flag -x should produce an error"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    /// The physical pwd function should return an absolute path.
    #[test]
    fn physical_pwd_returns_absolute_path() {
        // We import from the binary's library code via the unix_tools crate.
        // Since lib.rs has pub functions, we test them here.
        let result = unix_tools::get_physical_pwd();
        assert!(result.is_ok(), "get_physical_pwd should succeed");
        let path = result.unwrap();
        assert!(
            path.starts_with('/'),
            "physical pwd should be absolute, got: {}",
            path
        );
    }

    /// The logical pwd function should return an absolute path.
    #[test]
    fn logical_pwd_returns_absolute_path() {
        let result = unix_tools::get_logical_pwd();
        assert!(result.is_ok(), "get_logical_pwd should succeed");
        let path = result.unwrap();
        assert!(
            path.starts_with('/'),
            "logical pwd should be absolute, got: {}",
            path
        );
    }

    /// When $PWD matches the real cwd, get_logical_pwd should return it.
    #[test]
    fn logical_pwd_uses_env_when_valid() {
        let real = std::env::current_dir()
            .unwrap()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Temporarily set PWD to the real cwd.
        let old_pwd = std::env::var("PWD").ok();
        std::env::set_var("PWD", &real);

        let result = unix_tools::get_logical_pwd().unwrap();
        assert_eq!(result, real, "logical pwd should match $PWD when valid");

        // Restore original $PWD.
        match old_pwd {
            Some(val) => std::env::set_var("PWD", val),
            None => std::env::remove_var("PWD"),
        }
    }
}
