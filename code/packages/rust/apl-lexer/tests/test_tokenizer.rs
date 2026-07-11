use coding_adventures_apl_lexer::tokenize_apl;
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_apl(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_apl(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

#[test]
fn tokenizes_every_primitive_function_glyph() {
    // One glyph per row of MA05 §4's listed order: conjugate/add,
    // negate/subtract, sign/multiply, reciprocal/divide, ceiling·floor,
    // max·min, shape/reshape, index-generator/index-of, ravel/catenate,
    // plus the six comparison glyphs.
    assert_eq!(
        token_types("+ - × ÷ ⌈ ⌊ ⍴ ⍳ , = ≠ < ≤ ≥ >"),
        vec![
            "PLUS", "MINUS", "TIMES", "DIVIDE", "CEILING", "FLOOR", "RHO", "IOTA", "RAVEL", "EQ",
            "NE", "LT", "LE", "GE", "GT",
        ]
    );
}

#[test]
fn tokenizes_operators_grouping_and_assignment() {
    assert_eq!(
        token_types("A←(+/ B)"),
        vec![
            "NAME", "ARROW", "LPAREN", "PLUS", "REDUCE", "NAME", "RPAREN",
        ]
    );
    assert_eq!(token_types("+\\ B"), vec!["PLUS", "SCAN", "NAME"]);
    assert_eq!(token_types("∘.×"), vec!["OUTER", "TIMES"]);
}

#[test]
fn outer_is_not_confused_with_a_bare_jot_or_a_bare_dot() {
    // OUTER ("∘.") is declared as a single two-codepoint token ahead of any
    // rule that could confuse it — the grammar's longest-match-first
    // convention (SECTION 1 of apl.tokens). There is no standalone "jot"
    // token in this subset, so a jot immediately followed by a dot must
    // always lex as one OUTER token, never as two.
    assert_eq!(token_types("∘.="), vec!["OUTER", "EQ"]);
}

#[test]
fn tokenizes_dense_numeric_arrays_via_vector_stranding() {
    // MA05 §4 scopes this cut to dense numeric arrays; stranding (juxtaposed
    // literals separated only by whitespace, e.g. `1 2 3`) is a NUMBER NUMBER
    // NUMBER token sequence at the lexer level — grouping into one array
    // literal is the parser's job, not the lexer's.
    assert_eq!(
        token_types("1 2 3"),
        vec!["NUMBER", "NUMBER", "NUMBER"]
    );
    assert_eq!(token_values("1 2 3"), vec!["1", "2", "3"]);
}

#[test]
fn tokenizes_high_minus_negative_numbers_distinctly_from_the_minus_function() {
    // ¯ (U+00AF, macron) is APL's historical negative-literal sign, distinct
    // from the MINUS function glyph `-` (see apl.tokens SECTION 4) — so `¯3`
    // is one NUMBER token, but `A-B` keeps `-` as its own MINUS token.
    assert_eq!(token_types("¯3"), vec!["NUMBER"]);
    assert_eq!(token_values("¯3"), vec!["¯3"]);
    assert_eq!(token_types("A-B"), vec!["NAME", "MINUS", "NAME"]);
    assert_eq!(token_types("A -¯3"), vec!["NAME", "MINUS", "NUMBER"]);
}

#[test]
fn tokenizes_numbers_with_decimal_and_high_minus_exponent() {
    assert_eq!(token_types("1.5E¯3"), vec!["NUMBER"]);
    assert_eq!(token_values("1.5E¯3"), vec!["1.5E¯3"]);
    assert_eq!(token_values("3.14"), vec!["3.14"]);
}

#[test]
fn tokenizes_plain_ascii_names() {
    assert_eq!(token_types("VAR1 x Result"), vec!["NAME", "NAME", "NAME"]);
}

#[test]
fn skips_whitespace_and_line_comments() {
    assert_eq!(
        token_types("⍝ a whole comment line\nA←1"),
        vec!["NEWLINE", "NAME", "ARROW", "NUMBER"]
    );
    assert_eq!(
        token_types("A←1 ⍝ trailing comment\nB←2"),
        vec!["NAME", "ARROW", "NUMBER", "NEWLINE", "NAME", "ARROW", "NUMBER"]
    );
}

#[test]
fn newline_is_its_own_significant_token() {
    assert_eq!(
        token_types("A←1\nB←2"),
        vec!["NAME", "ARROW", "NUMBER", "NEWLINE", "NAME", "ARROW", "NUMBER"]
    );
}

#[test]
fn tokenizes_the_ma05_kickoff_example_end_to_end() {
    // `A←⍳5` then `B←+/A` — index-generator followed by a sum-reduce,
    // MA05's own running example for this historical-core subset.
    assert_eq!(
        token_types("A←⍳5\nB←+/A"),
        vec![
            "NAME", "ARROW", "IOTA", "NUMBER", "NEWLINE", "NAME", "ARROW", "PLUS", "REDUCE",
            "NAME",
        ]
    );
}
