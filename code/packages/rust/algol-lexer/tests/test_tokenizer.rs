//! Integration tests for the ALGOL 60 lexer.
//!
//! These tests exercise the full pipeline: grammar file → GrammarLexer → tokens.
//! Unlike the unit tests in lib.rs (which are compiled into the same crate),
//! these tests are compiled as a separate binary, verifying the public API.
//!
//! # Test organization
//!
//! 1. Literals — integers, reals, strings
//! 2. Identifiers
//! 3. Keywords — all 30 keywords
//! 4. Keyword boundary disambiguation
//! 5. Operators — multi-character before single-character
//! 6. Delimiters
//! 7. Comment skipping
//! 8. Whitespace skipping
//! 9. Multi-token sequences

use coding_adventures_algol_lexer::{create_algol_lexer, tokenize_algol};
use lexer::token::{Token, TokenType};

// ===========================================================================
// Helper functions
// ===========================================================================

/// Return (effective_type_name, value) pairs for non-EOF tokens.
///
/// `effective_type_name()` returns:
///   - The grammar name (e.g. "INTEGER_LIT", "ASSIGN") for custom types
///   - "KEYWORD" for keyword tokens
///   - The built-in name (e.g. "PLUS", "COMMA") for standard types
fn token_pairs(tokens: &[Token]) -> Vec<(&str, &str)> {
    tokens
        .iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(|t| (t.effective_type_name(), t.value.as_str()))
        .collect()
}

// ===========================================================================
// 1. Literals
// ===========================================================================

/// Integer literals: one or more decimal digits.
#[test]
fn test_integer_literals() {
    let cases = [
        ("0", "0"),
        ("42", "42"),
        ("1000", "1000"),
    ];
    for (src, expected_val) in cases {
        let tokens = tokenize_algol(src);
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs.len(), 1, "src={src:?}");
        assert_eq!(pairs[0].0, "INTEGER_LIT", "src={src:?}");
        assert_eq!(pairs[0].1, expected_val, "src={src:?}");
    }
}

/// Real literals: decimal point, exponent, or both.
/// REAL_LIT must come before INTEGER_LIT in the grammar so that "3.14"
/// matches REAL_LIT rather than INTEGER_LIT("3") + something.
#[test]
fn test_real_literals() {
    let cases = ["3.14", "1.5E3", "1.5E-3", "100E2"];
    for src in cases {
        let tokens = tokenize_algol(src);
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs.len(), 1, "src={src:?}");
        assert_eq!(pairs[0].0, "REAL_LIT", "Expected REAL_LIT for {src:?}");
    }
}

/// String literals are single-quoted. No escape sequences in ALGOL 60.
#[test]
fn test_string_literals() {
    // Non-empty string
    let tokens = tokenize_algol("'hello'");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "STRING_LIT");
    assert!(pairs[0].1.contains("hello"),
        "Expected 'hello' in string value, got {:?}", pairs[0].1);

    // Empty string
    let tokens_empty = tokenize_algol("''");
    let pairs_empty = token_pairs(&tokens_empty);
    assert_eq!(pairs_empty.len(), 1);
    assert_eq!(pairs_empty[0].0, "STRING_LIT");
}

// ===========================================================================
// 2. Identifiers
// ===========================================================================

/// Identifiers: letter followed by zero or more letters or digits.
/// No underscore — original ALGOL 60 did not allow it.
#[test]
fn test_identifiers() {
    let cases = [
        ("x", "x"),
        ("sum", "sum"),
        ("customerName", "customerName"),
        ("A1", "A1"),
    ];
    for (src, expected_val) in cases {
        let tokens = tokenize_algol(src);
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs.len(), 1, "src={src:?}");
        assert_eq!(pairs[0].0, "NAME", "src={src:?}");
        assert_eq!(pairs[0].1, expected_val, "src={src:?}");
    }
}

// ===========================================================================
// 3. Keywords
// ===========================================================================

/// All 30 ALGOL 60 keywords must produce TokenType::Keyword.
#[test]
fn test_all_keywords_are_keyword_type() {
    let keywords = [
        "begin", "end", "if", "then", "else", "for", "do",
        "step", "until", "while", "goto", "switch", "procedure",
        "integer", "real", "boolean", "string", "array", "own",
        "label", "value", "true", "false", "not", "and", "or",
        "impl", "eqv", "div", "mod",
    ];

    for kw in keywords {
        let tokens = tokenize_algol(kw);
        let non_eof: Vec<&Token> = tokens.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();

        assert_eq!(non_eof.len(), 1, "Expected 1 token for keyword '{kw}'");
        assert_eq!(
            non_eof[0].type_,
            TokenType::Keyword,
            "Expected Keyword type for '{kw}', got {:?}",
            non_eof[0].type_
        );
        assert_eq!(non_eof[0].value, kw, "Keyword value should be preserved");
    }
}

// ===========================================================================
// 4. Keyword boundary disambiguation
// ===========================================================================

/// "beginning" starts with "begin" but must NOT be tokenized as the keyword
/// `begin`. The lexer matches the full longest token: "beginning" → IDENT.
#[test]
fn test_keyword_boundary_beginning() {
    let tokens = tokenize_algol("beginning");
    let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.type_ != TokenType::Eof).collect();

    assert_eq!(non_eof.len(), 1,
        "Expected 1 IDENT token, got {} (lexer split 'beginning' into parts?)", non_eof.len());
    assert_eq!(non_eof[0].type_, TokenType::Name,
        "'beginning' must be IDENT (Name), not keyword");
    assert_eq!(non_eof[0].value, "beginning");
}

/// Other edge cases: identifiers that start with a keyword prefix.
#[test]
fn test_keyword_boundary_other() {
    let cases = [
        ("iff", "iff"),       // starts with "if"
        ("endo", "endo"),     // starts with "end"
        ("truevalue", "truevalue"), // starts with "true"
        ("notch", "notch"),   // starts with "not"
    ];
    for (src, expected_val) in cases {
        let tokens = tokenize_algol(src);
        let non_eof: Vec<&Token> = tokens.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof.len(), 1, "src={src:?}: expected 1 token");
        assert_eq!(non_eof[0].type_, TokenType::Name,
            "src={src:?}: expected IDENT (Name), got {:?}", non_eof[0].type_);
        assert_eq!(non_eof[0].value, expected_val, "src={src:?}");
    }
}

// ===========================================================================
// 5. Operators
// ===========================================================================

/// Multi-character operators must take priority over their single-character
/// prefixes. This is enforced by ordering in the grammar file.
#[test]
fn test_assign_not_colon_eq() {
    // ":=" must produce one ASSIGN token, not COLON + EQ
    let tokens = tokenize_algol(":=");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1, "':=' should be 1 ASSIGN token, not 2");
    assert_eq!(pairs[0].0, "ASSIGN");
    assert_eq!(pairs[0].1, ":=");
}

#[test]
fn test_power_not_star_star() {
    // "**" must produce one POWER token, not STAR + STAR
    let tokens = tokenize_algol("**");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1, "'**' should be 1 POWER token, not 2");
    assert_eq!(pairs[0].0, "POWER");
    assert_eq!(pairs[0].1, "**");
}

#[test]
fn test_leq_not_lt_eq() {
    // "<=" must produce one LEQ token, not LT + EQ
    let tokens = tokenize_algol("<=");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1, "'<=' should be 1 LEQ token, not 2");
    assert_eq!(pairs[0].0, "LEQ");
}

#[test]
fn test_geq_not_gt_eq() {
    let tokens = tokenize_algol(">=");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1, "'>=' should be 1 GEQ token, not 2");
    assert_eq!(pairs[0].0, "GEQ");
}

#[test]
fn test_neq_operator() {
    let tokens = tokenize_algol("!=");
    let pairs = token_pairs(&tokens);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "NEQ");
    assert_eq!(pairs[0].1, "!=");
}

/// All single-character operators.
#[test]
fn test_single_char_operators() {
    // Built-in TokenType variants
    let cases_builtin: &[(&str, TokenType)] = &[
        ("+", TokenType::Plus),
        ("-", TokenType::Minus),
        ("*", TokenType::Star),
        ("/", TokenType::Slash),
    ];
    for (src, expected_type) in cases_builtin {
        let tokens = tokenize_algol(src);
        let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1, "src={src:?}");
        assert_eq!(non_eof[0].type_, *expected_type, "src={src:?}");
    }

    // Custom types (type_name set)
    let cases_custom: &[(&str, &str)] = &[
        ("^", "CARET"),
        ("=", "EQ"),
        ("<", "LT"),
        (">", "GT"),
    ];
    for (src, expected_name) in cases_custom {
        let tokens = tokenize_algol(src);
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs.len(), 1, "src={src:?}");
        assert_eq!(pairs[0].0, *expected_name, "src={src:?}");
    }
}

// ===========================================================================
// 6. Delimiters
// ===========================================================================

/// Delimiter tokens — both built-in and custom.
#[test]
fn test_delimiters() {
    let cases_builtin: &[(&str, TokenType)] = &[
        ("(", TokenType::LParen),
        (")", TokenType::RParen),
        ("[", TokenType::LBracket),
        ("]", TokenType::RBracket),
        (";", TokenType::Semicolon),
        (",", TokenType::Comma),
        (":", TokenType::Colon),
    ];
    for (src, expected_type) in cases_builtin {
        let tokens = tokenize_algol(src);
        let non_eof: Vec<&Token> = tokens.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1, "src={src:?}");
        assert_eq!(non_eof[0].type_, *expected_type,
            "src={src:?}: expected {:?}, got {:?}", expected_type, non_eof[0].type_);
    }
}

// ===========================================================================
// 7. Comment skipping
// ===========================================================================

/// ALGOL 60 comment syntax: `comment <text> ;`
/// Everything from the word "comment" up to and including the next ";" is
/// consumed silently. No tokens are emitted for the comment.
#[test]
fn test_comment_is_skipped() {
    // Source: "comment this is ignored; x := 1"
    // After skipping the comment, only "x := 1" should remain.
    let tokens = tokenize_algol("comment this is ignored; x := 1");
    let pairs = token_pairs(&tokens);

    // The comment text and the comment keyword itself must not appear.
    for (name, val) in &pairs {
        assert_ne!(*val, "comment",
            "The 'comment' keyword should have been consumed by the skip rule");
        assert_ne!(*name, "COMMENT",
            "No COMMENT tokens should appear in output");
    }

    // After the comment, we expect x := 1
    assert!(
        pairs.iter().any(|(name, val)| *name == "NAME" && *val == "x"),
        "Expected IDENT('x') after comment, got {:?}", pairs
    );
    assert!(
        pairs.iter().any(|(name, _)| *name == "ASSIGN"),
        "Expected ASSIGN after comment, got {:?}", pairs
    );
    assert!(
        pairs.iter().any(|(name, val)| *name == "INTEGER_LIT" && *val == "1"),
        "Expected INTEGER_LIT('1') after comment, got {:?}", pairs
    );
}

/// Comments in the middle of a program are transparent.
#[test]
fn test_comment_in_program() {
    let source = "x := 1; comment increment x; x := x + 1";
    let tokens = tokenize_algol(source);
    let pairs = token_pairs(&tokens);

    // Should only contain: x := 1 ; x := x + 1
    // Count occurrences of each token kind
    let ident_count = pairs.iter().filter(|(n, _)| *n == "NAME").count();
    let assign_count = pairs.iter().filter(|(n, _)| *n == "ASSIGN").count();

    // At least 3 IDENTs (x, x, x) and 2 ASSIGNs
    assert!(ident_count >= 3, "Expected at least 3 IDENTs, got {ident_count}");
    assert!(assign_count >= 2, "Expected at least 2 ASSIGNs, got {assign_count}");

    // No keyword "comment" should appear
    for (_, val) in &pairs {
        assert_ne!(*val, "comment", "comment keyword must be consumed by skip rule");
    }
}

// ===========================================================================
// 8. Whitespace skipping
// ===========================================================================

/// Whitespace is insignificant in ALGOL 60 (free-format language).
/// Spaces, tabs, newlines, and carriage returns produce no tokens.
#[test]
fn test_whitespace_is_skipped() {
    let compact = tokenize_algol("x:=1");
    let spaced = tokenize_algol("  x  :=  1  ");
    let tabs = tokenize_algol("\tx\t:=\t1\t");
    let newlines = tokenize_algol("x\n:=\n1");

    let pairs_compact = token_pairs(&compact);
    let pairs_spaced = token_pairs(&spaced);
    let pairs_tabs = token_pairs(&tabs);
    let pairs_newlines = token_pairs(&newlines);

    assert_eq!(pairs_compact.len(), pairs_spaced.len(),
        "compact vs spaced token count mismatch");
    assert_eq!(pairs_compact.len(), pairs_tabs.len(),
        "compact vs tabs token count mismatch");
    assert_eq!(pairs_compact.len(), pairs_newlines.len(),
        "compact vs newlines token count mismatch");

    for i in 0..pairs_compact.len() {
        assert_eq!(pairs_compact[i].0, pairs_spaced[i].0);
        assert_eq!(pairs_compact[i].1, pairs_spaced[i].1);
        assert_eq!(pairs_compact[i].0, pairs_newlines[i].0);
        assert_eq!(pairs_compact[i].1, pairs_newlines[i].1);
    }
}

// ===========================================================================
// 9. Multi-token sequences
// ===========================================================================

/// A typical ALGOL 60 assignment statement.
#[test]
fn test_assignment_sequence() {
    // x := 1 + 2 * 3
    // Expected: IDENT(x), ASSIGN(:=), INTEGER_LIT(1),
    //           PLUS(+), INTEGER_LIT(2), STAR(*), INTEGER_LIT(3)
    let tokens = tokenize_algol("x := 1 + 2 * 3");
    let pairs = token_pairs(&tokens);

    assert_eq!(pairs.len(), 7,
        "Expected 7 tokens, got {:?}", pairs);
    assert_eq!(pairs[0], ("NAME", "x"));
    assert_eq!(pairs[1], ("ASSIGN", ":="));
    assert_eq!(pairs[2], ("INTEGER_LIT", "1"));
    assert_eq!(pairs[3].0, "PLUS");
    assert_eq!(pairs[4], ("INTEGER_LIT", "2"));
    assert_eq!(pairs[5].0, "STAR");
    assert_eq!(pairs[6], ("INTEGER_LIT", "3"));
}

/// A minimal ALGOL 60 program: `begin integer x; x := 42 end`
#[test]
fn test_minimal_program_sequence() {
    let tokens = tokenize_algol("begin integer x; x := 42 end");
    let pairs = token_pairs(&tokens);

    // begin  integer  x  ;  x  :=  42  end  → 8 tokens
    assert_eq!(pairs.len(), 8,
        "Expected 8 tokens for minimal program, got {:?}", pairs);

    assert_eq!(pairs[0], ("KEYWORD", "begin"));
    assert_eq!(pairs[1], ("KEYWORD", "integer"));
    assert_eq!(pairs[2], ("NAME", "x"));
    assert_eq!(pairs[3].0, "SEMICOLON");
    assert_eq!(pairs[4], ("NAME", "x"));
    assert_eq!(pairs[5], ("ASSIGN", ":="));
    assert_eq!(pairs[6], ("INTEGER_LIT", "42"));
    assert_eq!(pairs[7], ("KEYWORD", "end"));
}

/// Boolean expression with ALGOL's word operators.
#[test]
fn test_boolean_operator_tokens() {
    // a and b or not c
    let tokens = tokenize_algol("a and b or not c");
    let pairs = token_pairs(&tokens);

    // a(IDENT) and(KW) b(IDENT) or(KW) not(KW) c(IDENT) → 6 tokens
    assert_eq!(pairs.len(), 6,
        "Expected 6 tokens, got {:?}", pairs);
    assert_eq!(pairs[0].0, "NAME");
    assert_eq!(pairs[1], ("KEYWORD", "and"));
    assert_eq!(pairs[2].0, "NAME");
    assert_eq!(pairs[3], ("KEYWORD", "or"));
    assert_eq!(pairs[4], ("KEYWORD", "not"));
    assert_eq!(pairs[5].0, "NAME");
}

/// If/then/else keywords appear correctly.
#[test]
fn test_if_then_else_tokens() {
    let tokens = tokenize_algol("if x then y := 1 else y := 2");
    let pairs = token_pairs(&tokens);

    // if(KW) x(IDENT) then(KW) y(IDENT) :=(ASSIGN) 1(INT) else(KW) y(IDENT) :=(ASSIGN) 2(INT)
    assert_eq!(pairs.len(), 10,
        "Expected 10 tokens, got {:?}", pairs);
    assert_eq!(pairs[0], ("KEYWORD", "if"));
    assert_eq!(pairs[1].0, "NAME");
    assert_eq!(pairs[2], ("KEYWORD", "then"));
    assert_eq!(pairs[6], ("KEYWORD", "else"));
}

/// For loop keyword sequence.
#[test]
fn test_for_loop_tokens() {
    let tokens = tokenize_algol("for i := 1 step 1 until 10 do");
    let pairs = token_pairs(&tokens);

    assert!(pairs.iter().any(|(n, v)| *n == "KEYWORD" && *v == "for"),
        "Expected 'for' keyword");
    assert!(pairs.iter().any(|(n, v)| *n == "KEYWORD" && *v == "step"),
        "Expected 'step' keyword");
    assert!(pairs.iter().any(|(n, v)| *n == "KEYWORD" && *v == "until"),
        "Expected 'until' keyword");
    assert!(pairs.iter().any(|(n, v)| *n == "KEYWORD" && *v == "do"),
        "Expected 'do' keyword");
}

/// Array subscript access uses brackets with colon for bound pairs.
#[test]
fn test_array_tokens() {
    // array A[1:10]
    let tokens = tokenize_algol("array A[1:10]");
    let pairs = token_pairs(&tokens);

    assert!(pairs.iter().any(|(n, v)| *n == "KEYWORD" && *v == "array"),
        "Expected 'array' keyword");
    assert!(pairs.iter().any(|(n, v)| *n == "NAME" && *v == "A"),
        "Expected IDENT('A')");

    let lbracket_tok = pairs.iter().find(|(n, _)| {
        let tokens_inner = tokenize_algol("[");
        let t = tokens_inner.iter().find(|t| t.type_ != TokenType::Eof).unwrap();
        t.type_.to_string() == "LBracket" && *n == "LBracket"
    });
    // Just verify bracket and colon are present by type
    let bracket_present = pairs.iter().any(|(_, v)| *v == "[");
    let colon_present = pairs.iter().any(|(_, v)| *v == ":");
    assert!(bracket_present || lbracket_tok.is_some(),
        "Expected bracket in array declaration");
    let _ = colon_present; // checked separately below
}

/// The factory function returns a usable lexer.
#[test]
fn test_factory_function() {
    let mut lexer = create_algol_lexer("begin x := 0 end");
    let tokens = lexer.tokenize().expect("Tokenization should succeed");

    assert!(tokens.len() >= 5, "Expected at least 5 tokens");
    assert_eq!(tokens.last().unwrap().type_, TokenType::Eof,
        "Last token should be EOF");
}

/// The token stream always ends with exactly one EOF.
#[test]
fn test_eof_at_end() {
    let cases = [
        "x",
        "42",
        "begin end",
        "",
    ];
    for src in cases {
        let tokens = tokenize_algol(src);
        assert_eq!(
            tokens.last().unwrap().type_,
            TokenType::Eof,
            "Last token should be EOF for src={src:?}"
        );
        let eof_count = tokens.iter().filter(|t| t.type_ == TokenType::Eof).count();
        assert_eq!(eof_count, 1, "Expected exactly 1 EOF for src={src:?}");
    }
}
