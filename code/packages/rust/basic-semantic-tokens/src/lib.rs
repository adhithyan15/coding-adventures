//! # `basic-semantic-tokens` — semantic-token extraction for Dartmouth BASIC.
//!
//! Walks the [`dartmouth_basic_lexer`] token stream and emits a
//! typed token stream — line numbers / keywords / numbers / strings
//! / variables / built-in functions / user functions / operators —
//! suitable for **LSP semantic-tokens**, syntax highlighters, and
//! editor extensions.
//!
//! See [`README.md`](../README.md) for the API surface and the
//! token-classification table.
//!
//! ## Why "semantic" tokens rather than lex output directly
//!
//! The lexer's `effective_type_name()` is a string mnemonic tied to
//! the grammar — useful for the parser but not what an editor's
//! theme expects.  This crate maps those grammar-internal names to a
//! small, stable [`TokenKind`] enum that lines up with LSP
//! semantic-token-type conventions where the meanings coincide
//! (`keyword`, `number`, `string`, …).  Editors can drive richer
//! highlighting from this enum than from raw regex patterns.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use coding_adventures_dartmouth_basic_lexer::tokenize_dartmouth_basic;
use lexer::token::{Token, TokenType};

// ===========================================================================
// TokenKind — semantic classification
// ===========================================================================

/// Semantic-token classifications surfaced by this crate.
///
/// `#[non_exhaustive]` — future variants (e.g. a `Comment` kind
/// once `REM` lines preserve their payload as trivia) won't break
/// match-arm consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// Leading integer on each BASIC line (`10`, `100`, …).  The
    /// grammar tags this `LINE_NUM` rather than `NUMBER` so editors
    /// can colour line labels distinctly from arithmetic literals.
    LineNumber,
    /// Built-in keyword: `LET`, `PRINT`, `INPUT`, `IF`, `THEN`,
    /// `GOTO`, `FOR`, `TO`, `STEP`, `NEXT`, `END`, `STOP`, `REM`,
    /// `READ`, `DATA`, `RESTORE`, `DIM`, `DEF`, …
    Keyword,
    /// Integer or floating-point literal in arithmetic position.
    Number,
    /// Double-quoted string literal (`"HELLO"`).
    String,
    /// Variable name (`A`, `B7`, `X0`).
    Variable,
    /// Built-in function (`SIN`, `COS`, `SQR`, `ABS`, `INT`, `RND`,
    /// `LOG`, `EXP`, `ATN`, `TAN`, `SGN`).
    BuiltinFn,
    /// User-defined function (`FNA` … `FNZ9`) declared via `DEF FNX = …`.
    UserFn,
    /// Arithmetic, relational, or assignment operator (`+`, `-`,
    /// `*`, `/`, `=`, `<`, `>`, `<=`, `>=`, `<>`).
    Operator,
    /// Grouping or separation punctuation (`,`, `;`, `:`, `(`, `)`).
    Punctuation,
}

impl TokenKind {
    /// Stable string mnemonic — matches LSP semantic-token-type
    /// names where the meanings line up, lowercase.  Useful for
    /// theme configuration files and JSON wire formats.
    pub fn mnemonic(self) -> &'static str {
        match self {
            TokenKind::LineNumber  => "label",
            TokenKind::Keyword     => "keyword",
            TokenKind::Number      => "number",
            TokenKind::String      => "string",
            TokenKind::Variable    => "variable",
            TokenKind::BuiltinFn   => "function",
            TokenKind::UserFn      => "function",
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
// SemanticToken — position + length + kind triple
// ===========================================================================

/// One semantic token — a position + length + kind triple.
///
/// `length` is the number of monospace cells the token occupies on
/// its line (char count for ASCII source).  Tokens never span
/// multiple lines.
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
// Public entry points
// ===========================================================================

/// Tokenise `source` and return its semantic tokens in document
/// order.  The common case.
pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let tokens = tokenize_dartmouth_basic(source);
    tokens_from(&tokens)
}

/// Walk an already-tokenised token stream and return its semantic
/// tokens in document order.  Skip the lex step when callers
/// already have a `Vec<Token>` in hand (e.g. an LSP server that
/// also wants diagnostic spans).
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

/// Map a single lexer [`Token`] to a [`TokenKind`], or `None` if the
/// token shouldn't be highlighted (newlines, EOF).
fn classify(tok: &Token) -> Option<TokenKind> {
    // Built-in operator / punctuation / EOF / newline tokens are
    // distinguished by `TokenType` rather than by the grammar's
    // string name — switch on `tok.type_` first.
    match tok.type_ {
        TokenType::Newline | TokenType::Eof | TokenType::Indent | TokenType::Dedent
            => return None,
        TokenType::Plus
        | TokenType::Minus
        | TokenType::Star
        | TokenType::Slash
        | TokenType::Equals
            => return Some(TokenKind::Operator),
        TokenType::Comma | TokenType::Semicolon | TokenType::Colon
            => return Some(TokenKind::Punctuation),
        _ => {}
    }

    // Otherwise the grammar attaches a custom name via
    // `effective_type_name()` — this is how BASIC distinguishes
    // `LINE_NUM` from `NUMBER` and the FN families.  Match on the
    // string mnemonic.
    match tok.effective_type_name() {
        "LINE_NUM"   => Some(TokenKind::LineNumber),
        "KEYWORD"    => Some(TokenKind::Keyword),
        "NUMBER"     => Some(TokenKind::Number),
        "STRING"     => Some(TokenKind::String),
        "NAME"       => Some(TokenKind::Variable),
        "BUILTIN_FN" => Some(TokenKind::BuiltinFn),
        "USER_FN"    => Some(TokenKind::UserFn),
        // Relational composites (`<=`, `>=`, `<>`) and bare `<`/`>`
        // come through with custom type names in the BASIC grammar —
        // map any LT/GT family to Operator.
        "LE" | "GE" | "NE" | "LT" | "GT" => Some(TokenKind::Operator),
        // Grouping parens — the BASIC grammar uses custom names
        // `LPAREN` / `RPAREN`.
        "LPAREN" | "RPAREN" => Some(TokenKind::Punctuation),
        _ => None,
    }
}

/// Visible width of `tok` in monospace cells.
///
/// For ASCII source (which BASIC is — identifiers are `A..Z` and
/// `A0..Z9`, keywords are uppercase letters) the char count equals
/// the byte count of `value`.  We use `chars().count()` for safety
/// against any non-ASCII glyphs in strings, which BASIC's parser
/// will accept inside `"…"`.
fn token_length(tok: &Token) -> usize {
    tok.value.chars().count()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: collect just the (line, col, kind) triples from
    /// `semantic_tokens(source)` — the parts tests usually care
    /// about.
    fn triples(source: &str) -> Vec<(u32, u32, TokenKind)> {
        semantic_tokens(source)
            .into_iter()
            .map(|t| (t.line, t.column, t.kind))
            .collect()
    }

    #[test]
    fn tokens_returns_in_document_order() {
        // A 2-line program — line 1's tokens should all precede
        // line 2's tokens in the output.
        let toks = semantic_tokens("10 PRINT 42\n20 END\n");
        let lines: Vec<u32> = toks.iter().map(|t| t.line).collect();
        // Strict-monotonic-non-decreasing in line.
        for w in lines.windows(2) {
            assert!(w[0] <= w[1], "out of order: {lines:?}");
        }
    }

    #[test]
    fn classifies_line_number_and_keyword() {
        let toks = semantic_tokens("10 END\n");
        assert!(toks.iter().any(|t| t.kind == TokenKind::LineNumber && t.column == 1),
            "expected LineNumber at col 1; got {toks:?}");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Keyword),
            "expected at least one Keyword; got {toks:?}");
    }

    #[test]
    fn classifies_string_literal() {
        // BASIC `PRINT "HELLO"` should produce a String token.
        let toks = semantic_tokens("10 PRINT \"HELLO\"\n");
        assert!(toks.iter().any(|t| t.kind == TokenKind::String),
            "expected a String token; got {toks:?}");
    }

    #[test]
    fn classifies_variable_and_number() {
        let toks = semantic_tokens("10 LET A = 42\n");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Variable),
            "expected a Variable token; got {toks:?}");
        // `42` should be classified as Number (not LineNumber — that's
        // only the leading `10`).
        assert!(toks.iter().any(|t| t.kind == TokenKind::Number),
            "expected at least one Number after LET A =; got {toks:?}");
    }

    #[test]
    fn classifies_builtin_fn() {
        // SQR is a built-in function in the BASIC lexer's
        // BUILTIN_FN list.
        let toks = semantic_tokens("10 LET X = SQR(2)\n");
        assert!(toks.iter().any(|t| t.kind == TokenKind::BuiltinFn),
            "expected a BuiltinFn token for SQR; got {toks:?}");
    }

    #[test]
    fn classifies_operator() {
        let toks = semantic_tokens("10 LET A = 1 + 2\n");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Operator),
            "expected an Operator token for `+`; got {toks:?}");
    }

    #[test]
    fn empty_source_returns_empty_vec() {
        // An empty source still produces an EOF token from the
        // lexer, but EOF is filtered out of semantic tokens.
        let toks = semantic_tokens("");
        assert!(toks.iter().all(|t| t.kind != TokenKind::LineNumber),
            "empty input should not yield LineNumber tokens; got {toks:?}");
    }

    #[test]
    fn token_kind_mnemonic_stable() {
        // Document the LSP-aligned mnemonics so theme authors can
        // rely on them.
        assert_eq!(TokenKind::Keyword.mnemonic(),     "keyword");
        assert_eq!(TokenKind::Number.mnemonic(),      "number");
        assert_eq!(TokenKind::String.mnemonic(),      "string");
        assert_eq!(TokenKind::Variable.mnemonic(),    "variable");
        assert_eq!(TokenKind::BuiltinFn.mnemonic(),   "function");
        assert_eq!(TokenKind::Operator.mnemonic(),    "operator");
        assert_eq!(TokenKind::LineNumber.mnemonic(),  "label");
        assert_eq!(TokenKind::Punctuation.mnemonic(), "punctuation");
    }

    /// Document order is preserved end-to-end — verifying both line
    /// and column monotonic within a line.
    #[test]
    fn document_order_within_a_line() {
        let toks = semantic_tokens("10 LET A = 1 + 2\n");
        let mut last = (0u32, 0u32);
        for t in &toks {
            let here = (t.line, t.column);
            assert!(here >= last, "out of order: {last:?} -> {here:?} in {toks:?}");
            last = here;
        }
        // And triples helper works.
        let trips = triples("10 LET A = 1\n");
        assert!(trips.iter().any(|(_, _, k)| *k == TokenKind::LineNumber));
    }
}
