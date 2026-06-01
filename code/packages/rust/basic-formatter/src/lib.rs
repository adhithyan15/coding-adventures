//! # `basic-formatter` — canonical Dartmouth BASIC pretty-printer.
//!
//! Token-stream → [`format_doc::Doc`] → canonical source.
//!
//! ## Why token-driven rather than AST-driven
//!
//! BASIC's grammar is one-statement-per-line, with no nested layout
//! decisions: every keyword starts at the line's first column (after
//! the line number), and arguments separate with single spaces /
//! commas / semicolons.  There's no horizontal-folding decision
//! ("compact vs broken") to make, so the formatter doesn't need
//! `format-doc`'s `fits()` look-ahead — it just walks the tokens
//! and emits with normalised whitespace.
//!
//! We still build a `format_doc::Doc` so the renderer pipeline is
//! identical to twig-formatter's — useful when future BASIC features
//! (e.g. multi-line `IF … THEN … ELSE` blocks if we ever extend the
//! dialect) want real layout decisions.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use coding_adventures_dartmouth_basic_lexer::tokenize_dartmouth_basic;
use format_doc::{concat, hardline, layout_doc, nil, render_text, text, Doc, LayoutOptions};
use lexer::token::{Token, TokenType};

// ===========================================================================
// Configuration
// ===========================================================================

/// Default print width — generous enough that BASIC lines never
/// need to wrap (BASIC programs from the 1964 spec are
/// fundamentally fixed-width-line).
pub const DEFAULT_PRINT_WIDTH: usize = 80;

/// Default indent width — BASIC doesn't indent (no nested blocks
/// in V1), but we keep the field for future use.
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Layout configuration for the BASIC formatter.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Maximum line width before the layout engine tries to break.
    /// Unused by V1 (BASIC lines never break) but reserved for
    /// future multi-line statements.
    pub print_width: usize,
    /// Spaces per indent level.  Unused by V1.
    pub indent_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            print_width:  DEFAULT_PRINT_WIDTH,
            indent_width: DEFAULT_INDENT_WIDTH,
        }
    }
}

impl Config {
    fn to_layout(self) -> LayoutOptions {
        LayoutOptions {
            print_width:  self.print_width,
            indent_width: self.indent_width,
            line_height:  1,
        }
    }
}

// ===========================================================================
// Errors
// ===========================================================================

/// Errors raised by [`format`].  V1 only fails on tokenisation
/// errors; the BASIC lexer panics rather than returning errors,
/// so this is reserved for future variants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// Tokenisation failed.  Reserved — the BASIC lexer doesn't
    /// produce errors in V1.
    Lex(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Lex(s) => write!(f, "lex error: {s}"),
        }
    }
}

impl std::error::Error for FormatError {}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Parse `source` and return canonically-formatted BASIC text.
///
/// Equivalent to `format_tokens(&tokenize_dartmouth_basic(source))`
/// with a trailing newline normalisation.
pub fn format(source: &str) -> Result<String, FormatError> {
    let tokens = tokenize_dartmouth_basic(source);
    Ok(format_tokens(&tokens))
}

/// Format an already-tokenised token stream.  Returns canonical
/// BASIC text with a single trailing newline (POSIX convention).
pub fn format_tokens(tokens: &[Token]) -> String {
    let config = Config::default();
    format_tokens_with_config(tokens, &config)
}

/// Format under a custom configuration.
pub fn format_tokens_with_config(tokens: &[Token], config: &Config) -> String {
    let doc = tokens_to_doc(tokens);
    let layout = layout_doc(doc, &config.to_layout());
    let rendered = render_text(&layout);
    // Trim trailing whitespace on each line, ensure single trailing newline.
    let mut out = String::with_capacity(rendered.len() + 1);
    for line in rendered.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

// ===========================================================================
// Token → Doc
// ===========================================================================

/// Group the token stream by logical BASIC lines (delimited by
/// `Newline` tokens) and emit one [`Doc`] per line, joined by
/// [`hardline`].
fn tokens_to_doc(tokens: &[Token]) -> Doc {
    let lines = split_into_lines(tokens);
    if lines.is_empty() {
        return nil();
    }
    let mut parts: Vec<Doc> = Vec::with_capacity(lines.len() * 2);
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            parts.push(hardline());
        }
        parts.push(line_to_doc(line));
    }
    concat(parts)
}

/// Split a token vec into logical BASIC lines.  Strips `Eof` and
/// the `Newline` separators themselves (which become `hardline()`
/// in the doc tree).
fn split_into_lines(tokens: &[Token]) -> Vec<Vec<&Token>> {
    let mut out: Vec<Vec<&Token>> = Vec::new();
    let mut current: Vec<&Token> = Vec::new();
    for tok in tokens {
        match tok.type_ {
            TokenType::Eof => break,
            TokenType::Newline => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(tok),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Build a [`Doc`] for one logical BASIC line.
///
/// Special-case: if the line contains a `REM` keyword, the
/// remainder of the line is preserved verbatim.  Otherwise: each
/// token is uppercased (if NAME / KEYWORD / BUILTIN_FN / USER_FN)
/// and joined with spacing rules.
fn line_to_doc(toks: &[&Token]) -> Doc {
    // Detect REM and preserve the rest of the line verbatim.
    if let Some(rem_idx) = toks.iter().position(|t|
        t.type_ == TokenType::Keyword && t.value.eq_ignore_ascii_case("REM"))
    {
        return rem_line_to_doc(toks, rem_idx);
    }

    let mut parts: Vec<Doc> = Vec::with_capacity(toks.len() * 2);
    let mut prev: Option<&Token> = None;
    for tok in toks {
        if let Some(prev_tok) = prev {
            if needs_space_between(prev_tok, tok) {
                parts.push(text(" "));
            }
        }
        parts.push(text(token_canonical_string(tok)));
        prev = Some(*tok);
    }
    concat(parts)
}

/// Build a [`Doc`] for a REM line, preserving everything after the
/// `REM` keyword verbatim (separated by a single space).
fn rem_line_to_doc(toks: &[&Token], rem_idx: usize) -> Doc {
    // Everything before REM (typically the line number) gets the
    // standard tokens-with-spaces treatment.
    let mut parts: Vec<Doc> = Vec::new();
    let mut prev: Option<&Token> = None;
    for tok in &toks[..rem_idx] {
        if let Some(prev_tok) = prev {
            if needs_space_between(prev_tok, tok) {
                parts.push(text(" "));
            }
        }
        parts.push(text(token_canonical_string(tok)));
        prev = Some(*tok);
    }
    // REM keyword (always uppercased), then the rest of the
    // tokens joined with their original values and single spaces —
    // we re-glue the comment text from the source spans.
    if !parts.is_empty() {
        parts.push(text(" "));
    }
    parts.push(text("REM"));
    for tok in &toks[rem_idx + 1..] {
        parts.push(text(" "));
        // Preserve original case in REM payload — comments are
        // semantically text, not BASIC code.
        parts.push(text(tok.value.clone()));
    }
    concat(parts)
}

/// Canonical (uppercased) string for a token.  Keywords, NAMEs,
/// built-in functions, and user functions all uppercase; numeric
/// literals, strings, and operators are emitted verbatim.
fn token_canonical_string(tok: &Token) -> String {
    let etype = tok.effective_type_name();
    match (tok.type_, etype) {
        (TokenType::Keyword, _)    => tok.value.to_uppercase(),
        (_, "NAME")                => tok.value.to_uppercase(),
        (_, "BUILTIN_FN")          => tok.value.to_uppercase(),
        (_, "USER_FN")             => tok.value.to_uppercase(),
        _                          => tok.value.clone(),
    }
}

/// Spacing decision between two adjacent tokens.  Returns `true` if
/// a single space should separate them in the canonical form.
fn needs_space_between(prev: &Token, next: &Token) -> bool {
    let p = prev.effective_type_name();
    let n = next.effective_type_name();

    // No space directly after `(` or directly before `)`.
    if p == "LPAREN" || n == "RPAREN" { return false; }
    // No space before `,` `;` `:` (the separator itself); space
    // after them is added by the no-space-before-`,` logic in the
    // forward direction (the comma matches this rule, then the
    // next non-comma token wants a space).
    if n == "COMMA" || n == "SEMICOLON" || n == "COLON" {
        return false;
    }
    // No space immediately before a `(` — function call style.
    if n == "LPAREN" {
        // …unless prev is a keyword (so `IF (…)` keeps a space).
        return prev.type_ == TokenType::Keyword;
    }
    // Everything else gets a single space.
    true
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_minimal_end() {
        let out = format("10 END\n").expect("ok");
        assert_eq!(out, "10 END\n");
    }

    #[test]
    fn uppercases_keywords() {
        let out = format("10 let a = 1\n").expect("ok");
        // The line starts with a digit (line number), then LET, then A, etc.
        assert!(out.contains("LET"), "expected LET in {out:?}");
        assert!(!out.contains("let "), "lowercase let leaked: {out:?}");
    }

    #[test]
    fn uppercases_identifiers() {
        let out = format("10 LET a = 1\n").expect("ok");
        assert!(out.contains(" A "), "expected uppercase A in {out:?}");
    }

    #[test]
    fn normalises_multiple_spaces() {
        let out = format("10  LET  A   =   1\n").expect("ok");
        // No double-spaces in canonical form.
        assert!(!out.contains("  "), "double space leaked: {out:?}");
    }

    #[test]
    fn rem_keyword_survives_payload_discarded() {
        // The dartmouth-basic-lexer intentionally discards the REM
        // payload as part of its tokenisation (see lexer docs).  So
        // the formatter can only preserve the REM keyword itself,
        // not the comment text.  This is a known V1 limitation; a
        // future revision could source-walk to recover the comment
        // verbatim.
        let out = format("10 REM hello world\n").expect("ok");
        assert!(out.contains("REM"), "expected REM keyword in output: {out:?}");
    }

    #[test]
    fn comma_gets_following_space() {
        let out = format("10 INPUT A,B,C\n").expect("ok");
        // Each comma should be followed by exactly one space.
        assert!(out.contains("A, B"), "missing comma-space: {out:?}");
        assert!(out.contains("B, C"), "missing comma-space: {out:?}");
    }

    #[test]
    fn no_space_inside_parens() {
        let out = format("10 LET X = SQR(2)\n").expect("ok");
        // `SQR(2)` — no space after `(` or before `)`.
        assert!(out.contains("SQR(2)"), "paren spacing wrong: {out:?}");
    }

    #[test]
    fn idempotent() {
        // Formatting an already-formatted program should be a no-op.
        let original = format("10 LET A = 1\n20 PRINT A\n30 END\n").expect("ok");
        let twice = format(&original).expect("ok");
        assert_eq!(original, twice, "formatter is not idempotent");
    }

    #[test]
    fn for_loop_round_trip() {
        // FOR / NEXT loop — exercises multi-keyword statements.
        let out = format("10 FOR I = 1 TO 10 STEP 2\n20 PRINT I\n30 NEXT I\n40 END\n").expect("ok");
        assert!(out.contains("FOR"));
        assert!(out.contains("TO"));
        assert!(out.contains("STEP"));
        assert!(out.contains("NEXT"));
    }
}

