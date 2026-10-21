//! # Every way a release-path Python compile can fail (LANG84 C6)
//!
//! One enum, six categories. Every variant that comes after release
//! resolution names the release it ran against:
//!
//! | Variant               | Raised by                    | Carries                         |
//! |-----------------------|------------------------------|---------------------------------|
//! | `UnknownRelease`      | release resolution           | the (escaped) request, ≤5 IDs   |
//! | `ReleaseNotAvailable` | resolution / profile lookup  | release, why                    |
//! | `Budget`              | parser or compiler budgets   | release, which budget, limit    |
//! | `Lex`                 | `GrammarLexer`               | release, 1-based line/column    |
//! | `Parse`               | `GrammarParser`              | release, furthest failure       |
//! | `Unsupported`         | this crate                   | release, CST rule, start/end    |
//!
//! The first five arrive from `python-parser` as a `PythonFrontendError` and
//! are carried over field for field ([`From`] below). `Unsupported` is the
//! compiler's own: the tree parsed, but it is outside what `py-int-print`
//! lowers (a callee other than `print`, `/`, a literal outside `i64`, ...).
//!
//! LANG79's float pilot keeps its own `String` errors; this type belongs to
//! the release path only.

use std::fmt;

use coding_adventures_python_parser::PythonFrontendError;

/// A failed release-path compile. See the module docs for the categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    /// The requested ID is not a row of `code/grammars/python/releases.csv`.
    UnknownRelease {
        /// The request, truncated and with control characters escaped.
        requested: String,
        /// Up to five inventory IDs sharing the longest prefix with it.
        suggestions: Vec<String>,
    },
    /// The release is in the inventory but cannot compile this subset: it has
    /// no grammar pair yet, or no IIR profile (its `iir_status` is `-`).
    ReleaseNotAvailable {
        /// The inventory ID.
        release: String,
        /// Why, e.g. the row's `grammar_stage` or `"no IIR profile"`.
        stage: String,
    },
    /// A LANG84 C5 budget was exceeded.
    Budget {
        /// The release the compile ran against.
        release: String,
        /// Which budget.
        which: &'static str,
        /// Its limit.
        limit: usize,
    },
    /// No token pattern of the release's `.tokens` matches at this position.
    Lex {
        /// The release the compile ran against.
        release: String,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
        /// The lexer's message.
        message: String,
    },
    /// The release's `.grammar` rejects the token stream.
    Parse {
        /// The release the compile ran against.
        release: String,
        /// 1-based line of the furthest failure.
        line: usize,
        /// 1-based column of the furthest failure.
        column: usize,
        /// The parser's message, listing the expected terminals.
        message: String,
    },
    /// The tree parsed but lies outside the subset this compiler lowers.
    Unsupported {
        /// The release the compile ran against.
        release: String,
        /// The CST rule being lowered when the compiler refused.
        rule: String,
        /// The node's 1-based start `(line, column)`, when the CST has it.
        start: Option<(usize, usize)>,
        /// The node's 1-based end `(line, column)`, when the CST has it.
        end: Option<(usize, usize)>,
        /// What was refused.
        message: String,
    },
}

impl FrontendError {
    /// The release the failure belongs to, when one was resolved.
    pub fn release(&self) -> Option<&str> {
        match self {
            Self::UnknownRelease { .. } => None,
            Self::ReleaseNotAvailable { release, .. }
            | Self::Budget { release, .. }
            | Self::Lex { release, .. }
            | Self::Parse { release, .. }
            | Self::Unsupported { release, .. } => Some(release),
        }
    }

    /// The LANG84 C6 category name (`"Lex"`, `"Parse"`, `"Unsupported"`, …),
    /// which is also what a corpus `.reject` file names.
    pub fn category(&self) -> &'static str {
        match self {
            Self::UnknownRelease { .. } => "UnknownRelease",
            Self::ReleaseNotAvailable { .. } => "ReleaseNotAvailable",
            Self::Budget { .. } => "Budget",
            Self::Lex { .. } => "Lex",
            Self::Parse { .. } => "Parse",
            Self::Unsupported { .. } => "Unsupported",
        }
    }
}

impl From<PythonFrontendError> for FrontendError {
    fn from(error: PythonFrontendError) -> Self {
        match error {
            PythonFrontendError::UnknownRelease {
                requested,
                suggestions,
            } => Self::UnknownRelease {
                requested,
                suggestions,
            },
            PythonFrontendError::ReleaseNotAvailable { release, stage } => {
                Self::ReleaseNotAvailable { release, stage }
            }
            PythonFrontendError::Budget {
                release,
                which,
                limit,
            } => Self::Budget {
                release,
                which,
                limit,
            },
            PythonFrontendError::Lex {
                release,
                line,
                column,
                message,
            } => Self::Lex {
                release,
                line,
                column,
                message,
            },
            PythonFrontendError::Parse {
                release,
                line,
                column,
                message,
            } => Self::Parse {
                release,
                line,
                column,
                message,
            },
        }
    }
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelease {
                requested,
                suggestions,
            } => {
                // `requested` is already escaped and bounded by python-parser.
                write!(f, "unknown Python release \"{requested}\"")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::ReleaseNotAvailable { release, stage } => {
                write!(f, "Python {release} cannot compile this subset ({stage})")
            }
            Self::Budget {
                release,
                which,
                limit,
            } => write!(f, "Python {release}: {which} exceeds the limit of {limit}"),
            Self::Lex {
                release,
                line,
                column,
                message,
            } => write!(f, "Python {release} lex error at {line}:{column}: {message}"),
            Self::Parse {
                release,
                line,
                column,
                message,
            } => write!(f, "Python {release} parse error at {line}:{column}: {message}"),
            Self::Unsupported {
                release,
                rule,
                start,
                end,
                message,
            } => {
                write!(f, "Python {release}: {message} (rule {rule}")?;
                if let Some((line, column)) = start {
                    write!(f, " at {line}:{column}")?;
                }
                if let Some((line, column)) = end {
                    write!(f, "-{line}:{column}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl std::error::Error for FrontendError {}
