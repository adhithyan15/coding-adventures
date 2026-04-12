//! # ALGOL 60 Lexer — tokenizing ALGOL 60 source text.
//!
//! [ALGOL 60](https://en.wikipedia.org/wiki/ALGOL_60) (ALGOrithmic Language,
//! 1960) was the first programming language to be formally specified using BNF
//! (Backus-Naur Form). It introduced block structure, lexical scoping,
//! recursion, and the call stack — concepts every modern language inherits.
//! ALGOL 60 is the common ancestor of Pascal, C, Ada, Simula (the first OOP
//! language), and indirectly Java, Rust, Go, and Swift.
//!
//! This crate provides a lexer (tokenizer) for ALGOL 60. It does **not**
//! hand-write tokenization rules. Instead, it loads the `algol.tokens`
//! grammar file — a declarative description of every token in ALGOL 60 —
//! and feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! algol.tokens         (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for ALGOL 60 specifically. It knows where to find `algol.tokens` and
//! provides two public entry points:
//!
//! - [`create_algol_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_algol`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Token highlights
//!
//! ALGOL 60 has several lexical features worth noting:
//!
//! - **`:=` for assignment** — avoids the C bug of `=` (assignment) vs `==`
//!   (equality). ALGOL uses `:=` for assignment and `=` for equality comparison.
//!
//! - **`**` for exponentiation** — ALGOL originally used the mathematical
//!   uparrow symbol `↑`. ASCII implementations standardized on `**` (Fortran
//!   convention) or `^` (caret). Both are supported.
//!
//! - **Word-based boolean operators** — `and`, `or`, `not`, `impl`, `eqv`
//!   instead of `&&`, `||`, `!`. This reads much more like mathematics.
//!
//! - **Integer division as `div`** — distinguishes exact integer division from
//!   floating-point division `/`. Similarly `mod` for remainder.
//!
//! - **Comments via the `comment` keyword** — everything from `comment` up to
//!   the next `;` is consumed silently. No `//` or `/* */` syntax.
//!
//! - **Single-quoted strings** — unlike C and JSON which use double quotes.
//!   ALGOL 60 strings have no escape sequences — a single quote cannot appear
//!   inside a string literal.
//!
//! # Why grammar-driven instead of hand-written?
//!
//! A hand-written lexer for ALGOL would need to handle keyword disambiguation
//! (is `beginning` a keyword or an identifier?), the ordering constraint on
//! multi-character operators (`:=` before `:`), and comment skipping — all
//! character-by-character. The grammar-driven approach replaces all that logic
//! with the `algol.tokens` grammar file. When a new operator is added, you
//! edit the grammar — no Rust code changes needed.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for ALGOL 60 source text.
///
/// This function:
/// 1. Reads the `algol.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed. This should never
/// happen in practice — the grammar file is checked into the repository and
/// validated by the grammar-tools test suite. A panic here indicates a
/// broken build or missing file.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_algol_lexer::create_algol_lexer;
///
/// let mut lexer = create_algol_lexer("begin integer x; x := 42 end");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_algol_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize ALGOL 60 source text into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// Comments (from the `comment` keyword to the next `;`) are silently
/// consumed and do not appear in the token stream. Whitespace is also
/// skipped.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character (via `LexerError` propagation).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_algol_lexer::tokenize_algol;
///
/// let tokens = tokenize_algol("begin integer x; x := 42 end");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_algol(source: &str) -> Vec<Token> {
    // Create a fresh lexer for this source text.
    let mut algol_lexer = create_algol_lexer(source);

    // Tokenize and unwrap — any LexerError becomes a panic.
    //
    // In a production tool, you would want to propagate the error
    // via Result. For this educational codebase, panicking with a clear
    // message is sufficient and keeps the API simple.
    algol_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("ALGOL 60 tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect (type_, type_name, value) triples excluding EOF.
    //
    // ALGOL 60 uses many custom token types that don't map to the built-in
    // TokenType enum. For those, `type_` is TokenType::Name and `type_name`
    // carries the grammar name (e.g. Some("ASSIGN"), Some("INTEGER_LIT")).
    //
    // Built-in mappings used by ALGOL:
    //   PLUS      -> TokenType::Plus      (type_name: None)
    //   MINUS     -> TokenType::Minus     (type_name: None)
    //   STAR      -> TokenType::Star      (type_name: None)
    //   SLASH     -> TokenType::Slash     (type_name: None)
    //   LPAREN    -> TokenType::LParen    (type_name: None)
    //   RPAREN    -> TokenType::RParen    (type_name: None)
    //   LBRACKET  -> TokenType::LBracket  (type_name: None)
    //   RBRACKET  -> TokenType::RBracket  (type_name: None)
    //   SEMICOLON -> TokenType::Semicolon (type_name: None)
    //   COMMA     -> TokenType::Comma     (type_name: None)
    //   COLON     -> TokenType::Colon     (type_name: None)
    //
    // Custom (type_ == Name, type_name == Some("...")):
    //   NAME (identifiers), INTEGER_LIT, REAL_LIT, STRING_LIT
    //   ASSIGN, POWER, LEQ, GEQ, NEQ, CARET, EQ, LT, GT
    //
    // Keywords (type_ == Keyword, type_name == Some("KEYWORD")):
    //   begin, end, if, then, else, for, do, step, until, while,
    //   goto, switch, procedure, integer, real, boolean, string,
    //   array, own, label, value, true, false, not, and, or,
    //   impl, eqv, div, mod
    // -----------------------------------------------------------------------

    /// Return (effective_type_name, value) pairs for non-EOF tokens.
    /// Uses `token.effective_type_name()` which returns the grammar name
    /// for custom types and the built-in name for standard types.
    fn token_pairs(tokens: &[Token]) -> Vec<(&str, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name(), t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Integer literal
    // -----------------------------------------------------------------------

    /// An integer literal tokenizes as INTEGER_LIT (custom type).
    /// The `type_name` field carries "INTEGER_LIT".
    #[test]
    fn test_tokenize_integer_literal() {
        let tokens = tokenize_algol("42");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "INTEGER_LIT");
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 2: Zero
    // -----------------------------------------------------------------------

    /// Zero is a valid integer literal in ALGOL 60.
    #[test]
    fn test_tokenize_zero() {
        let tokens = tokenize_algol("0");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "INTEGER_LIT");
        assert_eq!(pairs[0].1, "0");
    }

    // -----------------------------------------------------------------------
    // Test 3: Large integer
    // -----------------------------------------------------------------------

    /// Larger integers are also INTEGER_LIT.
    #[test]
    fn test_tokenize_large_integer() {
        let tokens = tokenize_algol("1000");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "INTEGER_LIT");
        assert_eq!(pairs[0].1, "1000");
    }

    // -----------------------------------------------------------------------
    // Test 4: Real literal — decimal
    // -----------------------------------------------------------------------

    /// A real (floating-point) literal with a decimal point.
    /// REAL_LIT must come before INTEGER_LIT in the grammar so that
    /// "3.14" is not tokenized as INTEGER_LIT("3") + something.
    #[test]
    fn test_tokenize_real_decimal() {
        let tokens = tokenize_algol("3.14");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "REAL_LIT");
        assert_eq!(pairs[0].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 5: Real literal — integer with exponent
    // -----------------------------------------------------------------------

    /// An integer with an exponent part: 1.5E3 means 1500.0.
    #[test]
    fn test_tokenize_real_exponent() {
        let tokens = tokenize_algol("1.5E3");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "REAL_LIT");
        assert_eq!(pairs[0].1, "1.5E3");
    }

    // -----------------------------------------------------------------------
    // Test 6: Real literal — negative exponent
    // -----------------------------------------------------------------------

    /// 1.5E-3 means 0.0015 — a negative exponent shifts the decimal point left.
    #[test]
    fn test_tokenize_real_negative_exponent() {
        let tokens = tokenize_algol("1.5E-3");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "REAL_LIT");
        assert_eq!(pairs[0].1, "1.5E-3");
    }

    // -----------------------------------------------------------------------
    // Test 7: Real literal — integer + exponent, no decimal point
    // -----------------------------------------------------------------------

    /// 100E2 is a valid ALGOL real: integer part with exponent, no dot.
    #[test]
    fn test_tokenize_real_integer_exponent() {
        let tokens = tokenize_algol("100E2");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "REAL_LIT");
        assert_eq!(pairs[0].1, "100E2");
    }

    // -----------------------------------------------------------------------
    // Test 8: String literal
    // -----------------------------------------------------------------------

    /// ALGOL 60 strings are single-quoted. Unlike C/JSON, there are no
    /// escape sequences — a single quote cannot appear inside the string.
    #[test]
    fn test_tokenize_string_literal() {
        let tokens = tokenize_algol("'hello'");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "STRING_LIT");
        // The grammar's string pattern strips the quotes
        // (or keeps them, depending on grammar-tools behavior)
        // so we check the value contains "hello".
        assert!(
            pairs[0].1.contains("hello"),
            "Expected string value to contain 'hello', got {:?}",
            pairs[0].1
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Empty string literal
    // -----------------------------------------------------------------------

    /// An empty string literal `''` is valid in ALGOL 60.
    #[test]
    fn test_tokenize_empty_string() {
        let tokens = tokenize_algol("''");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "STRING_LIT");
    }

    // -----------------------------------------------------------------------
    // Test 10: Simple identifier
    // -----------------------------------------------------------------------

    /// A single-letter identifier tokenizes as IDENT.
    #[test]
    fn test_tokenize_identifier_single() {
        let tokens = tokenize_algol("x");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[0].1, "x");
    }

    // -----------------------------------------------------------------------
    // Test 11: Multi-character identifier
    // -----------------------------------------------------------------------

    /// Identifiers can be multiple letters and digits (after the first letter).
    #[test]
    fn test_tokenize_identifier_multi() {
        let tokens = tokenize_algol("sum");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[0].1, "sum");
    }

    // -----------------------------------------------------------------------
    // Test 12: Mixed-case identifier
    // -----------------------------------------------------------------------

    /// ALGOL identifiers can mix upper and lowercase letters.
    #[test]
    fn test_tokenize_identifier_mixed_case() {
        let tokens = tokenize_algol("customerName");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[0].1, "customerName");
    }

    // -----------------------------------------------------------------------
    // Test 13: Identifier with digit
    // -----------------------------------------------------------------------

    /// Identifiers can contain digits after the first letter.
    #[test]
    fn test_tokenize_identifier_with_digit() {
        let tokens = tokenize_algol("A1");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[0].1, "A1");
    }

    // -----------------------------------------------------------------------
    // Test 14: Keyword — begin
    // -----------------------------------------------------------------------

    /// `begin` is a keyword, not an identifier. The lexer promotes it from
    /// IDENT to KEYWORD. The token type is TokenType::Keyword.
    #[test]
    fn test_tokenize_keyword_begin() {
        let tokens = tokenize_algol("begin");
        let non_eof: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();

        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].type_, TokenType::Keyword);
        assert_eq!(non_eof[0].value, "begin");
    }

    // -----------------------------------------------------------------------
    // Test 15: Keyword — end
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_keyword_end() {
        let tokens = tokenize_algol("end");
        let non_eof: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();

        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].type_, TokenType::Keyword);
        assert_eq!(non_eof[0].value, "end");
    }

    // -----------------------------------------------------------------------
    // Test 16: All keywords
    // -----------------------------------------------------------------------

    /// Verify that every ALGOL 60 keyword produces a KEYWORD token.
    /// This ensures none are accidentally treated as identifiers.
    #[test]
    fn test_tokenize_all_keywords() {
        let keywords = [
            "begin",
            "end",
            "if",
            "then",
            "else",
            "for",
            "do",
            "step",
            "until",
            "while",
            "goto",
            "switch",
            "procedure",
            "integer",
            "real",
            "boolean",
            "string",
            "array",
            "own",
            "label",
            "value",
            "true",
            "false",
            "not",
            "and",
            "or",
            "impl",
            "eqv",
            "div",
            "mod",
        ];

        for kw in &keywords {
            let tokens = tokenize_algol(kw);
            let non_eof: Vec<&Token> = tokens
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .collect();

            assert_eq!(non_eof.len(), 1, "Expected 1 token for keyword '{kw}'");
            assert_eq!(
                non_eof[0].type_,
                TokenType::Keyword,
                "Expected KEYWORD type for '{kw}', got {:?}",
                non_eof[0].type_
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: Keyword boundary — "beginning" is not a keyword
    // -----------------------------------------------------------------------

    /// The identifier "beginning" starts with "begin" but is NOT the keyword
    /// `begin`. The lexer must match the full token, not a prefix. So
    /// "beginning" should produce a single IDENT token, not BEGIN + IDENT.
    #[test]
    fn test_keyword_boundary() {
        let tokens = tokenize_algol("beginning");
        let non_eof: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();

        assert_eq!(
            non_eof.len(),
            1,
            "Expected 1 token, not BEGIN + IDENT('ning')"
        );
        assert_eq!(
            non_eof[0].type_,
            TokenType::Name,
            "Expected IDENT (Name type), got {:?}",
            non_eof[0].type_
        );
        assert_eq!(non_eof[0].value, "beginning");
    }

    // -----------------------------------------------------------------------
    // Test 18: Operator — ASSIGN (:=)
    // -----------------------------------------------------------------------

    /// `:=` is the assignment operator — distinct from `:` (colon).
    /// The grammar lists ASSIGN before COLON so `:=` wins on first match.
    #[test]
    fn test_tokenize_assign() {
        let tokens = tokenize_algol(":=");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "ASSIGN");
        assert_eq!(pairs[0].1, ":=");
    }

    // -----------------------------------------------------------------------
    // Test 19: Operator — POWER (**)
    // -----------------------------------------------------------------------

    /// `**` is the exponentiation operator — distinct from `*` (multiply).
    /// The grammar lists POWER before STAR so `**` wins on first match.
    #[test]
    fn test_tokenize_power() {
        let tokens = tokenize_algol("**");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "POWER");
        assert_eq!(pairs[0].1, "**");
    }

    // -----------------------------------------------------------------------
    // Test 20: Operator — LEQ (<=)
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_leq() {
        let tokens = tokenize_algol("<=");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "LEQ");
        assert_eq!(pairs[0].1, "<=");
    }

    // -----------------------------------------------------------------------
    // Test 21: Operator — GEQ (>=)
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_geq() {
        let tokens = tokenize_algol(">=");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "GEQ");
        assert_eq!(pairs[0].1, ">=");
    }

    // -----------------------------------------------------------------------
    // Test 22: Operator — NEQ (!=)
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_neq() {
        let tokens = tokenize_algol("!=");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "NEQ");
        assert_eq!(pairs[0].1, "!=");
    }

    // -----------------------------------------------------------------------
    // Test 23: Single-character arithmetic operators
    // -----------------------------------------------------------------------

    /// PLUS, MINUS, STAR, SLASH map directly to built-in TokenType variants.
    #[test]
    fn test_tokenize_arithmetic_operators() {
        // PLUS maps to TokenType::Plus
        let tokens_plus = tokenize_algol("+");
        let non_eof: Vec<&Token> = tokens_plus
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[0].type_, TokenType::Plus);

        // MINUS maps to TokenType::Minus
        let tokens_minus = tokenize_algol("-");
        let non_eof: Vec<&Token> = tokens_minus
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[0].type_, TokenType::Minus);

        // STAR maps to TokenType::Star
        let tokens_star = tokenize_algol("*");
        let non_eof: Vec<&Token> = tokens_star
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[0].type_, TokenType::Star);

        // SLASH maps to TokenType::Slash
        let tokens_slash = tokenize_algol("/");
        let non_eof: Vec<&Token> = tokens_slash
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[0].type_, TokenType::Slash);
    }

    // -----------------------------------------------------------------------
    // Test 24: CARET operator
    // -----------------------------------------------------------------------

    /// `^` is an alternative exponentiation operator (alongside `**`).
    #[test]
    fn test_tokenize_caret() {
        let tokens = tokenize_algol("^");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "CARET");
        assert_eq!(pairs[0].1, "^");
    }

    // -----------------------------------------------------------------------
    // Test 25: EQ, LT, GT operators
    // -----------------------------------------------------------------------

    /// `=` is the equality test (not assignment), `<` and `>` are comparisons.
    #[test]
    fn test_tokenize_relational_operators() {
        let tokens_eq = tokenize_algol("=");
        let pairs_eq = token_pairs(&tokens_eq);
        assert_eq!(pairs_eq[0].0, "EQ");

        let tokens_lt = tokenize_algol("<");
        let pairs_lt = token_pairs(&tokens_lt);
        assert_eq!(pairs_lt[0].0, "LT");

        let tokens_gt = tokenize_algol(">");
        let pairs_gt = token_pairs(&tokens_gt);
        assert_eq!(pairs_gt[0].0, "GT");
    }

    // -----------------------------------------------------------------------
    // Test 26: Delimiter tokens
    // -----------------------------------------------------------------------

    /// LPAREN, RPAREN, LBRACKET, RBRACKET map to built-in TokenType variants.
    /// SEMICOLON, COMMA, COLON also map to built-ins.
    #[test]
    fn test_tokenize_delimiters() {
        // Tokens that map to built-in TokenType variants
        let cases: &[(&str, TokenType)] = &[
            ("(", TokenType::LParen),
            (")", TokenType::RParen),
            ("[", TokenType::LBracket),
            ("]", TokenType::RBracket),
            (";", TokenType::Semicolon),
            (",", TokenType::Comma),
            (":", TokenType::Colon),
        ];

        for (src, expected_type) in cases {
            let tokens = tokenize_algol(src);
            let non_eof: Vec<&Token> = tokens
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .collect();

            assert_eq!(non_eof.len(), 1, "Expected 1 token for {:?}", src);
            assert_eq!(
                non_eof[0].type_, *expected_type,
                "Expected {:?} for {:?}, got {:?}",
                expected_type, src, non_eof[0].type_
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 27: Disambiguation — := is not : followed by =
    // -----------------------------------------------------------------------

    /// `x := 1` must produce IDENT, ASSIGN, INTEGER_LIT — not IDENT, COLON, EQ, INTEGER_LIT.
    /// This tests the critical first-match ordering: ASSIGN before COLON.
    #[test]
    fn test_disambiguate_assign_vs_colon() {
        let tokens = tokenize_algol("x := 1");
        let pairs = token_pairs(&tokens);

        // Expected: IDENT("x"), ASSIGN(":="), INTEGER_LIT("1")
        assert_eq!(
            pairs.len(),
            3,
            "Expected 3 tokens (IDENT, ASSIGN, INTEGER_LIT), got {:?}",
            pairs
        );
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[1].0, "ASSIGN");
        assert_eq!(pairs[1].1, ":=");
        assert_eq!(pairs[2].0, "INTEGER_LIT");
    }

    // -----------------------------------------------------------------------
    // Test 28: Disambiguation — ** is not * followed by *
    // -----------------------------------------------------------------------

    /// `x ** 2` must produce IDENT, POWER, INTEGER_LIT — not IDENT, STAR, STAR, INTEGER_LIT.
    /// This tests the critical first-match ordering: POWER before STAR.
    #[test]
    fn test_disambiguate_power_vs_star() {
        let tokens = tokenize_algol("x ** 2");
        let pairs = token_pairs(&tokens);

        // Expected: IDENT("x"), POWER("**"), INTEGER_LIT("2")
        assert_eq!(
            pairs.len(),
            3,
            "Expected 3 tokens (IDENT, POWER, INTEGER_LIT), got {:?}",
            pairs
        );
        assert_eq!(pairs[0].0, "NAME");
        assert_eq!(pairs[1].0, "POWER");
        assert_eq!(pairs[1].1, "**");
        assert_eq!(pairs[2].0, "INTEGER_LIT");
    }

    // -----------------------------------------------------------------------
    // Test 29: Comment skipping
    // -----------------------------------------------------------------------

    /// ALGOL 60 comments start with the keyword `comment` and end with `;`.
    /// Everything in between is consumed silently — no tokens are emitted
    /// for the comment itself.
    ///
    /// Example: `comment this is ignored; x := 1`
    /// should produce only: IDENT("x"), ASSIGN(":="), INTEGER_LIT("1")
    #[test]
    fn test_comment_skipping() {
        let tokens = tokenize_algol("comment this is ignored; x := 1");
        let pairs = token_pairs(&tokens);

        // The comment (including "comment" keyword and everything up to ";")
        // should be consumed. Only "x := 1" remains.
        // Note: behavior depends on whether the grammar treats COMMENT as skip
        // before or after keyword promotion. The algol.tokens grammar has
        // COMMENT in the skip: section as /comment[^;]*;/ which is tried
        // before regular token patterns.
        assert!(
            pairs
                .iter()
                .all(|(name, _)| *name != "comment" && *name != "COMMENT"),
            "Comment tokens should not appear in output: {:?}",
            pairs
        );
        // After the comment, we expect x := 1
        let has_ident_x = pairs
            .iter()
            .any(|(name, val)| *name == "NAME" && *val == "x");
        assert!(
            has_ident_x,
            "Expected IDENT('x') after comment, got {:?}",
            pairs
        );
    }

    // -----------------------------------------------------------------------
    // Test 30: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace (spaces, tabs, newlines, carriage returns) between tokens
    /// should be consumed without producing tokens. This is fundamental to
    /// ALGOL 60's "free format" design.
    #[test]
    fn test_whitespace_skipped() {
        let tokens_compact = tokenize_algol("x:=1");
        let tokens_spaced = tokenize_algol("  x  :=  1  ");
        let tokens_newlines = tokenize_algol("x\n:=\n1");

        let pairs_compact = token_pairs(&tokens_compact);
        let pairs_spaced = token_pairs(&tokens_spaced);
        let pairs_newlines = token_pairs(&tokens_newlines);

        // All three should produce the same number of tokens.
        assert_eq!(pairs_compact.len(), pairs_spaced.len());
        assert_eq!(pairs_compact.len(), pairs_newlines.len());

        // Values should be identical.
        for i in 0..pairs_compact.len() {
            assert_eq!(
                pairs_compact[i].1, pairs_spaced[i].1,
                "Token {i} value mismatch between compact and spaced"
            );
            assert_eq!(
                pairs_compact[i].1, pairs_newlines[i].1,
                "Token {i} value mismatch between compact and newlines"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 31: Multi-token expression
    // -----------------------------------------------------------------------

    /// A typical ALGOL 60 expression: x := 1 + 2 * 3
    /// Should produce: IDENT, ASSIGN, INTEGER_LIT, PLUS, INTEGER_LIT, STAR, INTEGER_LIT
    #[test]
    fn test_multi_token_expression() {
        let tokens = tokenize_algol("x := 1 + 2 * 3");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 7, "Expected 7 tokens, got {:?}", pairs);
        assert_eq!(pairs[0], ("NAME", "x"));
        assert_eq!(pairs[1], ("ASSIGN", ":="));
        assert_eq!(pairs[2], ("INTEGER_LIT", "1"));
        assert_eq!(pairs[3].0, "PLUS");
        assert_eq!(pairs[4], ("INTEGER_LIT", "2"));
        assert_eq!(pairs[5].0, "STAR");
        assert_eq!(pairs[6], ("INTEGER_LIT", "3"));
    }

    // -----------------------------------------------------------------------
    // Test 32: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_algol_lexer` factory function should return a `GrammarLexer`
    /// that can successfully tokenize source text.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_algol_lexer("x := 42");
        let tokens = lexer
            .tokenize()
            .expect("Lexer should tokenize successfully");

        // Should produce: IDENT("x"), ASSIGN(":="), INTEGER_LIT("42"), EOF
        assert!(tokens.len() >= 4);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 33: Full minimal program tokens
    // -----------------------------------------------------------------------

    /// Tokenize a minimal ALGOL 60 program: `begin integer x; x := 42 end`
    /// Verify that keywords, identifiers, types, operators, and delimiters
    /// all appear in the correct order.
    #[test]
    fn test_minimal_program_tokens() {
        let tokens = tokenize_algol("begin integer x; x := 42 end");
        let pairs = token_pairs(&tokens);

        // begin integer x ; x := 42 end
        assert!(
            pairs.len() >= 7,
            "Expected at least 7 tokens, got {:?}",
            pairs
        );

        // begin: KEYWORD
        assert_eq!(pairs[0].1, "begin");
        let begin_tok = tokens.iter().find(|t| t.value == "begin").unwrap();
        assert_eq!(begin_tok.type_, TokenType::Keyword);

        // integer: KEYWORD
        let integer_tok = tokens.iter().find(|t| t.value == "integer").unwrap();
        assert_eq!(integer_tok.type_, TokenType::Keyword);

        // end: KEYWORD
        let end_tok = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .last()
            .unwrap();
        assert_eq!(end_tok.value, "end");
        assert_eq!(end_tok.type_, TokenType::Keyword);
    }
}
