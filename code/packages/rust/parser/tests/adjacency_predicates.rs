//! F11 / LANG84 E6: token-adjacency predicates in the grammar-driven parser.
//!
//! A predicate such as `@space_before` or `@no_newline_before` is a
//! zero-width element: it looks at the gap in front of the next token (the
//! flags `GrammarLexer` records) and succeeds or fails without consuming
//! anything. These tests drive real `.tokens` / `.grammar` text through the
//! real lexer and parser:
//!
//! - Ruby-style `f -1` (command call with a negative argument) versus
//!   `f - 1` and `f-1` (subtraction), in all three spellings F11 allows.
//! - ECMAScript-style `return [no LineTerminator here] expression` and
//!   postfix `++`, with newlines skipped as whitespace and with newlines
//!   lexed as `NEWLINE` tokens the parser ignores.
//! - Predicates under `&`, `!`, repetition and alternation, packrat reuse,
//!   unchanged error reporting, hand-built token streams, and a compiled
//!   (`_grammar.rs`-style) grammar round trip.

// The helpers return the library's own `GrammarParseError` unboxed, exactly as
// `GrammarParser::parse` does; boxing it here would only obscure the tests.
#![allow(clippy::result_large_err)]

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::{parse_parser_grammar, ParserGrammar};
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType, TOKEN_PRECEDED_BY_SPACE};
use parser::grammar_parser::{
    collect_tokens, find_nodes, ASTNodeOrToken, GrammarASTNode, GrammarParseError, GrammarParser,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_with(tokens_src: &str, grammar: ParserGrammar, source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let tg = parse_token_grammar(tokens_src).unwrap();
    let tokens = GrammarLexer::new(source, &tg).tokenize().unwrap();
    GrammarParser::new(tokens, grammar).parse()
}

fn parse(tokens_src: &str, grammar_src: &str, source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    parse_with(tokens_src, parse_parser_grammar(grammar_src).unwrap(), source)
}

/// The rule names of the root's direct child nodes, in order.
fn child_rules(node: &GrammarASTNode) -> Vec<String> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n.rule_name.clone()),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Token values under every node named `rule`, one string per node.
fn texts(ast: &GrammarASTNode, rule: &str) -> Vec<String> {
    find_nodes(ast, rule)
        .iter()
        .map(|n| collect_tokens(n, None).iter().map(|t| t.value.clone()).collect::<Vec<_>>().join(" "))
        .collect()
}

// ---------------------------------------------------------------------------
// Ruby: `f -1` versus `f - 1` versus `f-1`
// ---------------------------------------------------------------------------

const RUBY_TOKENS: &str = r#"
NAME   = /[a-z]+/
NUMBER = /[0-9]+/
MINUS  = "-"
skip:
  WS = /[ \t]+/
"#;

/// `command_call` wants a space before the minus and none after it. The
/// three F11 spellings of that condition must agree.
fn ruby_grammar(command_call_body: &str) -> String {
    format!(
        r#"
statement    = command_call | expression ;
command_call = term {command_call_body} term ;
expression   = term {{ MINUS term }} ;
term         = NAME | NUMBER ;
"#
    )
}

const RUBY_SPELLINGS: [&str; 3] = [
    // LANG84 §E6, verbatim.
    "@space_before !@space_after_next MINUS",
    // The `no_` form of the same thing.
    "@space_before @no_space_after_next MINUS",
    // Test the gap *after* the minus directly.
    "@space_before MINUS @no_space_before",
];

#[test]
fn ruby_command_argument_versus_subtraction() {
    for spelling in RUBY_SPELLINGS {
        let grammar = ruby_grammar(spelling);
        let cases = [
            ("f -1", "command_call"),
            ("f - 1", "expression"),
            ("f-1", "expression"),
            ("f- 1", "expression"),
            ("f  -1", "command_call"),
        ];
        for (source, expected) in cases {
            let ast = parse(RUBY_TOKENS, &grammar, source)
                .unwrap_or_else(|e| panic!("{spelling:?} on {source:?}: {e}"));
            assert_eq!(child_rules(&ast), vec![expected.to_string()], "{spelling:?} on {source:?}");
            // Whatever the reading, every token is still in the tree, once.
            let values: Vec<String> = collect_tokens(&ast, None).iter().map(|t| t.value.clone()).collect();
            assert_eq!(values.join(""), source.replace(' ', ""), "{spelling:?} on {source:?}");
        }
    }
}

#[test]
fn packrat_reuse_across_a_failed_predicate() {
    // `command_call` parses `term` at position 0 and then fails at its
    // predicate on `f - 1`; `expression` then gets `term` at position 0 from
    // the memo. The predicate's answer is a function of the position, so the
    // reuse is sound and the result is the same as without memoization.
    let grammar = ruby_grammar("@space_before MINUS @no_space_before");
    let ast = parse(RUBY_TOKENS, &grammar, "a - b - 3").unwrap();
    assert_eq!(child_rules(&ast), vec!["expression".to_string()]);
    assert_eq!(texts(&ast, "term"), vec!["a", "b", "3"]);
}

// ---------------------------------------------------------------------------
// ECMAScript: restricted productions
// ---------------------------------------------------------------------------

/// Newlines are skipped as whitespace: the parser never sees them, only the
/// flag on the following token.
const JS_TOKENS_SKIP_NEWLINES: &str = r#"
NAME          = /[a-z]+/
PLUS_PLUS     = "++"
PLUS          = "+"
SEMICOLON     = ";"
skip:
  WS      = /[ \t\r\n]+/
  COMMENT = /\/\*[\s\S]*?\*\//
"#;

/// Newlines are *not* skipped, so the lexer emits `NEWLINE` tokens; the
/// grammar never mentions `NEWLINE`, so the parser steps over them.
const JS_TOKENS_NEWLINE_TOKENS: &str = r#"
NAME          = /[a-z]+/
PLUS_PLUS     = "++"
PLUS          = "+"
SEMICOLON     = ";"
skip:
  WS      = /[ \t\r]+/
  COMMENT = /\/\*[\s\S]*?\*\//
"#;

const JS_GRAMMAR: &str = r#"
program              = { statement } ;
statement            = return_statement | expression_statement ;
return_statement     = "return" [ @no_newline_before expression ] [ SEMICOLON ] ;
expression_statement = expression [ SEMICOLON ] ;
expression           = unary { PLUS unary } ;
unary                = "++" postfix | postfix ;
postfix              = NAME [ @no_newline_before "++" ] ;
"#;

fn js_both_ways(source: &str) -> Vec<GrammarASTNode> {
    [JS_TOKENS_SKIP_NEWLINES, JS_TOKENS_NEWLINE_TOKENS]
        .iter()
        .map(|t| parse(t, JS_GRAMMAR, source).unwrap_or_else(|e| panic!("{source:?}: {e}")))
        .collect()
}

#[test]
fn js_return_no_line_terminator_here() {
    for ast in js_both_ways("return x") {
        assert_eq!(texts(&ast, "return_statement"), vec!["return x"]);
    }
    // `return ⏎ x` is `return; x;`.
    for ast in js_both_ways("return\nx") {
        assert_eq!(texts(&ast, "return_statement"), vec!["return"]);
        assert_eq!(texts(&ast, "expression_statement"), vec!["x"]);
    }
    // A line break hidden in a block comment counts too.
    for ast in js_both_ways("return /* a\n b */ x") {
        assert_eq!(texts(&ast, "return_statement"), vec!["return"]);
    }
    // A one-line comment does not.
    for ast in js_both_ways("return /* a */ x + y") {
        assert_eq!(texts(&ast, "return_statement"), vec!["return x + y"]);
    }
}

#[test]
fn js_postfix_increment_must_stay_on_the_line() {
    // `a ++` is postfix; `a ⏎ ++b` is `a; ++b;`.
    for ast in js_both_ways("a ++") {
        assert_eq!(texts(&ast, "postfix"), vec!["a ++"]);
    }
    for ast in js_both_ways("a\n++b") {
        assert_eq!(texts(&ast, "expression_statement"), vec!["a", "++ b"]);
        assert_eq!(texts(&ast, "postfix"), vec!["a", "b"]);
    }
}

#[test]
fn predicates_do_not_make_newlines_significant() {
    let g = parse_parser_grammar(JS_GRAMMAR).unwrap();
    let parser = GrammarParser::new(Vec::new(), g);
    assert!(!parser.is_newlines_significant());
}

#[test]
fn predicates_in_newline_significant_grammars_see_the_newline_token() {
    // Here the grammar references NEWLINE, so NEWLINE tokens are not skipped
    // and *are* the next token. `@no_newline_before` then tests the NEWLINE
    // token's own gap (none), while the token after the NEWLINE is flagged.
    let grammar = r#"
program   = { line } ;
line      = NAME [ @no_newline_before NAME ] NEWLINE ;
"#;
    let ast = parse(JS_TOKENS_NEWLINE_TOKENS, grammar, "a b\nc\n").unwrap();
    assert_eq!(texts(&ast, "line"), vec!["a b \\n", "c \\n"]);
}

// ---------------------------------------------------------------------------
// Composition: lookahead, repetition, alternation
// ---------------------------------------------------------------------------

const WORDS_TOKENS: &str = r#"
NAME  = /[a-z]+/
COMMA = ","
skip:
  WS = /[ \n]+/
"#;

#[test]
fn predicates_inside_repetition_split_lines_into_commands() {
    // A shell-like command: a name and its arguments, up to the end of line.
    let grammar = r#"
program = { command } ;
command = NAME { @no_newline_before NAME } ;
"#;
    let ast = parse(WORDS_TOKENS, grammar, "puts a b\nls\necho x").unwrap();
    assert_eq!(texts(&ast, "command"), vec!["puts a b", "ls", "echo x"]);
}

#[test]
fn predicates_under_lookahead_and_alternation() {
    // `glued` needs two names with nothing between them... which the NAME
    // regex cannot produce, so use a comma: `a,b` vs `a, b` vs `a ,b`.
    let grammar = r#"
program = { item } ;
item    = tight | loose ;
tight   = NAME &@no_space_before COMMA !@space_before_never NAME ;
loose   = NAME [ COMMA ] ;
"#;
    // `@space_before_never` is not a predicate: unknown names are rejected.
    assert!(parse_parser_grammar(grammar).is_err());

    let grammar = r#"
program = { item } ;
item    = tight | loose ;
tight   = NAME &@no_space_before COMMA !@space_before NAME ;
loose   = NAME [ COMMA ] ;
"#;
    let ast = parse(WORDS_TOKENS, grammar, "a,b c, d e ,f").unwrap();
    assert_eq!(texts(&ast, "tight"), vec!["a , b"]);
    assert_eq!(texts(&ast, "loose"), vec!["c ,", "d", "e ,", "f"]);
}

#[test]
fn trailing_predicate_tests_the_gap_after_a_rule() {
    // A predicate at the end of a rule looks past the rule's last token.
    let grammar = r#"
program = { word } ;
word    = joined | NAME [ COMMA ] ;
joined  = NAME COMMA @no_space_before ;
"#;
    let ast = parse(WORDS_TOKENS, grammar, "a,b c, d").unwrap();
    // `a,` is followed directly by `b`; `c,` by a space.
    assert_eq!(texts(&ast, "joined"), vec!["a ,"]);
}

#[test]
fn after_next_at_end_of_input_sees_no_gap() {
    // From `b`, the token after the next one is past the end: EOF stands in,
    // and EOF carries no flags.
    let ast = parse(WORDS_TOKENS, "program = NAME [ @no_space_after_next NAME ] ;", "a b").unwrap();
    assert_eq!(collect_tokens(&ast, None).len(), 2);
    assert!(parse(WORDS_TOKENS, "program = NAME [ @space_after_next NAME ] ;", "a b").is_err());
}

// ---------------------------------------------------------------------------
// Error reporting is unchanged
// ---------------------------------------------------------------------------

#[test]
fn a_failing_predicate_leaves_error_messages_and_positions_alone() {
    let with_predicate = r#"program = "return" [ @no_newline_before NAME ] SEMICOLON ;"#;
    let without = r#"program = "return" SEMICOLON ;"#;
    let source = "return\nx;";
    let e1 = parse(JS_TOKENS_SKIP_NEWLINES, with_predicate, source).unwrap_err();
    let e2 = parse(JS_TOKENS_SKIP_NEWLINES, without, source).unwrap_err();
    assert_eq!(e1.message, e2.message);
    assert_eq!((e1.token.line, e1.token.column), (e2.token.line, e2.token.column));
    assert_eq!((e1.token.line, e1.token.column), (2, 1));
    assert!(!e1.message.contains('@'), "{}", e1.message);
}

// ---------------------------------------------------------------------------
// Hand-built token streams
// ---------------------------------------------------------------------------

fn tok(type_name: &str, value: &str, column: usize, flags: Option<u32>) -> Token {
    Token {
        cv: None,
        type_: TokenType::Name,
        value: value.to_string(),
        line: 1,
        column,
        type_name: Some(type_name.to_string()),
        flags,
    }
}

#[test]
fn hand_built_tokens_without_flags_read_as_adjacent() {
    let grammar = ruby_grammar("@space_before MINUS @no_space_before");
    let eof = Token {
        cv: None,
        type_: TokenType::Eof,
        value: String::new(),
        line: 1,
        column: 5,
        type_name: None,
        flags: None,
    };
    let make = |minus_flags| {
        vec![
            tok("NAME", "f", 1, None),
            tok("MINUS", "-", 3, minus_flags),
            tok("NAME", "x", 4, None),
            eof.clone(),
        ]
    };
    let g = parse_parser_grammar(&grammar).unwrap();
    let ast = GrammarParser::new(make(None), g.clone()).parse().unwrap();
    assert_eq!(child_rules(&ast), vec!["expression".to_string()]);
    let ast = GrammarParser::new(make(Some(TOKEN_PRECEDED_BY_SPACE)), g).parse().unwrap();
    assert_eq!(child_rules(&ast), vec!["command_call".to_string()]);
}

// ---------------------------------------------------------------------------
// Compiled grammars: predicates survive `compile_parser_grammar`
// ---------------------------------------------------------------------------

/// The grammar compiled into `tests/fixtures/adjacency_compiled.rs`.
/// Regenerate the fixture with `F11_REGEN=1 cargo test --test
/// adjacency_predicates`.
const COMPILED_SOURCE: &str = r#"
statement    = command_call | expression ;
command_call = term @space_before !@space_after_next MINUS term [ @newline_before ] ;
expression   = term { @no_newline_before MINUS term } [ &@no_space_before @space_after_next ] ;
term         = NAME | NUMBER ;
"#;

mod compiled {
    include!("fixtures/adjacency_compiled.rs");
}

#[test]
fn compiled_grammar_round_trips() {
    let runtime = parse_parser_grammar(COMPILED_SOURCE).unwrap();
    let emitted = compile_parser_grammar(&runtime, "adjacency.grammar");
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/adjacency_compiled.rs");
    if std::env::var_os("F11_REGEN").is_some() {
        std::fs::write(fixture_path, &emitted).unwrap();
    }
    // 1. The checked-in compiled grammar is exactly what the compiler emits
    //    today (byte-stable emission).
    assert_eq!(std::fs::read_to_string(fixture_path).unwrap(), emitted);
    // 2. Compiling it as Rust and calling it gives back the same grammar.
    assert_eq!(compiled::parser_grammar(), runtime);
    // 3. And it parses like the runtime grammar does.
    for (source, expected) in [("f -1", "command_call"), ("f - 1", "expression"), ("f-1", "expression")] {
        let ast = parse_with(RUBY_TOKENS, compiled::parser_grammar(), source).unwrap();
        assert_eq!(child_rules(&ast), vec![expected.to_string()], "{source:?}");
    }
}
