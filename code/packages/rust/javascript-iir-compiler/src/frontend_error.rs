//! # Every way a release-path JavaScript compile can fail (LANG84 C6)
//!
//! One enum, six categories. Every variant after release resolution names
//! the ECMAScript edition it ran against:
//!
//! | Variant               | Raised by                     | Example `message`                                   |
//! |-----------------------|-------------------------------|-----------------------------------------------------|
//! | `UnknownRelease`      | release resolution            | (the request, escaped, and up to five suggestions)  |
//! | `ReleaseNotAvailable` | release resolution            | (why the row cannot compile the subset)             |
//! | `Budget`              | a C5 limit                    | `JavaScript source exceeds the 65536-byte …`        |
//! | `Lex`                 | `GrammarLexer`                | `JavaScript tokenization failed: …`                 |
//! | `Parse`               | `GrammarParser`, early errors | `JavaScript parse failed: …`                        |
//! | `Unsupported`         | this crate's subset check     | `unsupported JavaScript expression in …`            |
//!
//! The parser crate reports its failures as `String`s, so this module sorts
//! them into categories by the fixed prefix each producer writes (see
//! [`FrontendError::from_parse`] and [`FrontendError::from_compile`]). The
//! original text is kept verbatim in `message`: the pre-6d `compile_source`
//! returns exactly that text, so none of its callers sees a changed error.

use std::fmt;

/// A failed release-path compile. See the module docs for the categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    /// The requested ID is not a row of `code/grammars/ecmascript/releases.csv`.
    UnknownRelease {
        /// The request, truncated and with control characters escaped.
        requested: String,
        /// Up to five inventory IDs sharing the longest prefix with it.
        suggestions: Vec<String>,
    },
    /// The release is in the inventory but does not claim the subset: it has
    /// no grammar pair, or its pair cannot yet lex the subset (es3/es5 lack
    /// the F10 regex/division table, LANG84 phase 6a/6d).
    ReleaseNotAvailable {
        /// The inventory ID.
        release: String,
        /// Why, self-describing.
        stage: String,
    },
    /// A LANG84 C5 budget was exceeded.
    Budget {
        /// The edition the compile ran against.
        release: String,
        /// Which budget.
        which: &'static str,
        /// The producer's message, verbatim.
        message: String,
    },
    /// No token pattern of the edition's `.tokens` matches.
    Lex {
        /// The edition the compile ran against.
        release: String,
        /// The lexer's message, verbatim.
        message: String,
    },
    /// The edition's `.grammar` rejects the token stream, or the script has
    /// an early error (a `SyntaxError` before any statement runs).
    Parse {
        /// The edition the compile ran against.
        release: String,
        /// The parser's (or early-error check's) message, verbatim.
        message: String,
    },
    /// The program parsed but lies outside `es-number-log`.
    Unsupported {
        /// The edition the compile ran against.
        release: String,
        /// The compiler's message, verbatim.
        message: String,
    },
}

/// Prefixes and phrases each producer writes, mapped to a C6 category.
///
/// | Producer                                   | Text it writes                                      | Category    |
/// |--------------------------------------------|-----------------------------------------------------|-------------|
/// | `javascript-lexer`                         | `JavaScript tokenization failed: …`                 | Lex         |
/// | `parse_javascript_cst_bounded`             | `… exponentiation is unsupported …`                 | Unsupported |
/// | `parse_javascript_cst_bounded`             | `… bounded CST spine limit`                         | Budget      |
/// | `GrammarParser` depth guard                | `… nests deeper …` / `… supported nesting limit`    | Budget      |
/// | `asi::parse_with_asi`                      | `JavaScript parse failed: …`                        | Parse       |
impl FrontendError {
    /// Categorize an error from `parse_javascript_cst_bounded`.
    pub(crate) fn from_parse(release: &str, message: String) -> Self {
        let release = release.to_string();
        if message.starts_with("JavaScript tokenization failed") {
            Self::Lex { release, message }
        } else if message.contains("exponentiation is unsupported") {
            Self::Unsupported { release, message }
        } else if message.contains("spine limit") {
            Self::Budget {
                release,
                which: "expression chain spine",
                message,
            }
        } else if message.contains("nests deeper than the supported limit")
            || message.contains("input within the supported nesting limit")
        {
            Self::Budget {
                release,
                which: "parser rule depth",
                message,
            }
        } else {
            Self::Parse { release, message }
        }
    }

    /// Categorize an error from [`crate::compile_ast`].
    ///
    /// | Text                                | Category |
    /// |-------------------------------------|----------|
    /// | `… node limit` / `… depth limit`    | Budget   |
    /// | `SyntaxError: …` (an early error)   | Parse    |
    /// | anything else                       | Unsupported |
    pub(crate) fn from_compile(release: &str, message: String) -> Self {
        let release = release.to_string();
        if message.contains("node limit") {
            Self::Budget {
                release,
                which: "CST nodes",
                message,
            }
        } else if message.contains("depth limit") {
            Self::Budget {
                release,
                which: "expression depth",
                message,
            }
        } else if message.starts_with("SyntaxError:") {
            Self::Parse { release, message }
        } else {
            Self::Unsupported { release, message }
        }
    }

    /// The edition the failure belongs to, when one was resolved.
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

    /// The LANG84 C6 category name, which is also what a corpus `.reject`
    /// file names.
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

    /// The producer's own text, without the release prefix. This is what
    /// the pre-6d `String` API ([`crate::compile_source`]) returns.
    pub fn message(&self) -> String {
        match self {
            Self::Budget { message, .. }
            | Self::Lex { message, .. }
            | Self::Parse { message, .. }
            | Self::Unsupported { message, .. } => message.clone(),
            other => other.to_string(),
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
                write!(f, "unknown JavaScript release \"{requested}\"")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::ReleaseNotAvailable { release, stage } => {
                write!(f, "JavaScript {release} cannot compile yet ({stage})")
            }
            Self::Budget {
                release, message, ..
            }
            | Self::Lex { release, message }
            | Self::Parse { release, message }
            | Self::Unsupported { release, message } => write!(f, "JavaScript {release}: {message}"),
        }
    }
}

impl std::error::Error for FrontendError {}
