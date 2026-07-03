//! # `oct-semantic-tokens` — semantic-token extraction for Oct.
//!
//! Walks the [`coding_adventures_oct_lexer`] token stream and emits
//! a typed token stream suitable for **LSP semantic-tokens**,
//! syntax highlighters, and editor extensions.
//!
//! See [`README.md`](../README.md) for the API surface and the
//! token-classification table.
//!
//! ## Oct-specific: hardware intrinsics
//!
//! Oct is a typed 8-bit systems language that originated targeting
//! the Intel 8008.  The lexer's keyword set includes the 8008
//! hardware intrinsics (`in`, `out`, `adc`, `sbb`, `rlc`, `rrc`,
//! `ral`, `rar`, `carry`, `parity`).  Mixing those into the same
//! `Keyword` bucket as control-flow keywords (`if`, `while`,
//! `return`) hides a meaningful distinction.  We give them their
//! own [`TokenKind::Intrinsic`] so editors can colour CPU ops
//! distinctly — useful when reading Oct kernel/driver code.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use coding_adventures_oct_lexer::tokenize_oct;
use lexer::token::{Token, TokenType};

// ===========================================================================
// TokenKind
// ===========================================================================

/// Semantic-token classifications surfaced by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// Reserved control-flow / declaration keyword: `fn`, `let`,
    /// `static`, `if`, `else`, `while`, `loop`, `break`, `return`.
    Keyword,
    /// 8008 hardware intrinsic keyword: `in`, `out`, `adc`, `sbb`,
    /// `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`.  Separated
    /// from [`TokenKind::Keyword`] so editor themes can colour CPU
    /// ops distinctly.
    Intrinsic,
    /// Boolean literal (`true`, `false`).
    Boolean,
    /// Integer literal — decimal (`42`), hex (`0xFF`), or binary
    /// (`0b1010`).
    Number,
    /// Built-in type name (`u8`, `bool`).
    Type,
    /// User-defined identifier (function names, parameter names,
    /// `let`-bound names, variable references).
    Variable,
    /// Arithmetic, relational, assignment, or bitwise operator.
    Operator,
    /// Grouping / separation punctuation.
    Punctuation,
    /// Line comment payload (`// …` to end of line).
    Comment,
}

impl TokenKind {
    /// Stable string mnemonic.
    pub fn mnemonic(self) -> &'static str {
        match self {
            TokenKind::Keyword     => "keyword",
            TokenKind::Intrinsic   => "macro",
            TokenKind::Boolean     => "boolean",
            TokenKind::Number      => "number",
            TokenKind::Type        => "type",
            TokenKind::Variable    => "variable",
            TokenKind::Operator    => "operator",
            TokenKind::Punctuation => "punctuation",
            TokenKind::Comment     => "comment",
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
// Oct keyword sets
// ===========================================================================

/// Control-flow / declaration keywords — get [`TokenKind::Keyword`].
const OCT_KEYWORDS: &[&str] = &[
    "fn", "let", "static",
    "if", "else", "while", "loop", "break", "return",
];

/// 8008 hardware intrinsics — get [`TokenKind::Intrinsic`].  Oct
/// classifies these as KEYWORD in the lexer (they're reserved
/// words), but semantically they're closer to CPU ops than to
/// control-flow keywords.
const OCT_INTRINSICS: &[&str] = &[
    "in", "out", "adc", "sbb", "rlc", "rrc", "ral", "rar",
    "carry", "parity",
];

/// Built-in Oct types — get [`TokenKind::Type`].  Oct V1 has only
/// `u8` and `bool` at the source level (narrower types like u4 are
/// reserved for future versions); both come through as NAME tokens.
const OCT_TYPES: &[&str] = &["u8", "bool"];

fn is_oct_keyword(s: &str)   -> bool { OCT_KEYWORDS.contains(&s) }
fn is_oct_intrinsic(s: &str) -> bool { OCT_INTRINSICS.contains(&s) }
fn is_oct_type(s: &str)      -> bool { OCT_TYPES.contains(&s) }

// ===========================================================================
// Public entry points
// ===========================================================================

/// Tokenise `source` and return its semantic tokens in document
/// order.
pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let tokens = tokenize_oct(source);
    tokens_from(&tokens)
}

/// Walk an already-tokenised stream and return its semantic tokens
/// in document order.
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

fn classify(tok: &Token) -> Option<TokenKind> {
    // 1.  Built-in trivia tokens are skipped.
    match tok.type_ {
        TokenType::Newline | TokenType::Eof
        | TokenType::Indent | TokenType::Dedent
            => return None,
        // Built-in operators / punctuation that come through with a
        // standard TokenType (mostly fallbacks for the grammar's
        // custom-named variants — Oct's grammar attaches custom
        // names to almost every operator, so these rarely fire).
        TokenType::Plus | TokenType::Minus
        | TokenType::Star | TokenType::Slash
        | TokenType::Equals | TokenType::Bang
            => return Some(TokenKind::Operator),
        TokenType::Comma | TokenType::Semicolon | TokenType::Colon
            => return Some(TokenKind::Punctuation),
        _ => {}
    }

    // 2.  Grammar-attached type names — the common path.
    let etype = tok.effective_type_name();

    // Booleans
    if etype == "true" || etype == "false" {
        return Some(TokenKind::Boolean);
    }
    // Intrinsics (checked before Keyword because the intersection is
    // empty — there's no `in`/`out` in OCT_KEYWORDS — but listing
    // intrinsics first makes the intent explicit).
    if is_oct_intrinsic(etype) {
        return Some(TokenKind::Intrinsic);
    }
    if is_oct_keyword(etype) {
        return Some(TokenKind::Keyword);
    }

    match etype {
        "INT_LIT" | "HEX_LIT" | "BIN_LIT" => Some(TokenKind::Number),
        "NAME" => {
            if is_oct_type(&tok.value) {
                Some(TokenKind::Type)
            } else {
                Some(TokenKind::Variable)
            }
        }
        "LINE_COMMENT" => Some(TokenKind::Comment),
        // Custom-named operators
        "EQ" | "EQ_EQ" | "NEQ" | "LEQ" | "GEQ" | "LT" | "GT"
        | "LAND" | "LOR"
        | "PLUS" | "MINUS" | "AMP" | "PIPE" | "CARET"
        | "TILDE" | "BANG"
            => Some(TokenKind::Operator),
        // Custom-named punctuation
        "ARROW" | "LBRACE" | "RBRACE" | "LPAREN" | "RPAREN"
        | "COLON" | "SEMICOLON" | "COMMA"
            => Some(TokenKind::Punctuation),
        // Drop whitespace explicitly (it usually never reaches
        // tokens_from but be defensive).
        "WHITESPACE" => None,
        _ => None,
    }
}

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
        let toks = semantic_tokens("fn main() { let x: u8 = 42; }");
        let mut last = (0u32, 0u32);
        for t in &toks {
            let here = (t.line, t.column);
            assert!(here >= last, "out of order: {last:?} -> {here:?}");
            last = here;
        }
    }

    #[test]
    fn classifies_keywords() {
        let toks = semantic_tokens("fn main() { if true { return; } }");
        let keyword_count = toks.iter().filter(|t| t.kind == TokenKind::Keyword).count();
        // `fn`, `if`, `return` — 3 keywords.
        assert!(keyword_count >= 3, "expected at least 3 keywords; got {keyword_count}: {toks:?}");
    }

    #[test]
    fn classifies_intrinsics() {
        let toks = semantic_tokens("fn t() { out(1, 0); let c: bool = carry(); }");
        // Should have at least 2 Intrinsics (`out`, `carry`).
        let intrinsic_count = toks.iter().filter(|t| t.kind == TokenKind::Intrinsic).count();
        assert!(intrinsic_count >= 2,
            "expected at least 2 Intrinsics; got {intrinsic_count}: {toks:?}");
    }

    #[test]
    fn classifies_true_false_as_boolean() {
        let toks = semantic_tokens("fn t() -> bool { return true; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Boolean),
            "expected a Boolean for `true`; got {toks:?}");
        let toks2 = semantic_tokens("fn t() -> bool { return false; }");
        assert!(toks2.iter().any(|t| t.kind == TokenKind::Boolean),
            "expected a Boolean for `false`; got {toks2:?}");
    }

    #[test]
    fn classifies_int_hex_bin_literals_as_number() {
        for src in &[
            "fn t() { let x: u8 = 42; }",
            "fn t() { let x: u8 = 0xFF; }",
            "fn t() { let x: u8 = 0b1010; }",
        ] {
            let toks = semantic_tokens(src);
            assert!(toks.iter().any(|t| t.kind == TokenKind::Number),
                "expected a Number in {src:?}; got {toks:?}");
        }
    }

    #[test]
    fn classifies_type_names() {
        for ty in &["u8", "bool"] {
            let src = format!("fn t() {{ let x: {ty} = 1; }}");
            let toks = semantic_tokens(&src);
            assert!(toks.iter().any(|t| t.kind == TokenKind::Type),
                "expected Type for {ty}; got {toks:?}");
        }
    }

    #[test]
    fn classifies_user_name_as_variable() {
        let toks = semantic_tokens("fn main() { let x: u8 = 1; }");
        let vars: Vec<_> = toks.iter().filter(|t| t.kind == TokenKind::Variable).collect();
        // `main` and `x` should both be Variables.
        assert!(vars.len() >= 2, "expected at least 2 Variables; got {vars:?}");
    }

    #[test]
    fn classifies_operators_and_punctuation() {
        let toks = semantic_tokens("fn t() { let x: u8 = 1 + 2; if x == 3 { x = x - 1; } }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Operator),
            "expected Operator; got {toks:?}");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Punctuation),
            "expected Punctuation; got {toks:?}");
    }

    #[test]
    fn token_kind_mnemonic_stable() {
        assert_eq!(TokenKind::Keyword.mnemonic(),     "keyword");
        assert_eq!(TokenKind::Intrinsic.mnemonic(),   "macro");
        assert_eq!(TokenKind::Boolean.mnemonic(),     "boolean");
        assert_eq!(TokenKind::Number.mnemonic(),      "number");
        assert_eq!(TokenKind::Type.mnemonic(),        "type");
        assert_eq!(TokenKind::Variable.mnemonic(),    "variable");
        assert_eq!(TokenKind::Operator.mnemonic(),    "operator");
        assert_eq!(TokenKind::Punctuation.mnemonic(), "punctuation");
        assert_eq!(TokenKind::Comment.mnemonic(),     "comment");
    }

    /// Intrinsic vs Keyword separation — both classes must appear in
    /// the same program and be distinguished.
    #[test]
    fn keyword_and_intrinsic_are_distinct() {
        let toks = semantic_tokens("fn t() { if true { out(1, 0); } }");
        let has_kw  = toks.iter().any(|t| t.kind == TokenKind::Keyword);
        let has_int = toks.iter().any(|t| t.kind == TokenKind::Intrinsic);
        assert!(has_kw  && has_int,
            "expected both Keyword AND Intrinsic in same program; got {toks:?}");
    }
}
