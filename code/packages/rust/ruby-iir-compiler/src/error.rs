//! # Every way a Ruby compile can fail (LANG84 C6)
//!
//! One enum, six categories, and every variant that comes after release
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
//! The first five arrive from `ruby-parser` as a `RubyFrontendError` and are
//! carried over field for field ([`From`] below); `Unsupported` is the
//! compiler's own: the tree parsed, but it is outside what this compiler
//! lowers (a callee other than `puts`, a value outside `i64`, and so on).

use std::fmt;

use coding_adventures_ruby_parser::RubyFrontendError;

/// A failed Ruby compile. See the module docs for the categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    /// The requested ID is not a row of `code/grammars/ruby/releases.csv`.
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
        /// Why, self-describing: `no grammar pair: grammar_stage none` or
        /// `no IIR profile: …`.
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

impl From<RubyFrontendError> for FrontendError {
    fn from(error: RubyFrontendError) -> Self {
        match error {
            RubyFrontendError::UnknownRelease {
                requested,
                suggestions,
            } => Self::UnknownRelease {
                requested,
                suggestions,
            },
            // ruby-parser's `stage` is either the row's bare `grammar_stage`
            // (one word, such as `none`) or, for a regeneration bug, a fixed
            // phrase (`token table missing (regenerate ruby-lexer)`). The
            // first is prefixed here so the reason reads on its own next to
            // this crate's other one, `no IIR profile: …`, wherever it is
            // shown, including lang-aot's `cannot compile yet (…)`; the
            // phrase already says what is wrong and is kept as it is.
            RubyFrontendError::ReleaseNotAvailable { release, stage } => {
                let stage = if stage.contains(' ') {
                    stage
                } else {
                    format!("no grammar pair: grammar_stage {stage}")
                };
                Self::ReleaseNotAvailable { release, stage }
            }
            RubyFrontendError::Budget {
                release,
                which,
                limit,
            } => Self::Budget {
                release,
                which,
                limit,
            },
            RubyFrontendError::Lex {
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
            RubyFrontendError::Parse {
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
                // `requested` is already escaped and bounded by ruby-parser.
                write!(f, "unknown Ruby release \"{requested}\"")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::ReleaseNotAvailable { release, stage } => {
                write!(f, "Ruby {release} cannot compile this subset ({stage})")
            }
            Self::Budget {
                release,
                which,
                limit,
            } => write!(f, "Ruby {release}: {which} exceeds the limit of {limit}"),
            Self::Lex {
                release,
                line,
                column,
                message,
            } => write!(f, "Ruby {release} lex error at {line}:{column}: {message}"),
            Self::Parse {
                release,
                line,
                column,
                message,
            } => write!(f, "Ruby {release} parse error at {line}:{column}: {message}"),
            Self::Unsupported {
                release,
                rule,
                start,
                end,
                message,
            } => {
                write!(f, "Ruby {release}: {message} (rule {rule}")?;
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
