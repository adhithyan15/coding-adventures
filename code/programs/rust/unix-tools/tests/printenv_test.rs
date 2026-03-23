//! # Integration Tests for printenv
//!
//! These tests exercise both the CLI Builder integration and the printenv
//! business logic. We test specific variable lookup, missing variables,
//! NUL termination, and the "print all" mode.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::printenv_tool::get_env_vars;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper: locate the spec file
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir).join("printenv.json");
    path.to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load printenv.json");
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
        assert!(spec.is_ok(), "printenv.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: Flag parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn no_flags_returns_parse() {
        match parse_argv(&["printenv"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn null_flag() {
        match parse_argv(&["printenv", "-0"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(
                    result.flags.get("null"),
                    Some(&serde_json::json!(true))
                );
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["printenv", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["printenv", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — get_env_vars
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn specific_variable() {
        std::env::set_var("TEST_PRINTENV_INT_A", "hello");
        let result = get_env_vars(&["TEST_PRINTENV_INT_A".into()], false);
        assert_eq!(result, "hello\n");
        std::env::remove_var("TEST_PRINTENV_INT_A");
    }

    #[test]
    fn missing_variable() {
        let result = get_env_vars(&["NONEXISTENT_VAR_XYZ_12345".into()], false);
        assert_eq!(result, "");
    }

    #[test]
    fn null_terminated() {
        std::env::set_var("TEST_PRINTENV_INT_B", "value");
        let result = get_env_vars(&["TEST_PRINTENV_INT_B".into()], true);
        assert_eq!(result, "value\0");
        std::env::remove_var("TEST_PRINTENV_INT_B");
    }

    #[test]
    fn multiple_variables() {
        std::env::set_var("TEST_PRINTENV_INT_C", "alpha");
        std::env::set_var("TEST_PRINTENV_INT_D", "beta");
        let result = get_env_vars(
            &["TEST_PRINTENV_INT_C".into(), "TEST_PRINTENV_INT_D".into()],
            false,
        );
        assert_eq!(result, "alpha\nbeta\n");
        std::env::remove_var("TEST_PRINTENV_INT_C");
        std::env::remove_var("TEST_PRINTENV_INT_D");
    }

    #[test]
    fn all_variables_not_empty() {
        let result = get_env_vars(&[], false);
        assert!(!result.is_empty());
    }

    #[test]
    fn all_variables_contains_known() {
        std::env::set_var("TEST_PRINTENV_INT_E", "present");
        let result = get_env_vars(&[], false);
        assert!(result.contains("TEST_PRINTENV_INT_E=present"));
        std::env::remove_var("TEST_PRINTENV_INT_E");
    }
}
