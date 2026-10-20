//! # Release-selected Ruby frontend (LANG84)
//!
//! LANG84 makes every tagged Ruby release a selectable thing. Each release
//! with a materialized pair has its own `ruby<release>.tokens` and
//! `ruby<release>.grammar`; this module runs the generic, grammar-driven
//! machinery over exactly one of them:
//!
//! ```text
//! source ──► budget check ──► GrammarLexer(ruby<release>.tokens)
//!        ──► GrammarParser(ruby<release>.grammar) ──► GrammarASTNode
//! ```
//!
//! There is no hand-written lexing here. The compiled tables live in
//! `coding_adventures_ruby_lexer::releases` (tokens) and this crate's
//! `_grammar.rs` (grammars); both are generated from the CSV inventory by
//! `grammar-tools generate-rust-compiled-grammars ruby`.
//!
//! ## No silent fallback (LANG84 C4)
//!
//! A release resolves to exactly one inventory row or to a typed error:
//!
//! | Request                          | Result                                  |
//! |----------------------------------|-----------------------------------------|
//! | `None`                           | [`DEFAULT_RELEASE`]                     |
//! | `Some("")`                       | `UnknownRelease` (empty is not default) |
//! | `Some("3.3.6")` (has a pair)     | that release's pair                     |
//! | `Some("2.7.8")` (row, no pair)   | `ReleaseNotAvailable { stage: "none" }` |
//! | `Some("9.9")` (no row)           | `UnknownRelease` + up to 5 suggestions  |
//!
//! A neighbouring release, the unversioned `ruby.tokens`/`ruby.grammar`, or a
//! "closest" grammar is never substituted.
//!
//! ## Budgets (LANG84 C5)
//!
//! Source text is untrusted. Its size is checked before the lexer allocates
//! anything, and the parser runs with a measured recursion cap, so hostile
//! input produces a [`RubyFrontendError::Budget`] or `Parse` error instead of
//! a stack overflow. The token regexes run on Rust's `regex` crate, which
//! matches in linear time (no backtracking blow-up).

use std::fmt;

use coding_adventures_ruby_lexer::releases;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

use crate::_grammar;

/// The release used when a caller names none. It is an exact inventory ID.
///
/// 3.3.6 is the host Ruby whose `Ripper.lex` is the R0 oracle. LANG84 C4.4
/// also wants this row to claim an IIR subset; that claim arrives with
/// phase 5 (until then `iir_status` is `-` for every Ruby row).
pub const DEFAULT_RELEASE: &str = "3.3.6";

/// Default source budget for untrusted programs (LANG84 C5).
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 64 * 1024;

/// Ceiling a trusted caller may raise the source budget to (LANG84 C5).
pub const MAX_TRUSTED_SOURCE_BYTES: usize = 16 * 1024 * 1024;

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
    /// (`MAX_RULE_DEPTH`, 180) are clamped to it: a deeper cap could
    /// overflow the native stack, which is an abort, not an error.
    pub max_rule_depth: usize,
}

impl Default for FrontendBudgets {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_rule_depth: crate::MAX_RULE_DEPTH,
        }
    }
}

impl FrontendBudgets {
    fn source_limit(&self) -> usize {
        self.max_source_bytes.min(MAX_TRUSTED_SOURCE_BYTES)
    }

    fn depth_limit(&self) -> usize {
        self.max_rule_depth.min(crate::MAX_RULE_DEPTH)
    }
}

/// Every way a release-selected parse can fail (LANG84 C6 categories that
/// belong to the parser; `Unsupported` belongs to the IIR compiler).
///
/// Lexer and parser errors carry the release and a 1-based line and column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RubyFrontendError {
    /// The ID is not a row of `code/grammars/ruby/releases.csv`.
    UnknownRelease {
        requested: String,
        /// Up to five inventory IDs sharing the longest prefix with the request.
        suggestions: Vec<String>,
    },
    /// The row exists but has no pair yet.
    ReleaseNotAvailable { release: String, stage: String },
    /// A C5 budget was exceeded.
    Budget {
        release: String,
        which: &'static str,
        limit: usize,
    },
    /// No token pattern of the release's `.tokens` matches here.
    Lex {
        release: String,
        line: usize,
        column: usize,
        message: String,
    },
    /// The release's `.grammar` rejects the token stream.
    Parse {
        release: String,
        line: usize,
        column: usize,
        message: String,
    },
}

impl fmt::Display for RubyFrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRelease {
                requested,
                suggestions,
            } => {
                write!(f, "unknown Ruby release {requested:?}")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::ReleaseNotAvailable { release, stage } => write!(
                f,
                "Ruby {release} has no grammar pair yet (grammar_stage {stage})"
            ),
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
        }
    }
}

impl std::error::Error for RubyFrontendError {}

/// A parse tree together with the release that produced it.
#[derive(Debug, Clone)]
pub struct RubyParse {
    pub release: &'static str,
    pub ast: GrammarASTNode,
}

/// Releases with a pair, in inventory order.
pub fn supported_releases() -> &'static [&'static str] {
    _grammar::SUPPORTED_RELEASES
}

/// The deduplicated table modules `(tokens, grammar)` a release compiles to.
///
/// Releases whose `.tokens` (or `.grammar`) files differ only in comments
/// share a module (LANG84 "Compiled tables deduplicate identical grammars").
/// Tests use this to pin which releases are lexically identical.
pub fn grammar_classes(release: &str) -> Option<(&'static str, &'static str)> {
    Some((releases::grammar_class(release)?, _grammar::grammar_class(release)?))
}

/// Resolve a request to the exact inventory ID whose pair will be used.
pub fn resolve_release(release: Option<&str>) -> Result<&'static str, RubyFrontendError> {
    let requested = release.unwrap_or(DEFAULT_RELEASE);
    if requested.len() > MAX_RELEASE_ID_BYTES {
        return Err(RubyFrontendError::UnknownRelease {
            requested: format!("{}...", truncate(requested, MAX_RELEASE_ID_BYTES)),
            suggestions: Vec::new(),
        });
    }
    if let Some(&id) = supported_releases().iter().find(|id| **id == requested) {
        return Ok(id);
    }
    match _grammar::INVENTORY.iter().find(|(id, _)| *id == requested) {
        Some((id, stage)) => Err(RubyFrontendError::ReleaseNotAvailable {
            release: (*id).to_string(),
            stage: (*stage).to_string(),
        }),
        None => Err(RubyFrontendError::UnknownRelease {
            requested: requested.to_string(),
            suggestions: suggestions(requested),
        }),
    }
}

/// Lex `source` with exactly one release's `.tokens`.
pub fn tokenize_ruby_at(
    source: &str,
    release: Option<&str>,
    budgets: &FrontendBudgets,
) -> Result<(&'static str, Vec<Token>), RubyFrontendError> {
    let release = resolve_release(release)?;
    let limit = budgets.source_limit();
    if source.len() > limit {
        return Err(RubyFrontendError::Budget {
            release: release.to_string(),
            which: "source bytes",
            limit,
        });
    }
    // Both tables are generated from the same CSV rows; a release present in
    // one and not the other is a regeneration bug, reported rather than
    // papered over.
    let grammar = releases::token_grammar(release).ok_or_else(|| {
        RubyFrontendError::ReleaseNotAvailable {
            release: release.to_string(),
            stage: "token table missing (regenerate ruby-lexer)".to_string(),
        }
    })?;
    let tokens = GrammarLexer::new(source, &grammar)
        .tokenize()
        .map_err(|error| RubyFrontendError::Lex {
            release: release.to_string(),
            line: error.line,
            column: error.column,
            message: error.message,
        })?;
    Ok((release, tokens))
}

/// Lex and parse `source` with exactly one release's pair.
pub fn parse_ruby_at(
    source: &str,
    release: Option<&str>,
    budgets: &FrontendBudgets,
) -> Result<RubyParse, RubyFrontendError> {
    let (release, tokens) = tokenize_ruby_at(source, release, budgets)?;
    let grammar = _grammar::parser_grammar(release).ok_or_else(|| {
        RubyFrontendError::ReleaseNotAvailable {
            release: release.to_string(),
            stage: "parser table missing (regenerate ruby-parser)".to_string(),
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
            if error.message.starts_with("input nests deeper than the supported limit")
                || error.message.contains("input within the supported nesting limit")
            {
                RubyFrontendError::Budget {
                    release: release.to_string(),
                    which: "parser rule depth",
                    limit: depth,
                }
            } else {
                RubyFrontendError::Parse {
                    release: release.to_string(),
                    line: error.token.line,
                    column: error.token.column,
                    message: error.message,
                }
            }
        })?;
    Ok(RubyParse { release, ast })
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

fn truncate(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
