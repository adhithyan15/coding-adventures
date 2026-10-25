//! F11 "Automatic insertion" / LANG84 E6: `@asi(T, C…)` in the grammar-driven
//! parser.
//!
//! `@asi(SEMICOLON, RBRACE)` is ECMAScript's Automatic Semicolon Insertion
//! written as a grammar element. Where it succeeds, a zero-width *synthetic*
//! `SEMICOLON` joins the tree, so the tree has the same shape whether or not
//! the `;` was written. It succeeds when one of three things is true of the
//! next token:
//!
//! ```text
//!   a line break precedes it     a ⏎ b      →  a ;̂ b ;̂
//!   it is a closer (RBRACE)      { a }      →  { a ;̂ }
//!   the input has ended          a          →  a ;̂
//!   none of these                a b        →  error: Expected SEMICOLON
//! ```
//!
//! (`;̂` is a synthetic semicolon.) These tests drive real `.tokens` and
//! `.grammar` text through the real lexer and parser. They cover that table,
//! the synthetic token's fields, the PEG ordering that supplies ECMAScript's
//! "offending token" rule, restricted productions, spans, lookahead,
//! repetition, the budget, hand-built streams and a compiled-grammar round
//! trip.

// The helpers return the library's own `GrammarParseError` unboxed, exactly as
// `GrammarParser::parse` does.
#![allow(clippy::result_large_err)]

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::{parse_parser_grammar, ParserGrammar};
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType, TOKEN_PRECEDED_BY_NEWLINE, TOKEN_SYNTHETIC};
use parser::grammar_parser::{
    collect_tokens, find_nodes, GrammarASTNode, GrammarParseError, GrammarParser,
};

// ---------------------------------------------------------------------------
// A small ECMAScript-shaped language
// ---------------------------------------------------------------------------

/// Newlines are skipped as whitespace; the lexer still records them as
/// `TOKEN_PRECEDED_BY_NEWLINE` on the next token, which is what `@asi` reads.
const JS_TOKENS: &str = r#"
NAME      = /[a-z]+/
PLUS_PLUS = "++"
PLUS      = "+"
SEMICOLON = ";"
LPAREN    = "("
RPAREN    = ")"
LBRACE    = "{"
RBRACE    = "}"
skip:
  WS      = /[ \t\r\n]+/
  COMMENT = /\/\*[\s\S]*?\*\//
"#;

/// Statement ends are `SEMICOLON | @asi(SEMICOLON, RBRACE)`: a written `;`
/// wins, otherwise one is inserted where ECMAScript would insert it. The
/// empty statement is a plain `SEMICOLON`, where ASI must never apply.
const JS_GRAMMAR: &str = r#"
program              = { statement } ;
statement            = block | return_statement | empty_statement | expression_statement ;
block                = LBRACE { statement } RBRACE ;
empty_statement      = SEMICOLON ;
return_statement     = "return" [ @no_newline_before expression ] statement_end ;
expression_statement = expression statement_end ;
statement_end        = SEMICOLON | @asi(SEMICOLON, RBRACE) ;
expression           = postfix { PLUS postfix } ;
postfix              = primary { call_args } [ @no_newline_before "++" ] ;
call_args            = LPAREN [ expression ] RPAREN ;
primary              = NAME | LPAREN expression RPAREN ;
"#;

fn parse_with(tokens_src: &str, grammar: ParserGrammar, source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let tg = parse_token_grammar(tokens_src).unwrap();
    let tokens = GrammarLexer::new(source, &tg).tokenize().unwrap();
    GrammarParser::new(tokens, grammar).parse()
}

fn parse(tokens_src: &str, grammar_src: &str, source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    parse_with(tokens_src, parse_parser_grammar(grammar_src).unwrap(), source)
}

fn js(source: &str) -> GrammarASTNode {
    parse(JS_TOKENS, JS_GRAMMAR, source).unwrap_or_else(|e| panic!("{source:?}: {e}"))
}

fn is_synthetic(token: &Token) -> bool {
    token.flags.unwrap_or(0) & TOKEN_SYNTHETIC != 0
}

/// Each node named `rule`, as its token values joined by spaces, with a
/// synthetic token shown as `;^`.
fn texts(ast: &GrammarASTNode, rule: &str) -> Vec<String> {
    find_nodes(ast, rule)
        .iter()
        .map(|n| {
            collect_tokens(n, None)
                .iter()
                .map(|t| if is_synthetic(t) { ";^".to_string() } else { t.value.clone() })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn synthetic_tokens(ast: &GrammarASTNode) -> Vec<Token> {
    collect_tokens(ast, None).into_iter().filter(is_synthetic).collect()
}

// ---------------------------------------------------------------------------
// The three conditions
// ---------------------------------------------------------------------------

#[test]
fn line_break_closer_and_end_of_input_each_insert() {
    // A line break before the next statement, then the end of the input.
    let ast = js("a\nb");
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ;^", "b ;^"]);
    // A closer: `}` ends the statement inside the block.
    let ast = js("{ a }");
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ;^"]);
    // A line break hidden in a block comment counts, as ECMAScript requires.
    let ast = js("a /* x\n y */ b");
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ;^", "b ;^"]);
}

#[test]
fn a_written_semicolon_wins_and_nothing_is_inserted() {
    let ast = js("a; b;");
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ;", "b ;"]);
    assert!(synthetic_tokens(&ast).is_empty());
    // The empty statement is a plain SEMICOLON: `;;` is two of them.
    let ast = js("a;;");
    assert_eq!(texts(&ast, "empty_statement"), vec![";"]);
    assert!(synthetic_tokens(&ast).is_empty());
}

#[test]
fn none_of_the_conditions_is_an_ordinary_syntax_error() {
    // Same line, not a closer, not the end: no insertion. The error is the
    // one a grammar without `@asi` would give, at the offending token; the
    // failed `@asi` adds no expectation of its own.
    let err = parse(JS_TOKENS, JS_GRAMMAR, "a b").unwrap_err();
    assert_eq!(err.token.value, "b");
    assert!(err.message.contains("SEMICOLON"), "{}", err.message);
    assert!(!err.message.contains("asi"), "{}", err.message);
}

// ---------------------------------------------------------------------------
// The synthetic token
// ---------------------------------------------------------------------------

#[test]
fn the_synthetic_token_is_empty_flagged_and_placed_at_the_next_token() {
    let ast = js("a\n  b");
    let inserted = synthetic_tokens(&ast);
    assert_eq!(inserted.len(), 2);
    // Before `b` (line 2, column 3) …
    let first = &inserted[0];
    assert_eq!(first.type_, TokenType::Semicolon);
    assert_eq!(first.type_name, None);
    assert_eq!(first.value, "");
    assert_eq!((first.line, first.column), (2, 3));
    assert_eq!(first.flags, Some(TOKEN_SYNTHETIC));
    assert_eq!(first.cv, None);
    // … and before EOF.
    let eof = collect_tokens(&ast, None);
    assert!(eof.iter().all(|t| t.type_ != TokenType::Eof));
    assert_eq!(inserted[1].line, 2);
    // A written semicolon never carries the bit.
    let ast = js("a;");
    let written = collect_tokens(&ast, None);
    assert!(written.iter().all(|t| !is_synthetic(t)));
}

#[test]
fn a_custom_token_name_gets_the_lexers_type_fields() {
    // `SEMI` has no built-in `TokenType`, so the lexer would give it `Name`
    // plus `type_name = "SEMI"`, and so does the insertion.
    let tokens = "NAME = /[a-z]+/\nSEMI = \";\"\nskip:\n  WS = /[ \\n]+/\n";
    let ast = parse(tokens, "program = { NAME ( SEMI | @asi(SEMI) ) } ;", "a\nb").unwrap();
    let inserted = synthetic_tokens(&ast);
    assert_eq!(inserted.len(), 2);
    for token in &inserted {
        assert_eq!(token.type_, TokenType::Name);
        assert_eq!(token.type_name.as_deref(), Some("SEMI"));
        assert_eq!(token.value, "");
    }
}

#[test]
fn spans_skip_synthetic_tokens() {
    // Without the skip, `a`'s statement would end where `b` starts.
    let ast = js("a\nb");
    let statements = find_nodes(&ast, "expression_statement");
    assert_eq!((statements[0].start_line, statements[0].end_line), (Some(1), Some(1)));
    assert_eq!((statements[0].start_column, statements[0].end_column), (Some(1), Some(1)));
    assert_eq!((statements[1].start_line, statements[1].end_line), (Some(2), Some(2)));
    // A node whose only token is synthetic has no span at all.
    let ends = find_nodes(&ast, "statement_end");
    assert!(ends.iter().all(|n| n.start_line.is_none() && n.end_line.is_none()));
    // A written `;` still counts.
    let ast = js("a ;");
    let end = &find_nodes(&ast, "statement_end")[0];
    assert_eq!((end.start_line, end.start_column), (Some(1), Some(3)));
}

// ---------------------------------------------------------------------------
// PEG ordering and restricted productions
// ---------------------------------------------------------------------------

#[test]
fn a_line_break_before_a_parenthesis_does_not_end_the_statement() {
    // ECMAScript inserts a `;` only before an *offending* token. `(` can
    // continue `a` as a call, and `@asi` is tried only after the expression
    // has consumed all it can, so `a ⏎ (b)` is one statement: `a(b)`.
    let ast = js("a\n(b)");
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ( b ) ;^"]);
    assert_eq!(texts(&ast, "call_args"), vec!["( b )"]);
}

#[test]
fn restricted_return_inserts_after_the_keyword() {
    // `return ⏎ x` is `return; x;`.
    let ast = js("return\nx");
    assert_eq!(texts(&ast, "return_statement"), vec!["return ;^"]);
    assert_eq!(texts(&ast, "expression_statement"), vec!["x ;^"]);
    // On one line the expression belongs to the `return`.
    let ast = js("return x");
    assert_eq!(texts(&ast, "return_statement"), vec!["return x ;^"]);
    // Before `}` too.
    let ast = js("{ return }");
    assert_eq!(texts(&ast, "return_statement"), vec!["return ;^"]);
}

#[test]
fn restricted_postfix_increment_starts_a_new_statement_after_a_break() {
    // `a ⏎ ++` cannot be postfix, so a `;` goes in before `++`; the grammar
    // has no prefix `++`, so the rest is a syntax error at `++`.
    let err = parse(JS_TOKENS, JS_GRAMMAR, "a\n++").unwrap_err();
    assert_eq!(err.token.value, "++");
    // On one line it is postfix.
    let ast = js("a ++");
    assert_eq!(texts(&ast, "postfix"), vec!["a ++"]);
}

// ---------------------------------------------------------------------------
// Newline-significant grammars
// ---------------------------------------------------------------------------

#[test]
fn a_newline_token_as_the_next_token_is_a_line_break() {
    // This grammar references NEWLINE, so NEWLINE tokens are significant and
    // are not stepped over: the NEWLINE itself is the next token.
    let tokens = "NAME = /[a-z]+/\nSEMI = \";\"\nskip:\n  WS = /[ ]+/\n";
    let grammar = "program = { line } ;\nline = NAME ( SEMI | @asi(SEMI) ) [ NEWLINE ] ;";
    let ast = parse(tokens, grammar, "a\nb;\nc").unwrap();
    assert_eq!(texts(&ast, "line"), vec!["a ;^ \\n", "b ; \\n", "c ;^"]);
}

#[test]
fn asi_does_not_make_newlines_significant() {
    let g = parse_parser_grammar(JS_GRAMMAR).unwrap();
    assert!(!GrammarParser::new(Vec::new(), g).is_newlines_significant());
}

// ---------------------------------------------------------------------------
// Composition: lookahead and repetition
// ---------------------------------------------------------------------------

#[test]
fn under_lookahead_the_insertion_leaves_no_child() {
    let tokens = "NAME = /[a-z]+/\nSEMI = \";\"\nskip:\n  WS = /[ \\n]+/\n";
    // `!@asi(SEMI)`: the two names must be on one line.
    let same_line = "pair = NAME !@asi(SEMI) NAME ;";
    assert!(parse(tokens, same_line, "a b").is_ok());
    assert!(parse(tokens, same_line, "a\nb").is_err());
    // `&@asi(SEMI)`: succeeds, but the synthetic token is discarded.
    let ast = parse(tokens, "one = NAME &@asi(SEMI) ;", "a").unwrap();
    assert!(synthetic_tokens(&ast).is_empty());
}

#[test]
fn a_repetition_of_insertions_stops_after_an_empty_iteration() {
    // Validation reports this body as an error; the engine terminates
    // anyway, undoing the iteration that inserted without consuming.
    let tokens = "NAME = /[a-z]+/\nSEMI = \";\"\n";
    let ast = parse(tokens, "r = NAME { @asi(SEMI) } ;", "a").unwrap();
    assert!(synthetic_tokens(&ast).is_empty());
    let ast = parse(tokens, "r = NAME { @asi(SEMI) }+ ;", "a").unwrap();
    assert_eq!(synthetic_tokens(&ast).len(), 1);
}

// ---------------------------------------------------------------------------
// Budget
// ---------------------------------------------------------------------------

#[test]
fn more_than_one_insertion_per_input_token_is_refused() {
    let tokens = "NAME = /[a-z]+/\nSEMI = \";\"\n";
    // The empty input is one token (EOF): one insertion fits …
    assert!(parse(tokens, "s = @asi(SEMI) ;", "").is_ok());
    // … two do not.
    let err = parse(tokens, "s = @asi(SEMI) @asi(SEMI) ;", "").unwrap_err();
    assert!(
        err.message.contains("automatic insertion budget exceeded: 2 synthetic tokens for 1 input tokens"),
        "{}",
        err.message
    );
    // A memoized zero-width rule reused many times is counted once per
    // appearance: 2 × 2 × 2 = 8 insertions for 1 token.
    let doubled = "s = a ;\na = b b ;\nb = c c ;\nc = @asi(SEMI) @asi(SEMI) ;";
    let err = parse(tokens, doubled, "").unwrap_err();
    assert!(err.message.contains("8 synthetic tokens for 1 input tokens"), "{}", err.message);
}

// ---------------------------------------------------------------------------
// Hand-built token streams
// ---------------------------------------------------------------------------

fn tok(type_: TokenType, type_name: Option<&str>, value: &str, column: usize, flags: Option<u32>) -> Token {
    Token {
        cv: None,
        type_,
        value: value.to_string(),
        line: 1,
        column,
        type_name: type_name.map(str::to_string),
        flags,
    }
}

#[test]
fn hand_built_streams_use_flags_closers_and_the_end_of_the_stream() {
    let g = parse_parser_grammar("program = { NAME ( SEMICOLON | @asi(SEMICOLON, RBRACE) ) } [ RBRACE ] ;").unwrap();
    let name = |v: &str, col, flags| tok(TokenType::Name, None, v, col, flags);
    // No flags: `a b` on one line is an error …
    let stream = vec![name("a", 1, None), name("b", 3, None), tok(TokenType::Eof, None, "", 4, None)];
    assert!(GrammarParser::new(stream, g.clone()).parse().is_err());
    // … the newline flag on `b` inserts …
    let stream = vec![
        name("a", 1, None),
        name("b", 3, Some(TOKEN_PRECEDED_BY_NEWLINE)),
        tok(TokenType::Eof, None, "", 4, None),
    ];
    let ast = GrammarParser::new(stream, g.clone()).parse().unwrap();
    assert_eq!(synthetic_tokens(&ast).len(), 2);
    // … so does a closer, and a stream with no EOF token at all.
    let stream = vec![name("a", 1, None), tok(TokenType::RBrace, None, "}", 2, None)];
    let ast = GrammarParser::new(stream, g).parse().unwrap();
    assert_eq!(synthetic_tokens(&ast).len(), 1);
}

// ---------------------------------------------------------------------------
// Compiled grammars
// ---------------------------------------------------------------------------

mod compiled {
    include!("fixtures/auto_insert_compiled.rs");
}

#[test]
fn compiled_grammar_round_trips() {
    let runtime = parse_parser_grammar(JS_GRAMMAR).unwrap();
    let emitted = compile_parser_grammar(&runtime, "auto_insert.grammar");
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/auto_insert_compiled.rs");
    // Regenerate with `F11_REGEN=1 cargo test --test auto_insert`.
    if std::env::var_os("F11_REGEN").is_some() {
        std::fs::write(fixture_path, &emitted).unwrap();
    }
    assert_eq!(std::fs::read_to_string(fixture_path).unwrap(), emitted);
    assert_eq!(compiled::parser_grammar(), runtime);
    let ast = parse_with(JS_TOKENS, compiled::parser_grammar(), "{ a }\nb").unwrap();
    assert_eq!(texts(&ast, "expression_statement"), vec!["a ;^", "b ;^"]);
}
