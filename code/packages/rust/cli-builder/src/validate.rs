//! # Spec Validation
//!
//! Standalone validation of CLI Builder JSON specs.
//!
//! ## Why a separate module?
//!
//! The [`spec_loader`](crate::spec_loader) module combines two concerns:
//! parsing JSON into typed structs, and validating those structs. It returns
//! a `Result<CliSpec, CliBuilderError>`, which is perfect for loading — but
//! sometimes you just want to *check* a spec without needing the parsed value.
//!
//! Use cases for standalone validation:
//!
//! - **CI linting**: validate all JSON spec files in a directory, collecting
//!   errors without halting on the first one.
//! - **Editor integration**: show validation errors inline as the user edits
//!   a spec file.
//! - **Dry-run mode**: confirm a spec is valid before wiring it into a CLI.
//!
//! ## How it works
//!
//! Under the hood, validation delegates to the same
//! [`load_spec_from_str`](crate::spec_loader::load_spec_from_str) and
//! [`load_spec_from_file`](crate::spec_loader::load_spec_from_file)
//! functions used by the spec loader. The difference is purely in the return
//! type: instead of `Result<CliSpec, CliBuilderError>`, you get a
//! [`ValidationResult`] with a boolean `valid` flag and a list of error
//! messages.
//!
//! ```text
//! ┌─────────────┐     ┌───────────────────┐     ┌────────────────────┐
//! │  JSON string │────►│  load_spec_from_*  │────►│  ValidationResult  │
//! │  or file     │     │  (parse + validate)│     │  { valid, errors } │
//! └─────────────┘     └───────────────────┘     └────────────────────┘
//! ```

use crate::spec_loader;

// ---------------------------------------------------------------------------
// ValidationResult
// ---------------------------------------------------------------------------

/// The result of validating a CLI Builder spec.
///
/// A valid spec produces `ValidationResult { valid: true, errors: [] }`.
/// An invalid spec produces `ValidationResult { valid: false, errors: [...] }`,
/// where each error string describes one problem found in the spec.
///
/// ## Example
///
/// ```
/// use cli_builder::validate::validate_spec_str;
///
/// let result = validate_spec_str(r#"{
///     "cli_builder_spec_version": "1.0",
///     "name": "hello",
///     "description": "Say hello"
/// }"#);
/// assert!(result.valid);
/// assert!(result.errors.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// `true` if the spec passed all validation checks.
    pub valid: bool,
    /// A list of human-readable error descriptions. Empty when `valid` is true.
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result (no errors).
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: vec![],
        }
    }

    /// Create a failed validation result with the given error messages.
    pub fn err(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a CLI Builder JSON spec string.
///
/// Parses the JSON and runs all semantic validation checks (version, duplicate
/// IDs, flag forms, cross-references, cycle detection, etc.). Returns a
/// [`ValidationResult`] instead of a `Result`, making it convenient for
/// linting and CI workflows where you want to inspect errors without
/// pattern-matching on error types.
///
/// ## Example
///
/// ```
/// use cli_builder::validate::validate_spec_str;
///
/// // Valid spec:
/// let ok = validate_spec_str(r#"{
///     "cli_builder_spec_version": "1.0",
///     "name": "echo",
///     "description": "Print text"
/// }"#);
/// assert!(ok.valid);
///
/// // Invalid spec (unsupported version):
/// let bad = validate_spec_str(r#"{
///     "cli_builder_spec_version": "99.0",
///     "name": "echo",
///     "description": "Print text"
/// }"#);
/// assert!(!bad.valid);
/// assert!(bad.errors[0].contains("99.0"));
/// ```
pub fn validate_spec_str(json: &str) -> ValidationResult {
    match spec_loader::load_spec_from_str(json) {
        Ok(_) => ValidationResult::ok(),
        Err(e) => ValidationResult::err(vec![e.to_string()]),
    }
}

/// Validate a CLI Builder JSON spec file on disk.
///
/// Reads the file and delegates to [`validate_spec_str`]. Returns a
/// [`ValidationResult`] that captures IO errors (file not found, permission
/// denied) as well as JSON parse errors and semantic validation errors.
///
/// ## Example
///
/// ```
/// use cli_builder::validate::validate_spec_file;
///
/// // Use a path that doesn't exist on any platform.
/// let result = validate_spec_file("nonexistent_dir_xyz/no_such_file.json");
/// assert!(!result.valid);
/// assert!(!result.errors[0].is_empty());
/// ```
pub fn validate_spec_file(path: &str) -> ValidationResult {
    match spec_loader::load_spec_from_file(path) {
        Ok(_) => ValidationResult::ok(),
        Err(e) => ValidationResult::err(vec![e.to_string()]),
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================
//
// These tests mirror the spec_loader tests but verify the ValidationResult
// interface rather than the Result<CliSpec, CliBuilderError> interface.

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Valid spec
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_minimal_spec() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "echo",
            "description": "Print text"
        }"#,
        );
        assert!(result.valid, "expected valid, got errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_valid_spec_with_flags_and_args() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "ls",
            "description": "List directory contents",
            "flags": [
                {"id":"long","short":"l","description":"Long listing","type":"boolean"},
                {"id":"all","short":"a","long":"all","description":"Show hidden","type":"boolean"}
            ],
            "arguments": [
                {"id":"path","name":"PATH","description":"Directory","type":"path","required":false}
            ]
        }"#,
        );
        assert!(result.valid, "expected valid, got errors: {:?}", result.errors);
    }

    // -----------------------------------------------------------------------
    // Missing version (serde requires the field, so this is a JSON error)
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_version_field() {
        let result = validate_spec_str(
            r#"{
            "name": "echo",
            "description": "Print text"
        }"#,
        );
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        // serde will report the missing field
        assert!(
            result.errors[0].contains("cli_builder_spec_version"),
            "expected error about missing version field, got: {}",
            result.errors[0]
        );
    }

    // -----------------------------------------------------------------------
    // Unsupported version
    // -----------------------------------------------------------------------

    #[test]
    fn test_unsupported_version() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "99.0",
            "name": "echo",
            "description": "Print text"
        }"#,
        );
        assert!(!result.valid);
        assert!(
            result.errors[0].contains("99.0"),
            "expected error mentioning '99.0', got: {}",
            result.errors[0]
        );
    }

    // -----------------------------------------------------------------------
    // Missing required fields (name, description)
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_name_field() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "description": "Print text"
        }"#,
        );
        assert!(!result.valid);
        assert!(
            result.errors[0].contains("name"),
            "expected error about missing 'name', got: {}",
            result.errors[0]
        );
    }

    #[test]
    fn test_missing_description_field() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "echo"
        }"#,
        );
        assert!(!result.valid);
        assert!(
            result.errors[0].contains("description"),
            "expected error about missing 'description', got: {}",
            result.errors[0]
        );
    }

    // -----------------------------------------------------------------------
    // Invalid JSON
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_json() {
        let result = validate_spec_str("{not valid json at all");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_empty_string() {
        let result = validate_spec_str("");
        assert!(!result.valid);
    }

    // -----------------------------------------------------------------------
    // Nonexistent file
    // -----------------------------------------------------------------------

    #[test]
    fn test_nonexistent_file() {
        // Use a path that works on both Unix (/tmp/...) and Windows.
        // On Windows, /tmp doesn't exist, so we use a clearly-nonexistent
        // path under a drive letter instead.
        let path = if cfg!(windows) {
            "C:\\nonexistent_dir_xyz\\no_such_file.json"
        } else {
            "/tmp/cli_builder_validate_no_such_file_xyz.json"
        };
        let result = validate_spec_file(path);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        // Error messages differ across platforms:
        //   Unix:    "No such file or directory"
        //   Windows: "The system cannot find the path specified"
        // Just check that we got a non-empty error — the important thing
        // is that the function didn't panic.
        assert!(
            !result.errors[0].is_empty(),
            "expected a non-empty error message for nonexistent file"
        );
    }

    // -----------------------------------------------------------------------
    // Flag with no short/long (no usable form)
    // -----------------------------------------------------------------------

    #[test]
    fn test_flag_with_no_form() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"bad","description":"no form at all","type":"boolean"}
            ]
        }"#,
        );
        assert!(!result.valid);
        assert!(
            result.errors[0].contains("short") || result.errors[0].contains("long"),
            "expected error about missing flag form, got: {}",
            result.errors[0]
        );
    }

    // -----------------------------------------------------------------------
    // ValidationResult constructors
    // -----------------------------------------------------------------------

    #[test]
    fn test_validation_result_ok() {
        let r = ValidationResult::ok();
        assert!(r.valid);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn test_validation_result_err() {
        let r = ValidationResult::err(vec!["something broke".to_string()]);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 1);
        assert_eq!(r.errors[0], "something broke");
    }

    #[test]
    fn test_validation_result_clone() {
        let r = ValidationResult::err(vec!["oops".into()]);
        let r2 = r.clone();
        assert_eq!(r.valid, r2.valid);
        assert_eq!(r.errors, r2.errors);
    }

    #[test]
    fn test_validation_result_debug() {
        let r = ValidationResult::ok();
        let debug = format!("{:?}", r);
        assert!(debug.contains("valid"));
    }

    // -----------------------------------------------------------------------
    // Duplicate flag IDs
    // -----------------------------------------------------------------------

    #[test]
    fn test_duplicate_flag_ids() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"verbose","short":"v","description":"verbose","type":"boolean"},
                {"id":"verbose","short":"q","description":"quiet","type":"boolean"}
            ]
        }"#,
        );
        assert!(!result.valid);
        assert!(result.errors[0].contains("duplicate"));
    }

    // -----------------------------------------------------------------------
    // Circular requires dependency
    // -----------------------------------------------------------------------

    #[test]
    fn test_circular_requires() {
        let result = validate_spec_str(
            r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"a","short":"a","description":"a","type":"boolean","requires":["b"]},
                {"id":"b","short":"b","description":"b","type":"boolean","requires":["a"]}
            ]
        }"#,
        );
        assert!(!result.valid);
        assert!(
            result.errors[0].contains("circular") || result.errors[0].contains("cycle"),
            "expected cycle error, got: {}",
            result.errors[0]
        );
    }
}
