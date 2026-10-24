//! LANG84 E1 (line-start guards) and E8 (refusal transitions with whitespace
//! conditions), as the `GrammarLexer` interprets them.
//!
//! ```text
//! EMBDOC = /=begin[^\n]*/ @line_start     tried only at offset 0 or after "\n"
//! SHEBANG = /#![^\n]*/ @input_start       tried only at offset 0
//!
//! transitions:
//!   on SLASH if space_before && !space_after -> refuse
//! ```
//!
//! A guard decides whether a definition is *tried*; it never reorders
//! definitions. A refusal turns a token the grammar cannot handle yet into a
//! `LexerError` at that token, so a lexer ported in slices fails closed.

use grammar_tools::token_grammar::{parse_token_grammar, TokenGrammar};
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};

fn grammar(src: &str) -> TokenGrammar {
    parse_token_grammar(src).unwrap()
}

/// `TYPE:value` for every token, EOF dropped, so a mismatch reads clearly.
fn lex(g: &TokenGrammar, source: &str) -> Vec<String> {
    GrammarLexer::new(source, g)
        .tokenize()
        .unwrap()
        .iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(show)
        .collect()
}

fn show(t: &Token) -> String {
    let name = match (&t.type_name, t.type_) {
        (Some(n), _) => n.clone(),
        (None, TokenType::Newline) => "NEWLINE".to_string(),
        (None, other) => format!("{other:?}").to_uppercase(),
    };
    format!("{name}:{}", t.value)
}

fn lex_err(g: &TokenGrammar, source: &str) -> String {
    let error = GrammarLexer::new(source, g).tokenize().unwrap_err();
    format!("{}:{}: {}", error.line, error.column, error.message)
}

// ---------------------------------------------------------------------------
// E1: @line_start / @input_start on token definitions
// ---------------------------------------------------------------------------

const EMBDOC: &str = r#"
EMBDOC = /=begin[^\n]*/ @line_start
EQ = "="
NAME = /[a-z]+/
"#;

#[test]
fn line_start_guard_holds_at_offset_zero() {
    assert_eq!(lex(&grammar(EMBDOC), "=begin doc"), ["EMBDOC:=begin doc"]);
}

#[test]
fn line_start_guard_holds_after_lf_and_crlf() {
    let g = grammar(EMBDOC);
    assert_eq!(lex(&g, "x\n=begin"), ["NAME:x", "NEWLINE:\\n", "EMBDOC:=begin"]);
    // `\r` is built-in whitespace here; the line ends at the `\n` after it.
    assert_eq!(lex(&g, "x\r\n=begin"), ["NAME:x", "NEWLINE:\\n", "EMBDOC:=begin"]);
}

#[test]
fn line_start_guard_fails_mid_line_and_after_indentation() {
    let g = grammar(EMBDOC);
    assert_eq!(lex(&g, "x =begin"), ["NAME:x", "EQ:=", "NAME:begin"]);
    // Ruby requires `=begin` in column 1; leading spaces end the line start.
    assert_eq!(lex(&g, "  =begin"), ["EQ:=", "NAME:begin"]);
}

#[test]
fn line_start_guard_holds_after_a_skipped_comment_that_ends_the_line() {
    // The skip pattern consumes the comment *and* its newline, so no NEWLINE
    // token is emitted, but the next character is still at a line start.
    let g = grammar(&format!(
        "{EMBDOC}\nskip:\n  COMMENT = /#[^\\n]*\\n/\n  WS = /[ \\t]+/\n"
    ));
    assert_eq!(lex(&g, "x # note\n=begin"), ["NAME:x", "EMBDOC:=begin"]);
}

#[test]
fn input_start_guard_holds_only_at_offset_zero() {
    let g = grammar("SHEBANG = /#![^\\n]*/ @input_start\nHASH = \"#\"\nBANG = \"!\"\nNAME = /[a-z]+/\n");
    assert_eq!(lex(&g, "#!sh"), ["SHEBANG:#!sh"]);
    assert_eq!(lex(&g, "\n#!sh"), ["NEWLINE:\\n", "HASH:#", "BANG:!", "NAME:sh"]);
}

#[test]
fn a_guarded_skip_pattern_is_tried_only_where_its_guard_holds() {
    // `__END__` at a line start swallows the rest of the input (Ruby's data
    // section); anywhere else it is an ordinary name.
    let g = grammar(
        "NAME = /[A-Za-z_]+/\nskip:\n  DATA = /__END__[\\s\\S]*/ @line_start\n  WS = /[ \\t\\n]+/\n",
    );
    assert_eq!(lex(&g, "a\n__END__\nb c"), ["NAME:a"]);
    assert_eq!(lex(&g, "a __END__ b"), ["NAME:a", "NAME:__END__", "NAME:b"]);
}

#[test]
fn a_guard_never_reorders_definitions() {
    // An earlier unguarded definition that matches still wins at a line start.
    let g = grammar("EQ = \"=\"\nEMBDOC = /=begin/ @line_start\nNAME = /[a-z]+/\n");
    assert_eq!(lex(&g, "=begin"), ["EQ:=", "NAME:begin"]);
}

#[test]
fn guards_apply_inside_groups() {
    let g = grammar(
        "start_mode: doc\nNAME = /[a-z]+/\ngroup doc:\n  HEAD = /=[a-z]+/ @line_start\n  EQ = \"=\"\n",
    );
    assert_eq!(lex(&g, "=pod"), ["HEAD:=pod"]);
}

// ---------------------------------------------------------------------------
// E8: `-> refuse` and `if` conditions
// ---------------------------------------------------------------------------

const DIV: &str = "NAME = /[a-z]+/\nSLASH = \"/\"\nskip:\n  WS = /[ \\t]+/\n";

#[test]
fn an_unconditional_refusal_stops_at_the_token() {
    let g = grammar(&format!("{DIV}transitions:\n  on SLASH -> refuse\n"));
    assert_eq!(lex(&g, "a b"), ["NAME:a", "NAME:b"]);
    assert_eq!(
        lex_err(&g, "a / b"),
        "1:3: token SLASH (\"/\") is not supported in mode 'default' by this grammar"
    );
}

#[test]
fn whitespace_conditions_tell_a_command_argument_from_a_division() {
    // Ruby: `puts /re/` (space before, none after) starts a regexp, which a
    // slice without regexps must refuse; `a / b`, `a/b` and `a/ b` divide.
    let g = grammar(&format!(
        "{DIV}transitions:\n  on SLASH if space_before && !space_after -> refuse\n"
    ));
    assert_eq!(lex(&g, "a / b"), ["NAME:a", "SLASH:/", "NAME:b"]);
    assert_eq!(lex(&g, "a/b"), ["NAME:a", "SLASH:/", "NAME:b"]);
    assert_eq!(lex(&g, "a/ b"), ["NAME:a", "SLASH:/", "NAME:b"]);
    // End of input counts as a separator, so a trailing ` /` is not refused.
    assert_eq!(lex(&g, "a /"), ["NAME:a", "SLASH:/"]);
    assert!(lex_err(&g, "a /b").starts_with("1:3: token SLASH"));
}

#[test]
fn negated_and_positive_forms_are_exact_complements() {
    let refuse_if = |cond: &str| grammar(&format!("{DIV}transitions:\n  on SLASH if {cond} -> refuse\n"));
    for (source, before, after) in [("a/b", false, false), ("a /b", true, false), ("a/ b", false, true), ("a / b", true, true)] {
        let refused = |cond: &str| GrammarLexer::new(source, &refuse_if(cond)).tokenize().is_err();
        assert_eq!(refused("space_before"), before, "{source:?}");
        assert_eq!(refused("!space_before"), !before, "{source:?}");
        assert_eq!(refused("space_after"), after, "{source:?}");
        assert_eq!(refused("!space_after"), !after, "{source:?}");
    }
}

#[test]
fn a_line_start_condition_reads_where_the_token_began() {
    let g = grammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\ntransitions:\n  on NAME if line_start -> refuse\n");
    assert!(lex_err(&g, "x").starts_with("1:1:"));
    assert!(lex_err(&g, "a\nx").starts_with("1:1:"), "the first name is at offset 0");
    let g = grammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\ntransitions:\n  on NAME if !line_start -> refuse\n");
    assert_eq!(lex(&g, "a\nb"), ["NAME:a", "NEWLINE:\\n", "NAME:b"]);
    assert!(lex_err(&g, "a b").starts_with("1:3:"));
}

#[test]
fn a_failed_condition_falls_through_to_the_next_rule() {
    let g = grammar(&format!(
        "{DIV}group div:\n  SLASH = \"/\"\ntransitions:\n  on SLASH if space_before && !space_after -> refuse\n  on SLASH -> set-mode div\n"
    ));
    assert_eq!(lex(&g, "a / b"), ["NAME:a", "SLASH:/", "NAME:b"]);
    assert!(GrammarLexer::new("a /b", &g).tokenize().is_err());
}

#[test]
fn a_refusal_scoped_to_a_mode_fires_only_there() {
    // The Ruby R1a shape: after an operator (mode `beg`), `/` would open a
    // regexp, which the slice refuses; after an operand it divides.
    let g = grammar(
        "start_mode: beg\nNAME = /[a-z]+/\nSLASH = \"/\"\nEQ = \"=\"\ngroup beg:\n  EQ = \"=\"\nskip:\n  WS = /[ \\t]+/\ntransitions:\n  on SLASH in beg -> refuse\n  on NAME -> set-mode default\n  on (EQ | SLASH) -> set-mode beg\n",
    );
    assert_eq!(lex(&g, "a = b / c"), ["NAME:a", "EQ:=", "NAME:b", "SLASH:/", "NAME:c"]);
    assert!(lex_err(&g, "a = /b/").contains("in mode 'beg'"));
    assert!(lex_err(&g, "/b/").starts_with("1:1:"));
}

#[test]
fn the_built_in_newline_can_trigger_a_refusal() {
    let g = grammar("NAME = /[a-z]+/\ntransitions:\n  on NEWLINE if line_start -> refuse\n");
    // A newline at the start of a line is a blank line.
    assert_eq!(lex(&g, "a\n"), ["NAME:a", "NEWLINE:\\n"]);
    assert!(lex_err(&g, "a\n\n").starts_with("2:1:"));
}

// ---------------------------------------------------------------------------
// Backward compatibility
// ---------------------------------------------------------------------------

#[test]
fn rules_without_conditions_and_grammars_without_guards_are_unchanged() {
    let plain = grammar("NAME = /[a-z]+/\nSLASH = \"/\"\ngroup div:\n  SLASH = \"/\"\ntransitions:\n  on NAME -> set-mode div\n  on SLASH -> set-mode default\n");
    let guarded_never_used = grammar("NAME = /[a-z]+/\nSLASH = \"/\"\nEMBDOC = /=begin/ @line_start\ngroup div:\n  SLASH = \"/\"\ntransitions:\n  on NAME -> set-mode div\n  on SLASH -> set-mode default\n");
    for source in ["a/b/c", "a / b\nc", "/a", ""] {
        assert_eq!(lex(&plain, source), lex(&guarded_never_used, source), "{source:?}");
    }
}
