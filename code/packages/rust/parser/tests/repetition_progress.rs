//! Repetitions stop after an iteration that consumed nothing.
//!
//! A repetition body that *can* succeed without consuming a token —
//! `{ [ NAME ] }`, `{ @space_before | NAME }`, `{ p }` with `p = @space_before`
//! — used to make `GrammarParser` loop forever (and, through memo hits on a
//! rule body, push a node per turn until memory ran out). `GrammarParser::new`
//! does not validate its grammar, so the engine itself must terminate: after
//! an iteration that succeeded without moving the cursor (and without a `>>`
//! split), `{ }`, `{ }+` and `{ x // sep }` restore that iteration and stop.
//! These tests would hang without the guard.

// The helper returns the library's own `GrammarParseError` unboxed.
#![allow(clippy::result_large_err)]

use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{collect_tokens, find_nodes, GrammarASTNode, GrammarParseError, GrammarParser};

const TOKENS: &str = r#"
NAME  = /[a-z]+/
SEMI  = ";"
COMMA = ","
skip:
  WS = /[ \n]+/
"#;

fn parse(grammar: &str, source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let tg = parse_token_grammar(TOKENS).unwrap();
    let tokens = GrammarLexer::new(source, &tg).tokenize().unwrap();
    GrammarParser::new(tokens, parse_parser_grammar(grammar).unwrap()).parse()
}

fn values(ast: &GrammarASTNode) -> Vec<String> {
    collect_tokens(ast, None).iter().map(|t| t.value.clone()).collect()
}

/// The four grammars from the security review, plus the pre-F11 shape.
const PROBES: [&str; 5] = [
    "prog = NAME { @space_before | NAME } SEMI ;",
    "prog = NAME { @space_before [ NAME ] } SEMI ;",
    "prog = NAME { !@no_space_before | NAME } SEMI ;",
    "prog = NAME { p } SEMI ;\np = @space_before ;",
    "prog = NAME { [ NAME ] } SEMI ;",
];

#[test]
fn nullable_repetition_bodies_terminate() {
    for grammar in PROBES {
        let ast = parse(grammar, "a ;").unwrap_or_else(|e| panic!("{grammar}: {e}"));
        assert_eq!(values(&ast), vec!["a", ";"], "{grammar}");
        // No node per (empty) iteration: memory stays bounded.
        assert!(find_nodes(&ast, "p").is_empty(), "{grammar}");
        assert_eq!(ast.children.len(), 2, "{grammar}");
    }
}

#[test]
fn ordered_choice_still_decides_what_an_iteration_matches() {
    // `{ [ NAME ] }` consumes names until it can't.
    let ast = parse(PROBES[4], "a b c ;").unwrap();
    assert_eq!(values(&ast), vec!["a", "b", "c", ";"]);
    // `{ @space_before [ NAME ] }` consumes spaced names, then stops.
    let ast = parse(PROBES[1], "a b c ;").unwrap();
    assert_eq!(values(&ast), vec!["a", "b", "c", ";"]);
    // `{ @space_before | NAME }`: the empty first choice wins at `b`, so the
    // repetition stops there and `SEMI` is missing: a normal parse error,
    // reported at `b`, not a hang.
    let err = parse(PROBES[0], "a b ;").unwrap_err();
    assert_eq!(err.token.value, "b");
    assert!(err.message.contains("SEMI"), "{}", err.message);
}

#[test]
fn one_or_more_and_separated_repetition_terminate() {
    for grammar in [
        "prog = NAME { [ NAME ] }+ SEMI ;",
        "prog = NAME { @space_before [ NAME ] }+ SEMI ;",
        "prog = NAME { [ NAME ] // [ COMMA ] } SEMI ;",
        "prog = NAME { [ NAME ] // @space_before }+ SEMI ;",
    ] {
        let ast = parse(grammar, "a ;").unwrap_or_else(|e| panic!("{grammar}: {e}"));
        assert_eq!(values(&ast), vec!["a", ";"], "{grammar}");
        // Whatever the grammar makes of this, it must finish.
        let _ = parse(grammar, "a b , c ;");
    }
    let ast = parse("prog = { [ NAME ] // [ COMMA ] } SEMI ;", "a, b c ;").unwrap();
    assert_eq!(values(&ast), vec!["a", ",", "b", "c", ";"]);
}

#[test]
fn a_split_counts_as_progress() {
    // `>>` split into `>` `>` matches the first `>` without moving the
    // cursor; the guard must not stop the repetition there.
    let tok = |type_name: &str, value: &str, column: usize| Token {
        cv: None,
        type_: TokenType::Name,
        value: value.to_string(),
        line: 1,
        column,
        type_name: Some(type_name.to_string()),
        flags: None,
    };
    let tokens = vec![
        tok("RIGHT_SHIFT", ">>", 1),
        Token { type_: TokenType::Eof, type_name: None, value: String::new(), ..tok("X", "", 3) },
    ];
    let grammar = parse_parser_grammar("closers = { GREATER_THAN } ;").unwrap();
    let ast = GrammarParser::new(tokens, grammar).parse().unwrap();
    assert_eq!(values(&ast), vec![">", ">"]);
}
