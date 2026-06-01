//! # `nib-semantic-tokens` — semantic-token extraction for Nib.
//!
//! Walks the [`coding_adventures_nib_lexer`] token stream and emits
//! a typed token stream — keywords / booleans / numbers / types /
//! variables / operators / punctuation — suitable for **LSP
//! semantic-tokens**, syntax highlighters, and editor extensions.
//!
//! See [`README.md`](../README.md) for the API surface and the
//! token-classification table.
//!
//! ## Why "semantic" tokens rather than lex output directly
//!
//! The lexer's `type_name` is grammar-internal — useful for the
//! parser but not what an editor's theme expects.  This crate maps
//! those grammar names to a small, stable [`TokenKind`] enum that
//! lines up with LSP semantic-token-type conventions where the
//! meanings coincide (`keyword`, `number`, `string`, …).

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use coding_adventures_nib_lexer::tokenize_nib;
use lexer::token::{Token, TokenType};

// ===========================================================================
// TokenKind — semantic classification
// ===========================================================================

/// Semantic-token classifications surfaced by this crate.
///
/// `#[non_exhaustive]` — future variants (e.g. `Comment` once Nib's
/// lexer grows a trivia channel) won't break match-arm consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// Reserved Nib keyword: `fn`, `let`, `static`, `const`,
    /// `return`, `for`, `while`, `in`, `if`, `else`.
    Keyword,
    /// Boolean literal (`true`, `false`).
    Boolean,
    /// Integer literal (`INT_LIT` or `HEX_LIT`): `7`, `42`, `0xFF`.
    Number,
    /// Built-in type name (`u4`, `u8`, `u16`, `u32`, `bool`).
    /// Distinguished from `Variable` so themes can colour types
    /// specially.
    Type,
    /// User-defined identifier (function names, parameter names,
    /// `let`-bound names, variable references) that is not one of
    /// the recognised Nib types.
    Variable,
    /// Arithmetic, relational, or assignment operator (`+`, `-`,
    /// `*`, `/`, `=`, `<`, `>`, `==`, `!=`, …).
    Operator,
    /// Grouping or separation punctuation (`,`, `;`, `:`, `(`, `)`,
    /// `{`, `}`).
    Punctuation,
}

impl TokenKind {
    /// Stable string mnemonic — matches LSP semantic-token-type
    /// names where the meanings line up, lowercase.
    pub fn mnemonic(self) -> &'static str {
        match self {
            TokenKind::Keyword     => "keyword",
            TokenKind::Boolean     => "boolean",
            TokenKind::Number      => "number",
            TokenKind::Type        => "type",
            TokenKind::Variable    => "variable",
            TokenKind::Operator    => "operator",
            TokenKind::Punctuation => "punctuation",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ===========================================================================
// SemanticToken
// ===========================================================================

/// One semantic token — a position + length + kind triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticToken {
    /// 1-based source line.
    pub line: u32,
    /// 1-based starting column.
    pub column: u32,
    /// Token width in monospace cells.
    pub length: u32,
    /// Classification.
    pub kind: TokenKind,
}

// ===========================================================================
// Nib keyword + type sets
// ===========================================================================

/// The 10 reserved Nib keywords — anything in this set classifies
/// as [`TokenKind::Keyword`] (except `true`/`false`, which get
/// promoted to [`TokenKind::Boolean`]).  Mirrors the `keywords`
/// list in `coding_adventures_nib_lexer::_grammar`.
const NIB_KEYWORDS: &[&str] = &[
    "fn", "let", "static", "const", "return",
    "for", "while", "in", "if", "else",
];

/// Built-in Nib type names.  Recognised in NAME-position so the
/// editor can colour them as types rather than variables.
const NIB_TYPES: &[&str] = &["u4", "u8", "u16", "u32", "bool"];

fn is_nib_keyword(s: &str) -> bool { NIB_KEYWORDS.contains(&s) }
fn is_nib_type(s: &str)    -> bool { NIB_TYPES.contains(&s) }

// ===========================================================================
// Public entry points
// ===========================================================================

/// Tokenise `source` and return its semantic tokens in document
/// order.  The common case.
pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let tokens = tokenize_nib(source);
    tokens_from(&tokens)
}

/// Walk an already-tokenised stream and return its semantic tokens
/// in document order.  Skip the lex step when callers already have
/// a `Vec<Token>` in hand (e.g. an LSP server that also wants
/// diagnostic spans).
pub fn tokens_from(tokens: &[Token]) -> Vec<SemanticToken> {
    let mut out: Vec<SemanticToken> = Vec::with_capacity(tokens.len());
    for tok in tokens {
        if let Some(kind) = classify(tok) {
            out.push(SemanticToken {
                line:   tok.line as u32,
                column: tok.column as u32,
                length: token_length(tok) as u32,
                kind,
            });
        }
    }
    out
}

// ===========================================================================
// Classification
// ===========================================================================

/// Map a single lexer [`Token`] to a [`TokenKind`], or `None` if it
/// shouldn't be highlighted.
fn classify(tok: &Token) -> Option<TokenKind> {
    // 1.  Built-in operator / punctuation / trivia tokens are
    // distinguished by `TokenType`.
    match tok.type_ {
        TokenType::Newline | TokenType::Eof
        | TokenType::Indent | TokenType::Dedent
            => return None,
        TokenType::Plus
        | TokenType::Minus
        | TokenType::Star
        | TokenType::Slash
        | TokenType::Equals
        | TokenType::Bang
            => return Some(TokenKind::Operator),
        TokenType::Comma | TokenType::Semicolon | TokenType::Colon
            => return Some(TokenKind::Punctuation),
        _ => {}
    }

    // 2.  Otherwise check the grammar-attached `type_name`.  The
    // nib-lexer promotes keyword tokens' `type_name` to the literal
    // keyword text (e.g. "fn", "let"), so we check the keyword set
    // first.
    let etype = tok.effective_type_name();
    if etype == "true" || etype == "false" {
        return Some(TokenKind::Boolean);
    }
    if is_nib_keyword(etype) {
        return Some(TokenKind::Keyword);
    }
    match etype {
        "INT_LIT" | "HEX_LIT" => Some(TokenKind::Number),
        "NAME" => {
            // Types (u4, u8, u16, u32, bool) come through as plain
            // NAMEs in the grammar but should be coloured as types.
            if is_nib_type(&tok.value) {
                Some(TokenKind::Type)
            } else {
                Some(TokenKind::Variable)
            }
        }
        // Composite operator/punctuation tokens that the grammar
        // attaches custom names to (LE, GE, EQ, NE, LT, GT, ARROW,
        // LBRACE, RBRACE, LPAREN, RPAREN).  We classify by family.
        "LE" | "GE" | "EQ" | "NE" | "LT" | "GT" => Some(TokenKind::Operator),
        "ARROW" | "LBRACE" | "RBRACE" | "LPAREN" | "RPAREN"
            => Some(TokenKind::Punctuation),
        _ => None,
    }
}

/// Visible width of `tok` in monospace cells.
fn token_length(tok: &Token) -> usize {
    tok.value.chars().count()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_returns_in_document_order() {
        let toks = semantic_tokens("fn main() -> u8 { return 42; }");
        let mut last = (0u32, 0u32);
        for t in &toks {
            let here = (t.line, t.column);
            assert!(here >= last, "out of order: {last:?} -> {here:?}");
            last = here;
        }
    }

    #[test]
    fn classifies_fn_keyword() {
        let toks = semantic_tokens("fn main() {}");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Keyword && t.column == 1),
            "expected `fn` Keyword at col 1; got {toks:?}");
    }

    #[test]
    fn classifies_let_keyword() {
        let toks = semantic_tokens("fn f() { let x: u8 = 7; }");
        // Both `fn` and `let` should be Keywords.
        let kw_count = toks.iter().filter(|t| t.kind == TokenKind::Keyword).count();
        assert!(kw_count >= 2, "expected at least 2 Keywords (fn, let); got {kw_count}");
    }

    #[test]
    fn classifies_true_false_as_boolean() {
        let toks = semantic_tokens("fn f() -> bool { return true; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Boolean),
            "expected at least one Boolean; got {toks:?}");
        let toks2 = semantic_tokens("fn f() -> bool { return false; }");
        assert!(toks2.iter().any(|t| t.kind == TokenKind::Boolean),
            "expected Boolean for `false`; got {toks2:?}");
    }

    #[test]
    fn classifies_int_and_hex_literals_as_number() {
        let toks = semantic_tokens("fn f() { let x: u8 = 42; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Number),
            "expected a Number; got {toks:?}");
        let toks2 = semantic_tokens("fn f() { let x: u8 = 0xFF; }");
        assert!(toks2.iter().any(|t| t.kind == TokenKind::Number),
            "expected a Number for 0xFF; got {toks2:?}");
    }

    #[test]
    fn classifies_type_names() {
        // u4, u8, u16, u32, bool should be classified as Type.
        for ty in &["u4", "u8", "u16", "u32", "bool"] {
            let src = format!("fn f() {{ let x: {ty} = 1; }}");
            let toks = semantic_tokens(&src);
            assert!(toks.iter().any(|t| t.kind == TokenKind::Type),
                "expected Type for `{ty}`; got {toks:?}");
        }
    }

    #[test]
    fn classifies_user_name_as_variable() {
        let toks = semantic_tokens("fn main() { let x: u8 = 1; }");
        // `main` and `x` should be Variables.
        let vars: Vec<_> = toks.iter().filter(|t| t.kind == TokenKind::Variable)
            .collect();
        assert!(vars.len() >= 2, "expected at least 2 Variables; got {vars:?}");
    }

    #[test]
    fn classifies_operators_and_punctuation() {
        let toks = semantic_tokens("fn f() { let x: u8 = 1 + 2; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Operator),
            "expected Operator; got {toks:?}");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Punctuation),
            "expected Punctuation; got {toks:?}");
    }

    #[test]
    fn token_kind_mnemonic_stable() {
        assert_eq!(TokenKind::Keyword.mnemonic(),     "keyword");
        assert_eq!(TokenKind::Boolean.mnemonic(),     "boolean");
        assert_eq!(TokenKind::Number.mnemonic(),      "number");
        assert_eq!(TokenKind::Type.mnemonic(),        "type");
        assert_eq!(TokenKind::Variable.mnemonic(),    "variable");
        assert_eq!(TokenKind::Operator.mnemonic(),    "operator");
        assert_eq!(TokenKind::Punctuation.mnemonic(), "punctuation");
    }
}
