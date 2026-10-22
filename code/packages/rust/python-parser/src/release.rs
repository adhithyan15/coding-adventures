//! # Release-selected Python frontend (LANG84)
//!
//! LANG84 makes every tagged CPython release a selectable thing. Each release
//! with a materialized pair has its own `python<release>.tokens` and
//! `python<release>.grammar`, and this module runs the generic,
//! grammar-driven machinery over exactly one of them:
//!
//! ```text
//! source ──► budget check ──► GrammarLexer(python<release>.tokens)
//!        ──► token budget ──► GrammarParser(python<release>.grammar)
//!        ──► GrammarASTNode
//! ```
//!
//! There is no hand-written lexing here. The compiled tables live in
//! `coding_adventures_python_lexer::releases` (tokens) and this crate's
//! `_grammar.rs` (grammars). `grammar-tools generate-rust-compiled-grammars
//! python` generates both from `code/grammars/python/releases.csv`.
//!
//! ## Release IDs are not minor lines
//!
//! The crate also keeps the pre-LANG84 *minor-line* pairs (`python3.12.*`)
//! for [`crate::parse_python`]. A minor line such as `"3.12"` is not a
//! tagged release, so this module never accepts one, and the legacy API
//! never accepts a release ID:
//!
//! | Request                      | Legacy `parse_python` | Release `parse_python_at`        |
//! |------------------------------|-----------------------|----------------------------------|
//! | `"3.12"`                     | the 3.12 minor line   | `UnknownRelease` (not a tag)     |
//! | `"3.13.16"`                  | error                 | that release's pair              |
//! | `""`                         | the default 3.12 line | `UnknownRelease` (empty is not default) |
//!
//! ## No silent fallback (LANG84 C4)
//!
//! A release resolves to exactly one inventory row or to a typed error:
//!
//! | Request                          | Result                                  |
//! |----------------------------------|-----------------------------------------|
//! | `None`                           | [`DEFAULT_RELEASE`]                     |
//! | `Some("")`                       | `UnknownRelease` (empty is not default) |
//! | `Some("3.12.3")` (has a pair)    | that release's pair                     |
//! | `Some("3.12.2")` (row, no pair)  | `ReleaseNotAvailable { stage: "none" }` |
//! | `Some("9.9")` (no row)           | `UnknownRelease` + up to 5 suggestions  |
//!
//! A neighbouring release, a minor-line pair, the unversioned
//! `python.tokens`/`python.grammar`, or a "closest" grammar is never
//! substituted.
//!
//! ## Budgets (LANG84 C5)
//!
//! Source text is untrusted. Its size is checked before the lexer allocates
//! anything, the token count is checked before the parser builds its memo,
//! and the parser runs with the crate's measured recursion cap. Hostile input
//! therefore produces a [`PythonFrontendError::Budget`] or `Parse` error
//! instead of a stack overflow or an out-of-memory abort. The token regexes
//! run on Rust's `regex` crate, which matches in linear time.

use std::fmt;

use coding_adventures_python_lexer::releases;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

use crate::_grammar;

/// The release used when a caller names none. It is an exact inventory ID.
///
/// 3.13.16 is the host `python3` whose interpreter and `tokenize` module are
/// the phase-7a oracle, and its row claims `iir_status` `py-int-print`
/// (LANG84 C4.4).
pub const DEFAULT_RELEASE: &str = "3.13.16";

/// Default source budget for untrusted programs (LANG84 C5).
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 64 * 1024;

/// Ceiling a trusted caller may raise the source budget to (LANG84 C5).
///
/// Sized by memory, not by time: the packrat `GrammarParser` keeps its memo
/// and CST at roughly 1–3 KB per token, so a 16 MiB source could need tens of
/// gigabytes and end in an out-of-memory abort rather than a `Budget` error.
/// The token budget below bounds the parser's input directly.
pub const MAX_TRUSTED_SOURCE_BYTES: usize = 1024 * 1024;

/// Default token budget for untrusted programs (LANG84 C5), checked after
/// lexing and before the parser builds its memo.
pub const DEFAULT_MAX_TOKENS: usize = 16 * 1024;

/// Ceiling a trusted caller may raise the token budget to.
pub const MAX_TRUSTED_TOKENS: usize = 256 * 1024;

/// Upper bound for one release ID; longer requests are rejected before any
/// lookup so a hostile string cannot be echoed back at length.
const MAX_RELEASE_ID_BYTES: usize = 64;

/// Limits applied to one parse. `Default` is the untrusted profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendBudgets {
    /// Source size in bytes, checked before lexing. Values above
    /// [`MAX_TRUSTED_SOURCE_BYTES`] are clamped to it.
    pub max_source_bytes: usize,
    /// Parser recursion cap. Values above the crate's measured safe cap
    /// (`MAX_RULE_DEPTH`, 190) are clamped to it: a deeper cap could
    /// overflow the native stack, which is an abort, not an error.
    pub max_rule_depth: usize,
    /// Token count, checked after lexing and before parsing. Values above
    /// [`MAX_TRUSTED_TOKENS`] are clamped to it.
    pub max_tokens: usize,
}

impl Default for FrontendBudgets {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_rule_depth: crate::MAX_RULE_DEPTH,
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}

impl FrontendBudgets {
    fn source_limit(&self) -> usize {
        self.max_source_bytes.min(MAX_TRUSTED_SOURCE_BYTES)
    }

    fn token_limit(&self) -> usize {
        self.max_tokens.min(MAX_TRUSTED_TOKENS)
    }

    fn depth_limit(&self) -> usize {
        self.max_rule_depth.min(crate::MAX_RULE_DEPTH)
    }
}

/// Every way a release-selected parse can fail. These are the LANG84 C6
/// categories that belong to the parser; `Unsupported` belongs to the IIR
/// compiler.
///
/// Lexer and parser errors carry the release and a 1-based line and column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonFrontendError {
    /// The ID is not a row of `code/grammars/python/releases.csv`.
    UnknownRelease {
        /// The request, truncated and with control characters escaped.
        requested: String,
        /// Up to five inventory IDs sharing the longest prefix with the request.
        suggestions: Vec<String>,
    },
    /// The row exists but has no pair yet.
    ReleaseNotAvailable {
        /// The inventory ID.
        release: String,
        /// The row's `grammar_stage`, or why its table is missing.
        stage: String,
    },
    /// A C5 budget was exceeded.
    Budget {
        /// The release the parse ran against.
        release: String,
        /// Which budget.
        which: &'static str,
        /// Its limit.
        limit: usize,
    },
    /// No token pattern of the release's `.tokens` matches here.
    Lex {
        /// The release the parse ran against.
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
        /// The release the parse ran against.
        release: String,
        /// 1-based line of the furthest failure.
        line: usize,
        /// 1-based column of the furthest failure.
        column: usize,
        /// The parser's message, listing the expected terminals.
        message: String,
    },
}

impl fmt::Display for PythonFrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelease {
                requested,
                suggestions,
            } => {
                // `requested` is already escaped and bounded by `resolve_release`.
                write!(f, "unknown Python release \"{requested}\"")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::ReleaseNotAvailable { release, stage } => write!(
                f,
                "Python {release} has no grammar pair yet (grammar_stage {stage})"
            ),
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
        }
    }
}

impl std::error::Error for PythonFrontendError {}

/// A parse tree together with the release that produced it.
#[derive(Debug, Clone)]
pub struct PythonParse {
    /// The exact inventory ID whose pair parsed the source.
    pub release: &'static str,
    /// The `file` node of the release's grammar.
    pub ast: GrammarASTNode,
}

/// Releases with a pair, in inventory (date) order.
pub fn supported_releases() -> &'static [&'static str] {
    _grammar::SUPPORTED_RELEASES
}

/// The deduplicated table modules `(tokens, grammar)` a release compiles to.
///
/// Releases whose `.tokens` (or `.grammar`) files differ only in comments
/// share a module (LANG84 "Compiled tables deduplicate identical grammars").
/// Tests use this to pin which releases are lexically identical.
pub fn grammar_classes(release: &str) -> Option<(&'static str, &'static str)> {
    Some((
        releases::grammar_class(release)?,
        _grammar::grammar_class(release)?,
    ))
}

/// The compiled parser grammar of one paired release, or `None` for a
/// release with no pair (exact IDs only, like every lookup here).
///
/// Exposed for LANG84 C8 `cst_contract` tests: an IIR compiler checks that
/// every rule name it dispatches on exists in each release it claims.
pub fn release_parser_grammar(
    release: &str,
) -> Option<grammar_tools::parser_grammar::ParserGrammar> {
    _grammar::parser_grammar(release)
}

/// Resolve a request to the exact inventory ID whose pair will be used.
pub fn resolve_release(release: Option<&str>) -> Result<&'static str, PythonFrontendError> {
    let requested = release.unwrap_or(DEFAULT_RELEASE);
    if requested.len() > MAX_RELEASE_ID_BYTES {
        return Err(PythonFrontendError::UnknownRelease {
            requested: format!("{}...", echo(truncate(requested, MAX_RELEASE_ID_BYTES))),
            suggestions: Vec::new(),
        });
    }
    if let Some(&id) = supported_releases().iter().find(|id| **id == requested) {
        return Ok(id);
    }
    match _grammar::INVENTORY.iter().find(|(id, _)| *id == requested) {
        Some((id, stage)) => Err(PythonFrontendError::ReleaseNotAvailable {
            release: (*id).to_string(),
            stage: (*stage).to_string(),
        }),
        None => Err(PythonFrontendError::UnknownRelease {
            requested: echo(requested),
            suggestions: suggestions(requested),
        }),
    }
}

/// Lex `source` with exactly one release's `.tokens`.
pub fn tokenize_python_at(
    source: &str,
    release: Option<&str>,
    budgets: &FrontendBudgets,
) -> Result<(&'static str, Vec<Token>), PythonFrontendError> {
    let release = resolve_release(release)?;
    let limit = budgets.source_limit();
    if source.len() > limit {
        return Err(PythonFrontendError::Budget {
            release: release.to_string(),
            which: "source bytes",
            limit,
        });
    }
    // Both tables are generated from the same CSV rows. A release present in
    // one and not the other is a regeneration bug, reported rather than
    // papered over.
    let grammar = releases::token_grammar(release).ok_or_else(|| {
        PythonFrontendError::ReleaseNotAvailable {
            release: release.to_string(),
            stage: "token table missing (regenerate python-lexer)".to_string(),
        }
    })?;
    let tokens = GrammarLexer::new(source, &grammar)
        .tokenize()
        .map_err(|error| PythonFrontendError::Lex {
            release: release.to_string(),
            line: error.line,
            column: error.column,
            // The lexer quotes the offending character with `{:?}`, so a
            // control character in hostile source arrives escaped.
            message: error.message,
        })?;
    let token_limit = budgets.token_limit();
    if tokens.len() > token_limit {
        return Err(PythonFrontendError::Budget {
            release: release.to_string(),
            which: "tokens",
            limit: token_limit,
        });
    }
    Ok((release, tokens))
}

/// Lex and parse `source` with exactly one release's pair.
pub fn parse_python_at(
    source: &str,
    release: Option<&str>,
    budgets: &FrontendBudgets,
) -> Result<PythonParse, PythonFrontendError> {
    let (release, tokens) = tokenize_python_at(source, release, budgets)?;
    let grammar = _grammar::parser_grammar(release).ok_or_else(|| {
        PythonFrontendError::ReleaseNotAvailable {
            release: release.to_string(),
            stage: "parser table missing (regenerate python-parser)".to_string(),
        }
    })?;
    let depth = budgets.depth_limit();
    let ast = GrammarParser::new(tokens, grammar)
        .with_max_depth(depth)
        .parse()
        .map_err(|error| {
            // The parser reports its depth guard as an ordinary error: either
            // its dedicated message, or (when an enclosing optional or
            // repetition backtracked past the refusal) a furthest-failure
            // message whose expected terminal is the guard's marker. Both
            // are the budget, not a syntax error.
            if error
                .message
                .starts_with("input nests deeper than the supported limit")
                || error
                    .message
                    .contains("input within the supported nesting limit")
            {
                PythonFrontendError::Budget {
                    release: release.to_string(),
                    which: "parser rule depth",
                    limit: depth,
                }
            } else {
                PythonFrontendError::Parse {
                    release: release.to_string(),
                    line: error.token.line,
                    column: error.token.column,
                    message: error.message,
                }
            }
        })?;
    Ok(PythonParse { release, ast })
}

/// Up to five inventory IDs that share the longest prefix with `requested`.
fn suggestions(requested: &str) -> Vec<String> {
    let common = |id: &str| {
        id.bytes()
            .zip(requested.bytes())
            .take_while(|(a, b)| a == b)
            .count()
    };
    let best = _grammar::INVENTORY
        .iter()
        .map(|(id, _)| common(id))
        .max()
        .unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    _grammar::INVENTORY
        .iter()
        .filter(|(id, _)| common(id) == best)
        .take(5)
        .map(|(id, _)| (*id).to_string())
        .collect()
}

/// Neutralize caller-supplied text before it enters a diagnostic.
///
/// A release ID comes from outside (an API argument, later a `--lang
/// python@...` flag), and every error that names it ends up in a log or a
/// terminal. A raw newline there forges a log line; an ESC drives the
/// terminal. So each character goes through `char::escape_debug`: printable
/// text is unchanged and a control character arrives as visible text.
///
/// | request          | echoed as          |
/// |------------------|--------------------|
/// | `3.13`           | `3.13`             |
/// | `x<LF>INFO: ok`  | `x\nINFO: ok`      |
/// | `a<ESC>[2J`      | `a\u{1b}[2J`       |
fn echo(text: &str) -> String {
    text.chars().flat_map(char::escape_debug).collect()
}

fn truncate(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
