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
