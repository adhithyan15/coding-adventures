//! F11 / LANG84 E6: the gap flags `GrammarLexer` records on each token.
//!
//! Every token matched from a `.tokens` pattern says what was consumed
//! between it and the previous emitted token:
//!
//! ```text
//! TOKEN_PRECEDED_BY_SPACE    anything at all (whitespace, comment, newline,
//!                            a built-in NEWLINE token, a suppressed token)
//! TOKEN_PRECEDED_BY_NEWLINE  a line terminator among that trivia
//! ```
//!
//! These tests pin the definition down across every way the lexer consumes
//! trivia: skip patterns, the built-in whitespace rule, the built-in NEWLINE
//! token, F10 mode switches and skip toggles, indentation mode, and callback
//! suppression. Token types and values must never change.

use grammar_tools::token_grammar::{parse_token_grammar, TokenGrammar};
use lexer::grammar_lexer::{GrammarLexer, LexerContext};
use lexer::token::{
    Token, TokenType, TOKEN_CONTEXT_KEYWORD, TOKEN_PRECEDED_BY_NEWLINE, TOKEN_PRECEDED_BY_SPACE,
};

const S: u32 = TOKEN_PRECEDED_BY_SPACE;
const N: u32 = TOKEN_PRECEDED_BY_NEWLINE;

fn grammar(src: &str) -> TokenGrammar {
    parse_token_grammar(src).unwrap()
}

fn lex(g: &TokenGrammar, source: &str) -> Vec<Token> {
    GrammarLexer::new(source, g).tokenize().unwrap()
}

/// `(value, gap bits)` for every token, EOF included. Only the two F11 bits
/// are kept so unrelated flags cannot make a test pass by accident.
fn gaps(tokens: &[Token]) -> Vec<(String, u32)> {
    tokens
        .iter()
        .map(|t| {
            let value = if t.type_ == TokenType::Eof { "<EOF>".to_string() } else { t.value.clone() };
            (value, t.flags.unwrap_or(0) & (S | N))
        })
        .collect()
}

fn g(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
    pairs.iter().map(|(v, f)| (v.to_string(), *f)).collect()
}

/// A Ruby/JS-ish grammar whose skip patterns eat spaces, newlines and both
/// comment styles.
fn skip_grammar() -> TokenGrammar {
    grammar(
        r#"
NAME     = /[a-z]+/
NUMBER   = /[0-9]+/
TEMPLATE = /`[^`]*`/
MINUS    = "-"
LBRACKET = "["
RBRACKET = "]"
skip:
  WS            = /[ \t\r\n]+/
  LINE_COMMENT  = /#[^\n]*/
  BLOCK_COMMENT = /\/\*[\s\S]*?\*\//
"#,
    )
}

#[test]
fn ruby_minus_spacing_is_visible() {
    let gr = skip_grammar();
    // `f -1`: space before `-`, none after → command argument.
    assert_eq!(gaps(&lex(&gr, "f -1")), g(&[("f", 0), ("-", S), ("1", 0), ("<EOF>", 0)]));
    // `f - 1`: space on both sides → subtraction.
    assert_eq!(gaps(&lex(&gr, "f - 1")), g(&[("f", 0), ("-", S), ("1", S), ("<EOF>", 0)]));
    // `f-1`: no space at all → subtraction.
    assert_eq!(gaps(&lex(&gr, "f-1")), g(&[("f", 0), ("-", 0), ("1", 0), ("<EOF>", 0)]));
    // `foo [1]` vs `foo[1]`.
    assert_eq!(gaps(&lex(&gr, "foo [1]"))[1], ("[".to_string(), S));
    assert_eq!(gaps(&lex(&gr, "foo[1]"))[1], ("[".to_string(), 0));
}

#[test]
fn leading_trivia_flags_the_first_token() {
    let gr = skip_grammar();
    assert_eq!(gaps(&lex(&gr, "  a")), g(&[("a", S), ("<EOF>", 0)]));
    assert_eq!(gaps(&lex(&gr, "\na")), g(&[("a", S | N), ("<EOF>", 0)]));
}

#[test]
fn newline_skipped_as_whitespace_sets_both_bits() {
    let gr = skip_grammar();
    assert_eq!(gaps(&lex(&gr, "return\nx")), g(&[("return", 0), ("x", S | N), ("<EOF>", 0)]));
    assert_eq!(gaps(&lex(&gr, "a\r\nb"))[1], ("b".to_string(), S | N));
}

#[test]
fn comments_are_trivia_and_multiline_comments_carry_the_newline() {
    let gr = skip_grammar();
    // A one-line block comment is space, not a newline.
    assert_eq!(gaps(&lex(&gr, "a/* x */b"))[1], ("b".to_string(), S));
    // A line break *inside* a block comment is a line terminator before `b`
    // (ECMAScript: a MultiLineComment containing a LineTerminator).
    assert_eq!(gaps(&lex(&gr, "a/* x\n y */b"))[1], ("b".to_string(), S | N));
    // A line comment is followed by the newline that ends it.
    assert_eq!(gaps(&lex(&gr, "a # note\nb"))[1], ("b".to_string(), S | N));
}

#[test]
fn a_newline_inside_a_token_is_not_a_gap() {
    let gr = skip_grammar();
    // The template literal spans two lines, but the line break is part of
    // the token, so `c` (touching it) has no flags and `d` only a space.
    let toks = lex(&gr, "`a\nb`c `x\ny` d");
    assert_eq!(
        gaps(&toks),
        g(&[("`a\nb`", 0), ("c", 0), ("`x\ny`", S), ("d", S), ("<EOF>", 0)])
    );
}

#[test]
fn values_types_and_positions_are_unchanged() {
    let gr = skip_grammar();
    let toks = lex(&gr, "f -1\n  g");
    let shape: Vec<(&str, &str, usize, usize)> = toks
        .iter()
        .map(|t| (t.value.as_str(), t.effective_type_name(), t.line, t.column))
        .collect();
    assert_eq!(
        shape,
        vec![
            ("f", "NAME", 1, 1),
            ("-", "MINUS", 1, 3),
            ("1", "NUMBER", 1, 4),
            ("g", "NAME", 2, 3),
            ("", "EOF", 2, 4),
        ]
    );
}

#[test]
fn context_keyword_flag_coexists_with_gap_flags() {
    let gr = grammar(
        r#"
NAME = /[a-z]+/
context_keywords:
  async
skip:
  WS = /[ \n]+/
"#,
    );
    let toks = lex(&gr, "x\nasync");
    assert_eq!(toks[1].flags, Some(TOKEN_CONTEXT_KEYWORD | S | N));
}

// ---------------------------------------------------------------------------
// Grammars without skip patterns: built-in whitespace and NEWLINE tokens
// ---------------------------------------------------------------------------

fn builtin_grammar() -> TokenGrammar {
    grammar(
        r#"
NAME   = /[a-z]+/
NUMBER = /[0-9]+/
MINUS  = "-"
"#,
    )
}

#[test]
fn builtin_whitespace_sets_space() {
    let gr = builtin_grammar();
    assert_eq!(gaps(&lex(&gr, "f -1")), g(&[("f", 0), ("-", S), ("1", 0), ("<EOF>", 0)]));
    assert_eq!(gaps(&lex(&gr, "f\t-\t1"))[2], ("1".to_string(), S));
}

#[test]
fn builtin_newline_token_counts_as_trivia_for_the_next_token() {
    let gr = builtin_grammar();
    let toks = lex(&gr, "a \n-1\n\nb");
    // The NEWLINE tokens themselves keep `flags: None`; the token after each
    // is flagged as preceded by a line break.
    assert_eq!(
        gaps(&toks),
        g(&[
            ("a", 0),
            ("\\n", 0),
            ("-", S | N),
            ("1", 0),
            ("\\n", 0),
            ("\\n", 0),
            ("b", S | N),
            ("<EOF>", 0),
        ])
    );
    assert!(toks.iter().filter(|t| t.type_ == TokenType::Newline).all(|t| t.flags.is_none()));
}

#[test]
fn lone_carriage_return_is_a_line_terminator() {
    let gr = builtin_grammar();
    assert_eq!(gaps(&lex(&gr, "a\rb"))[1], ("b".to_string(), S | N));
}

// ---------------------------------------------------------------------------
// F10 modes and skip toggles
// ---------------------------------------------------------------------------

#[test]
fn f10_mode_switches_do_not_disturb_the_gap() {
    // JS-shaped regex/division table: NAME switches to `div` mode.
    let gr = grammar(
        r#"
NAME = /[a-z]+/
REGEX = /\/[a-z]+\//
SLASH = "/"
EQUALS = "="
group div:
  SLASH = "/"
skip:
  WS = /[ \n]+/
start_mode: default
transitions:
  on NAME -> set-mode div
  on (EQUALS | SLASH | REGEX) -> set-mode default
"#,
    );
    // Division (in `div` mode after a NAME) with and without spaces.
    assert_eq!(
        gaps(&lex(&gr, "a / b")),
        g(&[("a", 0), ("/", S), ("b", S), ("<EOF>", 0)])
    );
    assert_eq!(gaps(&lex(&gr, "a/b")), g(&[("a", 0), ("/", 0), ("b", 0), ("<EOF>", 0)]));
    // A regex (in `default` mode after `=`) across a newline.
    assert_eq!(
        gaps(&lex(&gr, "x =\n/ab/")),
        g(&[("x", 0), ("=", S), ("/ab/", S | N), ("<EOF>", 0)])
    );
}

#[test]
fn with_skipping_disabled_nothing_is_trivia() {
    // While skip is disabled, the space is a token, so nothing is flagged.
    let gr = grammar(
        r#"
NAME = /[a-z]+/
QUOTE = "'"
SPACE = / /
skip:
  WS = / +/
transitions:
  on QUOTE in default -> push str
  on QUOTE in str -> pop
group str:
  QUOTE = "'"
  NAME = /[a-z]+/
  SPACE = / /
"#,
    );
    let mut gr = gr;
    // Make entering `str` disable skipping and leaving it re-enable it.
    gr.transitions[0].actions.push(grammar_tools::token_grammar::TransitionAction::DisableSkip);
    gr.transitions[1].actions.push(grammar_tools::token_grammar::TransitionAction::EnableSkip);
    let toks = lex(&gr, "a 'b c' d");
    assert_eq!(
        gaps(&toks),
        g(&[
            ("a", 0),
            ("'", S),
            ("b", 0),
            (" ", 0),
            ("c", 0),
            ("'", 0),
            ("d", S),
            ("<EOF>", 0),
        ])
    );
}

// ---------------------------------------------------------------------------
// Indentation mode
// ---------------------------------------------------------------------------

#[test]
fn indentation_mode_records_the_same_gaps() {
    let gr = grammar(
        r#"
mode: indentation
NAME   = /[a-z]+/
NUMBER = /[0-9]+/
MINUS  = "-"
EQUALS = "="
COLON  = ":"
LPAREN = "("
RPAREN = ")"
COMMA  = ","
skip:
  COMMENT = /#[^\n]*/
"#,
    );
    let toks = lex(&gr, "if x:\n    y = -1 # c\n    f(a,\n      b)\n");
    let flagged: Vec<(String, u32)> = gaps(&toks)
        .into_iter()
        .zip(toks.iter())
        .filter(|(_, t)| !matches!(t.type_, TokenType::Indent | TokenType::Dedent))
        .map(|(p, _)| p)
        .collect();
    assert_eq!(
        flagged,
        g(&[
            ("if", 0),
            ("x", S),
            (":", 0),
            ("\\n", 0),
            ("y", S | N),
            ("=", S),
            ("-", S),
            ("1", 0),
            ("\\n", 0),
            ("f", S | N),
            ("(", 0),
            ("a", 0),
            (",", 0),
            // The newline inside the brackets is swallowed, but it is still
            // a line terminator between `,` and `b`.
            ("b", S | N),
            (")", 0),
            ("\\n", 0),
            ("<EOF>", 0),
        ])
    );
    // Synthetic layout tokens carry no flags.
    assert!(toks
        .iter()
        .filter(|t| matches!(t.type_, TokenType::Indent | TokenType::Dedent | TokenType::Eof))
        .all(|t| t.flags.is_none()));
}

// ---------------------------------------------------------------------------
// Callback suppression
// ---------------------------------------------------------------------------

#[test]
fn a_suppressed_token_becomes_trivia() {
    let gr = grammar(
        r#"
NAME = /[a-z]+/
BACKSLASH_NL = /\\\n/
skip:
  WS = /[ ]+/
"#,
    );
    // Suppress a `\`-newline continuation token, like a line-continuation
    // hook would. The next token is separated by it (space) and its text
    // contains a line terminator (newline).
    let mut lx = GrammarLexer::new("a\\\nb c", &gr);
    lx.set_on_token(Some(Box::new(|t: &Token, ctx: &mut LexerContext| {
        if t.type_name.as_deref() == Some("BACKSLASH_NL") {
            ctx.suppress();
        }
    })));
    let toks = lx.tokenize().unwrap();
    assert_eq!(gaps(&toks), g(&[("a", 0), ("b", S | N), ("c", S), ("<EOF>", 0)]));

    // A suppressed token with no line break in it is only space, and the gap
    // before it carries over.
    let gr2 = grammar(
        r#"
NAME = /[a-z]+/
TILDE = "~"
skip:
  WS = /[ \n]+/
"#,
    );
    for (src, expected) in [("a~b", S), ("a\n~b", S | N), ("a ~b", S)] {
        let mut lx = GrammarLexer::new(src, &gr2);
        lx.set_on_token(Some(Box::new(|t: &Token, ctx: &mut LexerContext| {
            if t.value == "~" {
                ctx.suppress();
            }
        })));
        let toks = lx.tokenize().unwrap();
        assert_eq!(gaps(&toks), g(&[("a", 0), ("b", expected), ("<EOF>", 0)]), "{:?}", src);
    }
}

#[test]
fn a_suppressed_token_with_a_replacement_hands_its_gap_to_the_replacement() {
    // excel-lexer style: a NAME directly followed by `(` is suppressed and
    // re-emitted as FUNCTION_NAME. The replacement stands where the original
    // stood, so it inherits the gap, and the `(` after it must not.
    let gr = grammar(
        r#"
NAME   = /[A-Za-z]+/
NUMBER = /[0-9]+/
PLUS   = "+"
LPAREN = "("
RPAREN = ")"
skip:
  WS = /[ \n]+/
"#,
    );
    let lex_fn = |src: &str| {
        let mut lx = GrammarLexer::new(src, &gr);
        lx.set_on_token(Some(Box::new(|t: &Token, ctx: &mut LexerContext| {
            if t.effective_type_name() == "NAME" && ctx.peek(1) == "(" {
                ctx.suppress();
                ctx.emit(Token {
                    type_name: Some("FUNCTION_NAME".to_string()),
                    flags: None,
                    ..t.clone()
                });
            }
        })));
        lx.tokenize().unwrap()
    };
    let toks = lex_fn("SUM(1)");
    assert_eq!(toks[0].type_name.as_deref(), Some("FUNCTION_NAME"));
    assert_eq!(gaps(&toks), g(&[("SUM", 0), ("(", 0), ("1", 0), (")", 0), ("<EOF>", 0)]));

    let toks = lex_fn("x +\nSUM(1)");
    assert_eq!(
        gaps(&toks),
        g(&[("x", 0), ("+", S), ("SUM", S | N), ("(", 0), ("1", 0), (")", 0), ("<EOF>", 0)])
    );
    // A callback-emitted token's own flags are kept and the gap is OR'd in.
    let mut lx = GrammarLexer::new("a b", &gr);
    lx.set_on_token(Some(Box::new(|t: &Token, ctx: &mut LexerContext| {
        if t.value == "b" {
            ctx.suppress();
            ctx.emit(Token { flags: Some(TOKEN_CONTEXT_KEYWORD), ..t.clone() });
            ctx.emit(Token { value: "c".to_string(), flags: None, ..t.clone() });
        }
    })));
    let toks = lx.tokenize().unwrap();
    assert_eq!(toks[1].flags, Some(TOKEN_CONTEXT_KEYWORD | S));
    // Only the first replacement inherits the gap.
    assert_eq!(toks[2].flags, None);
}
