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

/// The fixed text each producer writes, mapped to a C6 category.
///
/// Every test anchors on text the *producer* writes before any echo of the
/// source: a whole fixed message, or a fixed prefix. A substring test would
/// let source text pick the category: `console.log(1 "spine limit")` fails
/// to parse with a message that quotes the string, and was reported as a
/// Budget refusal (review L1).
///
/// | Producer                        | Text it writes (prefix or whole)                                      | Category    |
/// |---------------------------------|-----------------------------------------------------------------------|-------------|
/// | `javascript-lexer`              | `JavaScript tokenization failed: …`                                   | Lex         |
/// | `parse_javascript_cst_bounded`  | `JavaScript exponentiation is unsupported in the bounded CST parser`  | Unsupported |
/// | `parse_javascript_cst_bounded`  | `JavaScript expression chain …` (only the spine bound writes this)    | Budget      |
/// | `GrammarParser` depth guard     | `JavaScript parse failed: Parse error at L:C: input nests deeper than the supported limit (N)`, or `… L:C: Expected input within the supported nesting limit or …, got "…"` (the phrase as a list item before `, got `) | Budget |
/// | anything else from the parser   | `JavaScript parse failed: …`                                          | Parse       |
const LEX_PREFIX: &str = "JavaScript tokenization failed";
const EXPONENT_MESSAGE: &str = "JavaScript exponentiation is unsupported in the bounded CST parser";
const SPINE_PREFIX: &str = "JavaScript expression chain ";
const PARSE_PREFIX: &str = "JavaScript parse failed: ";
const DEPTH_MESSAGE_PREFIX: &str = "input nests deeper than the supported limit (";
const DEPTH_EXPECTATION: &str = "input within the supported nesting limit";
const NODE_LIMIT_MESSAGE: &str = "JavaScript AST exceeds the native pilot node limit";
const DEPTH_LIMIT_MESSAGE: &str = "JavaScript AST exceeds the native pilot depth limit";

/// Whether `message` is the grammar parser's depth refusal: the
/// `GrammarParseError` display is `Parse error at <line>:<col>: <message>`,
/// and only the depth guard writes a message starting
/// [`DEPTH_MESSAGE_PREFIX`]; ordinary failures start `Expected …`.
fn is_depth_refusal(message: &str) -> bool {
    let Some(rest) = message.strip_prefix(PARSE_PREFIX) else {
        return false;
    };
    let rest = match rest.strip_prefix("Parse error at ") {
        Some(located) => {
            // Skip `<line>:<column>: `, both all digits.
            let mut parts = located.splitn(3, ':');
            let (Some(line), Some(column), Some(tail)) = (parts.next(), parts.next(), parts.next()) else {
                return false;
            };
            if line.is_empty() || column.is_empty() || !line.bytes().chain(column.bytes()).all(|b| b.is_ascii_digit()) {
                return false;
            }
            match tail.strip_prefix(' ') {
                Some(tail) => tail,
                None => return false,
            }
        }
        None => rest,
    };
    if rest.starts_with(DEPTH_MESSAGE_PREFIX) {
        return true;
    }
    // The furthest-failure form: `Expected A or B or …, got "<token>"`. The
    // depth guard records the expectation `input within the supported
    // nesting limit`. Only the list before `, got ` is producer text; what
    // follows quotes the source, so the phrase is matched as a whole list
    // item there and nowhere else.
    let Some(expected) = rest.strip_prefix("Expected ") else {
        return false;
    };
    let list = expected.split_once(", got ").map_or(expected, |(list, _)| list);
    list.split(" or ").any(|item| item == DEPTH_EXPECTATION)
}

impl FrontendError {
    /// Categorize an error from `parse_javascript_cst_bounded`.
    pub(crate) fn from_parse(release: &str, message: String) -> Self {
        let release = release.to_string();
        if message.starts_with(LEX_PREFIX) {
            Self::Lex { release, message }
        } else if message == EXPONENT_MESSAGE {
            Self::Unsupported { release, message }
        } else if message.starts_with(SPINE_PREFIX) {
            Self::Budget {
                release,
                which: "expression chain spine",
                message,
            }
        } else if is_depth_refusal(&message) {
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
    /// | Text                                       | Category    |
    /// |--------------------------------------------|-------------|
    /// | exactly the node-limit message             | Budget      |
    /// | exactly the depth-limit message            | Budget      |
    /// | `SyntaxError: …` (an early error)          | Parse       |
    /// | anything else                              | Unsupported |
    ///
    /// The budget messages are compared whole: other compile messages quote
    /// source names, which must not choose the category.
    pub(crate) fn from_compile(release: &str, message: String) -> Self {
        let release = release.to_string();
        if message == NODE_LIMIT_MESSAGE {
            Self::Budget {
                release,
                which: "CST nodes",
                message,
            }
        } else if message == DEPTH_LIMIT_MESSAGE {
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
