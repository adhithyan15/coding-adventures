//! Errors produced by the capability cage.

use std::fmt;

use crate::category::{Action, Category};

/// A capability check denied an OS operation.
///
/// Returned by [`Manifest::check`](crate::Manifest::check) and by
/// the secure-wrapper functions when the manifest does not cover
/// the requested triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityViolationError {
    pub category: Category,
    pub action: Action,
    pub target: String,
    pub message: String,
}

impl fmt::Display for CapabilityViolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CapabilityViolationError {}

/// A `(category, action)` pair that is not a meaningful combination
/// per the cage taxonomy, or a target / format that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidCombination {
    UnsupportedPair { category: Category, action: Action },
    EmptyTarget,
    InvalidTargetFormat { reason: String },
}

impl fmt::Display for InvalidCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidCombination::UnsupportedPair { category, action } => write!(
                f,
                "category {} does not support action {} per the cage taxonomy",
                category, action
            ),
            InvalidCombination::EmptyTarget => f.write_str("capability target is empty"),
            InvalidCombination::InvalidTargetFormat { reason } => {
                write!(f, "invalid capability target: {reason}")
            }
        }
    }
}

impl std::error::Error for InvalidCombination {}

/// Anything that can go wrong loading a `required_capabilities.json`.
#[derive(Debug)]
pub enum ManifestError {
    /// The JSON did not parse.
    Parse(String),
    /// The JSON parsed but did not match the expected schema.
    Schema { reason: String },
    /// The JSON contained an unsupported (category, action) pair, an
    /// empty target, or a bad target format.
    InvalidCombination(InvalidCombination),
    /// An IO error reading the manifest file.
    Io(std::io::Error),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Parse(msg) => write!(f, "manifest parse error: {msg}"),
            ManifestError::Schema { reason } => write!(f, "manifest schema error: {reason}"),
            ManifestError::InvalidCombination(inner) => {
                write!(f, "manifest validation error: {inner}")
            }
            ManifestError::Io(err) => write!(f, "manifest I/O error: {err}"),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManifestError::InvalidCombination(inner) => Some(inner),
            ManifestError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<InvalidCombination> for ManifestError {
    fn from(value: InvalidCombination) -> Self {
        ManifestError::InvalidCombination(value)
    }
}

impl From<std::io::Error> for ManifestError {
    fn from(value: std::io::Error) -> Self {
        ManifestError::Io(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_violation_error_displays_message() {
        let err = CapabilityViolationError {
            category: Category::Fs,
            action: Action::Read,
            target: "/etc/passwd".into(),
            message: "fs:read:/etc/passwd not declared".into(),
        };
        assert_eq!(format!("{err}"), "fs:read:/etc/passwd not declared");
    }

    #[test]
    fn invalid_combination_messages() {
        let pair = InvalidCombination::UnsupportedPair {
            category: Category::Fs,
            action: Action::Connect,
        };
        let s = format!("{pair}");
        assert!(s.contains("fs"));
        assert!(s.contains("connect"));

        let empty = InvalidCombination::EmptyTarget;
        assert!(format!("{empty}").contains("empty"));

        let bad = InvalidCombination::InvalidTargetFormat {
            reason: "expected host:port".into(),
        };
        assert!(format!("{bad}").contains("expected host:port"));
    }

    #[test]
    fn manifest_error_into_chain() {
        use std::error::Error;
        let err: ManifestError = InvalidCombination::EmptyTarget.into();
        assert!(matches!(err, ManifestError::InvalidCombination(_)));
        assert!(err.source().is_some());

        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err: ManifestError = io.into();
        assert!(matches!(err, ManifestError::Io(_)));
    }
}
