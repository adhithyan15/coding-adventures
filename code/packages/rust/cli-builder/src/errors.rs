// errors.rs -- Error types for CLI Builder
// ==========================================
//
// CLI Builder can fail in two distinct phases:
//
//   1. Spec loading (load-time): the JSON spec is malformed, has duplicate IDs,
//      contains circular `requires` dependencies, etc. These errors mean the
//      developer made a mistake in the spec file. They are always fatal.
//
//   2. Parsing (runtime): the user's argv fails validation — unknown flags,
//      missing required arguments, conflicting flags, etc. We collect *all*
//      errors rather than stopping at the first one, so the user gets a
//      complete picture of what's wrong in a single run.
//
// The separation between `CliBuilderError` (top-level wrapper) and
// `ParseError` (individual argv error) mirrors this two-phase structure.

use std::fmt;

// ---------------------------------------------------------------------------
// Individual parse error
// ---------------------------------------------------------------------------

/// A single error arising from argv parsing.
///
/// Each `ParseError` is machine-readable via `error_type` and human-readable
/// via `message`. The optional `suggestion` field carries a fuzzy-matched
/// correction hint (e.g., "did you mean `--verbose`?"). The `context` field
/// records the command path at the point of error so the user knows which
/// subcommand's scope was active.
///
/// # Error types
///
/// The `error_type` string matches the snake_case identifiers in §8.2 of the
/// spec: `unknown_flag`, `missing_required_flag`, `conflicting_flags`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Snake_case category identifier (machine-readable).
    pub error_type: String,
    /// Human-readable explanation of what went wrong.
    pub message: String,
    /// Optional corrective hint, e.g. a fuzzy match suggestion.
    pub suggestion: Option<String>,
    /// The command path at the point where the error was detected.
    ///
    /// For example `["git", "commit"]` means the error occurred while
    /// processing the `git commit` subcommand scope.
    pub context: Vec<String>,
}

impl ParseError {
    /// Construct a new `ParseError` without a suggestion.
    pub fn new(error_type: impl Into<String>, message: impl Into<String>, context: Vec<String>) -> Self {
        ParseError {
            error_type: error_type.into(),
            message: message.into(),
            suggestion: None,
            context,
        }
    }

    /// Construct a new `ParseError` with a fuzzy suggestion.
    pub fn with_suggestion(
        error_type: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
        context: Vec<String>,
    ) -> Self {
        ParseError {
            error_type: error_type.into(),
            message: message.into(),
            suggestion: Some(suggestion.into()),
            context,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error_type, self.message)?;
        if let Some(ref sug) = self.suggestion {
            write!(f, " (suggestion: {})", sug)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Collected parse errors
// ---------------------------------------------------------------------------

/// A collection of one or more `ParseError` objects from a single parse run.
///
/// CLI Builder collects *all* errors in a single pass (rather than failing
/// fast) so the user sees everything wrong at once. This struct implements
/// `std::error::Error` so it can be returned from `parse()` as the `Err`
/// variant of a `Result`.
///
/// # Example
///
/// ```
/// # use cli_builder::errors::{ParseError, ParseErrors};
/// let errs = ParseErrors {
///     errors: vec![
///         ParseError::new("missing_required_flag", "--output is required", vec!["cp".into()]),
///     ],
/// };
/// println!("{}", errs);
/// ```
#[derive(Debug)]
pub struct ParseErrors {
    /// All errors collected during parsing.
    pub errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Create a `ParseErrors` from a single error.
    pub fn single(e: ParseError) -> Self {
        ParseErrors { errors: vec![e] }
    }
}

impl fmt::Display for ParseErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print a numbered list of all errors so the user can address them all.
        for (i, e) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "error {}: {}", i + 1, e)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseErrors {}

// ---------------------------------------------------------------------------
// Top-level library error
// ---------------------------------------------------------------------------

/// Top-level error type for CLI Builder.
///
/// Wraps the two categories of failure:
///
/// - `SpecError`: the JSON spec itself is invalid (spec load time).
/// - `ParseErrors`: one or more argv errors (runtime).
/// - `IoError`: file I/O failed (when reading a spec file from disk).
/// - `JsonError`: `serde_json` failed to parse the spec JSON.
#[derive(Debug)]
pub enum CliBuilderError {
    /// The JSON spec is semantically invalid (cycle in requires, duplicate IDs, etc.).
    ///
    /// This is a developer error — the spec must be fixed before parsing can proceed.
    SpecError(String),

    /// One or more argv errors were collected during parsing.
    ///
    /// Contains all errors found in a single pass.
    ParseErrors(ParseErrors),

    /// File I/O failed while reading a spec file.
    IoError(std::io::Error),

    /// JSON deserialization of the spec failed.
    JsonError(String),
}

impl fmt::Display for CliBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliBuilderError::SpecError(msg) => write!(f, "spec error: {}", msg),
            CliBuilderError::ParseErrors(errs) => write!(f, "parse errors:\n{}", errs),
            CliBuilderError::IoError(e) => write!(f, "IO error: {}", e),
            CliBuilderError::JsonError(msg) => write!(f, "JSON error: {}", msg),
        }
    }
}

impl std::error::Error for CliBuilderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliBuilderError::IoError(e) => Some(e),
            CliBuilderError::ParseErrors(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CliBuilderError {
    fn from(e: std::io::Error) -> Self {
        CliBuilderError::IoError(e)
    }
}

impl From<serde_json::Error> for CliBuilderError {
    fn from(e: serde_json::Error) -> Self {
        CliBuilderError::JsonError(e.to_string())
    }
}

impl From<ParseErrors> for CliBuilderError {
    fn from(e: ParseErrors) -> Self {
        CliBuilderError::ParseErrors(e)
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ParseError construction and Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_error_new_no_suggestion() {
        let e = ParseError::new("missing_required_flag", "--output is required", vec!["prog".into()]);
        assert_eq!(e.error_type, "missing_required_flag");
        assert_eq!(e.message, "--output is required");
        assert!(e.suggestion.is_none());
        assert_eq!(e.context, vec!["prog".to_string()]);
    }

    #[test]
    fn test_parse_error_with_suggestion() {
        let e = ParseError::with_suggestion(
            "unknown_flag",
            "Unknown flag '--verbos'",
            "--verbose",
            vec!["prog".into()],
        );
        assert_eq!(e.suggestion, Some("--verbose".to_string()));
    }

    #[test]
    fn test_parse_error_display_no_suggestion() {
        let e = ParseError::new("missing_required_flag", "flag is required", vec![]);
        let s = format!("{}", e);
        assert!(s.contains("missing_required_flag"));
        assert!(s.contains("flag is required"));
        assert!(!s.contains("suggestion"));
    }

    #[test]
    fn test_parse_error_display_with_suggestion() {
        let e = ParseError::with_suggestion(
            "unknown_flag",
            "Unknown flag '--verbo'",
            "--verbose",
            vec![],
        );
        let s = format!("{}", e);
        assert!(s.contains("suggestion"));
        assert!(s.contains("--verbose"));
    }

    #[test]
    fn test_parse_error_equality() {
        let a = ParseError::new("missing_required_flag", "msg", vec!["prog".into()]);
        let b = ParseError::new("missing_required_flag", "msg", vec!["prog".into()]);
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // ParseErrors construction and Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_errors_single() {
        let e = ParseErrors::single(ParseError::new("err", "msg", vec![]));
        assert_eq!(e.errors.len(), 1);
    }

    #[test]
    fn test_parse_errors_display_single() {
        let errs = ParseErrors {
            errors: vec![ParseError::new("missing_required_flag", "--output is required", vec!["cp".into()])],
        };
        let s = format!("{}", errs);
        assert!(s.contains("error 1:"));
        assert!(s.contains("missing_required_flag"));
    }

    #[test]
    fn test_parse_errors_display_multiple() {
        let errs = ParseErrors {
            errors: vec![
                ParseError::new("err_a", "first error", vec![]),
                ParseError::new("err_b", "second error", vec![]),
            ],
        };
        let s = format!("{}", errs);
        assert!(s.contains("error 1:"));
        assert!(s.contains("error 2:"));
        assert!(s.contains("first error"));
        assert!(s.contains("second error"));
    }

    // -----------------------------------------------------------------------
    // CliBuilderError Display variants
    // -----------------------------------------------------------------------

    #[test]
    fn test_cli_builder_error_spec_error_display() {
        let e = CliBuilderError::SpecError("duplicate id 'verbose'".into());
        let s = format!("{}", e);
        assert!(s.contains("spec error"));
        assert!(s.contains("duplicate id"));
    }

    #[test]
    fn test_cli_builder_error_json_error_display() {
        let e = CliBuilderError::JsonError("unexpected token at line 1".into());
        let s = format!("{}", e);
        assert!(s.contains("JSON error"));
    }

    #[test]
    fn test_cli_builder_error_parse_errors_display() {
        let errs = ParseErrors {
            errors: vec![ParseError::new("unknown_flag", "--foo unknown", vec![])],
        };
        let e = CliBuilderError::ParseErrors(errs);
        let s = format!("{}", e);
        assert!(s.contains("parse errors"));
    }

    #[test]
    fn test_cli_builder_error_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let e = CliBuilderError::IoError(io_err);
        let s = format!("{}", e);
        assert!(s.contains("IO error"));
    }

    // -----------------------------------------------------------------------
    // CliBuilderError::source() coverage
    // -----------------------------------------------------------------------

    #[test]
    fn test_cli_builder_error_source_io() {
        use std::error::Error;
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e = CliBuilderError::IoError(io_err);
        assert!(e.source().is_some());
    }

    #[test]
    fn test_cli_builder_error_source_parse_errors() {
        use std::error::Error;
        let errs = ParseErrors { errors: vec![] };
        let e = CliBuilderError::ParseErrors(errs);
        assert!(e.source().is_some());
    }

    #[test]
    fn test_cli_builder_error_source_spec_error_is_none() {
        use std::error::Error;
        let e = CliBuilderError::SpecError("bad spec".into());
        assert!(e.source().is_none());
    }

    #[test]
    fn test_cli_builder_error_source_json_error_is_none() {
        use std::error::Error;
        let e = CliBuilderError::JsonError("bad json".into());
        assert!(e.source().is_none());
    }

    // -----------------------------------------------------------------------
    // From conversions
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_parse_errors() {
        let errs = ParseErrors { errors: vec![] };
        let e: CliBuilderError = errs.into();
        assert!(matches!(e, CliBuilderError::ParseErrors(_)));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let e: CliBuilderError = io_err.into();
        assert!(matches!(e, CliBuilderError::IoError(_)));
    }
}
