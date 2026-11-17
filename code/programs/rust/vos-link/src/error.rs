//! # Findings and "no verdict"
//!
//! Two different kinds of failure, kept apart because the exit code differs and a CI
//! reader must be able to tell them apart:
//!
//! * a [`Finding`] is a rule that failed on bytes the tool was able to read and admit
//!   (a forged marker, a drifted document, a weakened claim).  Exit 1.
//! * a [`NoVerdict`] is "I could not even apply the rules": a missing path, a symlink,
//!   a file over its cap, a path outside the repository.  Exit 2.

use std::fmt;

/// One rule failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The rule: `GD01`..`GD05` (inputs) or `CD01`..`CD08` (the document).
    pub code: &'static str,
    /// What is wrong, naming the file and line where there is one.
    pub message: String,
}

impl Finding {
    /// A finding.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Finding {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code, self.message)
    }
}

/// The tool could not reach a verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoVerdict(pub String);

impl NoVerdict {
    /// A no-verdict with a reason.
    pub fn new(reason: impl Into<String>) -> Self {
        NoVerdict(reason.into())
    }
}

impl fmt::Display for NoVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NoVerdict {}
