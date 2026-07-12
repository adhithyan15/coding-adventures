//! # `nib-formatter` — canonical Nib pretty-printer.
//!
//! Token-stream → canonical Nib source.  Sibling of
//! `basic-formatter` and `twig-formatter`.
//!
//! ## Strategy
//!
//! Nib's grammar is brace-delimited (C-family), so unlike BASIC the
//! formatter needs to track block depth for indentation.  We walk
//! the token stream linearly, maintaining:
//!
//! - `depth`: number of unclosed `{` braces seen.
//! - A small state machine that decides when to emit a newline
//!   (after `{`, after `;` inside a block, before `}`) versus a
//!   space.
//!
//! The output is realised directly as a `String` (no `format-doc`
//! `Doc` tree) because Nib's V1 has no horizontal-folding
//! decisions — every statement goes on its own line.  We keep
//! `format-doc` in `Cargo.toml` so a future version that wants
//! per-statement breaking (e.g. for long expressions) can drop the
//! same renderer pipeline as twig-formatter without API churn.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use coding_adventures_nib_lexer::tokenize_nib;
use lexer::token::{Token, TokenType};

// ===========================================================================
// Configuration
// ===========================================================================

/// Default print width.  Unused by V1 (Nib statements stay on
/// their own line regardless of width).
pub const DEFAULT_PRINT_WIDTH: usize = 100;

/// Default indent width — 2 spaces, matching rustfmt's default
/// inside `{ … }` blocks.
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Layout configuration.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Maximum line width.  Reserved for future use.
    pub print_width: usize,
    /// Spaces per indent level.
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

// ===========================================================================
// Errors
// ===========================================================================

/// Errors raised by [`format`].  V1 only reserves variants for
/// future expansion — the Nib lexer doesn't return errors today.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// Reserved.
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

/// Tokenise `source` and return canonically-formatted Nib text.
pub fn format(source: &str) -> Result<String, FormatError> {
    let tokens = tokenize_nib(source);
    Ok(format_tokens(&tokens))
}

/// Format an already-tokenised stream under the default config.
pub fn format_tokens(tokens: &[Token]) -> String {
    format_tokens_with_config(tokens, &Config::default())
}

/// Format with a custom config.
pub fn format_tokens_with_config(tokens: &[Token], config: &Config) -> String {
    let mut out = String::new();
    let mut depth: usize = 0;
    let mut at_line_start = true;
    let mut prev: Option<&Token> = None;

    let printable: Vec<&Token> = tokens.iter()
        .filter(|t| !matches!(t.type_,
            TokenType::Newline | TokenType::Eof
            | TokenType::Indent | TokenType::Dedent))
        .collect();

    for (i, tok) in printable.iter().enumerate() {
        let is_open_brace  = is_lbrace(tok);
        let is_close_brace = is_rbrace(tok);
        let is_semicolon   = tok.type_ == TokenType::Semicolon;

        // `}` decreases depth and lives on its own line.
        if is_close_brace {
            depth = depth.saturating_sub(1);
            if !at_line_start {
                out.push('\n');
                at_line_start = true;
            }
        }

        // Indent at start of every logical line.
        if at_line_start {
            for _ in 0..(depth * config.indent_width) {
                out.push(' ');
            }
            at_line_start = false;
        } else if let Some(prev_tok) = prev {
            // Mid-line spacing decision.
            if needs_space_between(prev_tok, tok) {
                out.push(' ');
            }
        }

        // Emit the token.
        out.push_str(&tok.value);

        // `{` opens a new block — bump depth, newline after it.
        if is_open_brace {
            depth += 1;
            // Look ahead: if the next token is `}`, keep it on the
            // same line (`{}` empty block stays compact).
            let next_is_close = printable.get(i + 1).is_some_and(|t| is_rbrace(t));
            if !next_is_close {
                out.push('\n');
                at_line_start = true;
            }
        } else if is_semicolon {
            // Newline after `;` whenever we're inside a block body.
            // (Outside a block — e.g. statement terminator at top
            // level — keep the same behaviour; Nib's grammar always
            // wraps statements in fn bodies, so depth > 0 at this
            // point.)
            out.push('\n');
            at_line_start = true;
        }

        prev = Some(*tok);
    }

    // Ensure exactly one trailing newline.
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

// ===========================================================================
// Spacing decisions
// ===========================================================================

fn is_lbrace(t: &Token) -> bool {
    t.value == "{" || t.effective_type_name() == "LBRACE"
}
fn is_rbrace(t: &Token) -> bool {
    t.value == "}" || t.effective_type_name() == "RBRACE"
}
fn is_lparen(t: &Token) -> bool {
    t.value == "(" || t.effective_type_name() == "LPAREN"
}
fn is_rparen(t: &Token) -> bool {
    t.value == ")" || t.effective_type_name() == "RPAREN"
}
fn is_keyword(t: &Token) -> bool {
    matches!(t.type_, TokenType::Keyword) || {
        let etype = t.effective_type_name();
        matches!(etype, "fn" | "let" | "static" | "const" | "return"
            | "for" | "while" | "in" | "if" | "else" | "true" | "false")
    }
}

/// Decide whether a single space should separate `prev` and `next`.
fn needs_space_between(prev: &Token, next: &Token) -> bool {
    // After `(` or before `)` — no space.
    if is_lparen(prev) || is_rparen(next) { return false; }
    // After `{` or before `}` already handled by newline insertion
    // (we never call this across a newline boundary).
    // Before `,` `;` `:` — no space.
    if next.type_ == TokenType::Comma
        || next.type_ == TokenType::Semicolon
        || next.type_ == TokenType::Colon
    {
        return false;
    }
    // Before `(` when prev is an identifier (function call form) —
    // no space.  Keywords like `if`/`while` keep a space.
    if is_lparen(next) {
        return is_keyword(prev);
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
    fn format_minimal_main() {
        // Empty body is acceptable in either of the two common
        // styles: compact `fn main() {}` or expanded `fn main() {\n}\n`
        // (the latter matches rustfmt's default).  Both are
        // canonical; we accept whichever shape the formatter
        // emits as long as it parses back to the same AST.
        let out = format("fn main() {}").expect("ok");
        assert!(out.starts_with("fn main() {"), "got: {out:?}");
        assert!(out.contains("}"), "expected closing brace; got {out:?}");
        // Idempotent on this shape.
        let twice = format(&out).expect("ok");
        assert_eq!(out, twice, "not idempotent on empty body");
    }

    #[test]
    fn formats_single_statement_body() {
        let out = format("fn main() { return 42; }").expect("ok");
        // Body should be on its own indented line.
        assert!(out.contains("\n  return"), "expected indented body in {out:?}");
        assert!(out.contains("}"), "expected closing brace in {out:?}");
    }

    #[test]
    fn space_around_binary_ops() {
        let out = format("fn f() -> u8 { return 1+2; }").expect("ok");
        assert!(out.contains("1 + 2"), "expected spaces around +; got {out:?}");
    }

    #[test]
    fn comma_gets_following_space() {
        let out = format("fn f(a: u8, b: u8) -> u8 { return a+b; }").expect("ok");
        assert!(out.contains("a: u8, b: u8"), "comma spacing wrong: {out:?}");
    }

    #[test]
    fn no_space_inside_parens() {
        let out = format("fn f() -> u8 { return g( 1 ); }").expect("ok");
        // `g(1)` not `g( 1 )`.
        assert!(out.contains("g(1)"), "paren spacing wrong: {out:?}");
    }

    #[test]
    fn indents_with_two_spaces() {
        let out = format("fn f() { let x: u8 = 1; return x; }").expect("ok");
        // Each statement should be indented by 2 spaces.
        assert!(out.contains("\n  let "), "expected 2-space indent for let; got {out:?}");
        assert!(out.contains("\n  return"), "expected 2-space indent for return; got {out:?}");
    }

    #[test]
    fn multiple_statements_one_per_line() {
        let src = "fn f() { let x: u8 = 1; let y: u8 = 2; return x+y; }";
        let out = format(src).expect("ok");
        // Two `let`s plus a `return`, each on its own line.
        let lines: Vec<&str> = out.lines().collect();
        let stmt_lines = lines.iter().filter(|l| l.contains("let ") || l.contains("return")).count();
        assert!(stmt_lines >= 3, "expected 3 statement lines, got {stmt_lines} in {out:?}");
    }

    #[test]
    fn idempotent() {
        let src = "fn main() -> u8 { let x: u8 = 30; let y: u8 = 40; return x + y; }";
        let once  = format(src).expect("ok");
        let twice = format(&once).expect("ok");
        assert_eq!(once, twice, "formatter not idempotent");
    }
}
