use coding_adventures_j_lexer::{create_j_lexer, tokenize_j};
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_j(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_j(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

#[test]
fn slash_is_reduce_not_divide_and_percent_is_divide() {
    // MA06 §1 bullet 1: the single most common APL-to-J transliteration
    // mistake is assuming `/` carries over as division the way APL's `÷`
    // would suggest. It doesn't — `/` is the reduce adverb (shared with
    // APL), and division is `%` instead. This is the one test in this
    // suite that exists purely to guard against getting that backwards.
    assert_eq!(token_types("+/"), vec!["PLUS", "REDUCE"]);
    assert_eq!(token_types("A%B"), vec!["NAME", "PERCENT", "NAME"]);
}

#[test]
fn tokenizes_every_primitive_verb_glyph() {
    // One glyph per row of MA06 §4's listed order: conjugate/add,
    // negate/subtract, sign/multiply, reciprocal/divide, exponential/power,
    // floor·ceiling / min·max (digraphs), shape/reshape, index-generator/
    // index-of (digraph), ravel/catenate, tally/copy-replicate.
    assert_eq!(
        token_types("+ - * % ^ <. >. $ i. , #"),
        vec![
            "PLUS", "MINUS", "STAR", "PERCENT", "CARET", "FLOOR", "CEILING", "DOLLAR", "IDOT",
            "RAVEL", "HASH",
        ]
    );
}

#[test]
fn tokenizes_all_six_comparison_glyphs() {
    assert_eq!(
        token_types("= ~: < > <: >:"),
        vec!["EQ", "NE", "LT", "GT", "LE", "GE"]
    );
}

#[test]
fn digraphs_are_not_swallowed_by_their_single_character_prefix() {
    // The core longest-match-first hazard this grammar has to get right:
    // every `.`/`:`-suffixed digraph must win over the bare prefix
    // character it starts with, at every position it appears.
    assert_eq!(token_types("<."), vec!["FLOOR"]);
    assert_eq!(token_types("<:"), vec!["LE"]);
    assert_eq!(token_types("<"), vec!["LT"]);
    assert_eq!(token_types(">."), vec!["CEILING"]);
    assert_eq!(token_types(">:"), vec!["GE"]);
    assert_eq!(token_types(">"), vec!["GT"]);
    assert_eq!(token_types("~:"), vec!["NE"]);
    assert_eq!(token_types("=."), vec!["ASSIGN_LOCAL"]);
    assert_eq!(token_types("=:"), vec!["ASSIGN_GLOBAL"]);
    assert_eq!(token_types("="), vec!["EQ"]);
    assert_eq!(token_types("i."), vec!["IDOT"]);
}

#[test]
fn idot_does_not_swallow_an_ordinary_name_starting_with_i() {
    // `i.` is only the index-generator/index-of digraph when the very next
    // character is a literal dot; a name like `iota`, `if`, or `i` alone
    // must still tokenize as a plain NAME.
    assert_eq!(token_types("iota"), vec!["NAME"]);
    assert_eq!(token_types("i"), vec!["NAME"]);
    assert_eq!(token_types("i.5"), vec!["IDOT", "NUMBER"]);
}

#[test]
fn tokenizes_adverbs_conjunction_grouping_and_assignment() {
    assert_eq!(
        token_types("A=.(+/ B)"),
        vec![
            "NAME", "ASSIGN_LOCAL", "LPAREN", "PLUS", "REDUCE", "NAME", "RPAREN",
        ]
    );
    assert_eq!(token_types("A=:B"), vec!["NAME", "ASSIGN_GLOBAL", "NAME"]);
    assert_eq!(token_types("+\\ B"), vec!["PLUS", "SCAN", "NAME"]);
    assert_eq!(token_types("f@g"), vec!["NAME", "AT", "NAME"]);
}

#[test]
fn tokenizes_dense_numeric_arrays_via_vector_stranding() {
    // Mirrors apl-lexer: stranding (juxtaposed literals separated only by
    // whitespace) is a NUMBER NUMBER NUMBER token sequence at the lexer
    // level — grouping into one array literal is the parser's job.
    assert_eq!(token_types("1 2 3"), vec!["NUMBER", "NUMBER", "NUMBER"]);
    assert_eq!(token_values("1 2 3"), vec!["1", "2", "3"]);
}

#[test]
fn tokenizes_underscore_negative_numbers_distinctly_from_the_minus_verb() {
    // A leading underscore marks a negative literal (this file's own
    // design choice, MA06 §4 leaves the literal syntax unstated) — distinct
    // from the MINUS verb glyph `-`, so `_3` is one NUMBER token but `A-B`
    // keeps `-` as its own MINUS token.
    assert_eq!(token_types("_3"), vec!["NUMBER"]);
    assert_eq!(token_values("_3"), vec!["_3"]);
    assert_eq!(token_types("A-B"), vec!["NAME", "MINUS", "NAME"]);
    assert_eq!(token_types("A -_3"), vec!["NAME", "MINUS", "NUMBER"]);
}

#[test]
fn a_bare_underscore_is_not_a_number() {
    // Infinity (a bare `_`/`__`) is out of scope for this cut (MA06 §4
    // never lists it as a primitive) — NUMBER's regex requires at least
    // one digit after the underscore, so a lone `_` matches no token in
    // this grammar at all (it is structurally excluded, not merely
    // documented as deferred) and the lexer reports it as an error rather
    // than silently inventing a meaning for it.
    assert!(create_j_lexer("_").tokenize().is_err());
}

#[test]
fn tokenizes_numbers_with_decimal_and_underscore_exponent() {
    assert_eq!(token_types("1.5E_3"), vec!["NUMBER"]);
    assert_eq!(token_values("1.5E_3"), vec!["1.5E_3"]);
    assert_eq!(token_values("3.14"), vec!["3.14"]);
}

#[test]
fn tokenizes_plain_ascii_names() {
    assert_eq!(token_types("VAR1 x Result"), vec!["NAME", "NAME", "NAME"]);
}

#[test]
fn skips_whitespace_and_nb_line_comments() {
    assert_eq!(
        token_types("NB. a whole comment line\nA=.1"),
        vec!["NEWLINE", "NAME", "ASSIGN_LOCAL", "NUMBER"]
    );
    assert_eq!(
        token_types("A=.1 NB. trailing comment\nB=.2"),
        vec![
            "NAME", "ASSIGN_LOCAL", "NUMBER", "NEWLINE", "NAME", "ASSIGN_LOCAL", "NUMBER",
        ]
    );
}

#[test]
fn nb_comment_does_not_swallow_an_ordinary_name_starting_with_nb() {
    // "NB." is only a comment starter when followed by a literal dot; a
    // name that merely starts with those two letters (e.g. `NBA`) must
    // still tokenize as a plain NAME.
    assert_eq!(token_types("NBA=.1"), vec!["NAME", "ASSIGN_LOCAL", "NUMBER"]);
}

#[test]
fn newline_is_its_own_significant_token() {
    assert_eq!(
        token_types("A=.1\nB=.2"),
        vec!["NAME", "ASSIGN_LOCAL", "NUMBER", "NEWLINE", "NAME", "ASSIGN_LOCAL", "NUMBER"]
    );
}

#[test]
fn tokenizes_the_ma06_index_generator_and_sum_reduce_end_to_end() {
    // `A=.i.5` then `B=.+/A` — 0-based index-generator followed by a
    // sum-reduce, J's own ASCII-spelled analogue of MA05's `A←⍳5` / `B←+/A`
    // running example.
    assert_eq!(
        token_types("A=.i.5\nB=.+/A"),
        vec![
            "NAME", "ASSIGN_LOCAL", "IDOT", "NUMBER", "NEWLINE", "NAME", "ASSIGN_LOCAL", "PLUS",
            "REDUCE", "NAME",
        ]
    );
}
