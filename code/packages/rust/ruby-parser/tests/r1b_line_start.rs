//! LANG84 R1b: `=begin`/`=end` embedded documents and the `__END__` line in
//! the 3.3.x pairs, and the refusals that keep them from hiding code.
//!
//! Every expectation below was checked against host ruby 3.3.6 (whose
//! parse.y blob `baf391e5` is the one all three 3.3.x pairs cite):
//!
//! | Input                         | Ruby                                  |
//! |-------------------------------|---------------------------------------|
//! | `=begin` … `=end` at col 0    | skipped, rest of both lines too       |
//! | `=beginx`, `  =begin`, `x =begin` | not a comment (a syntax error)    |
//! | `=begin` with no `=end` line  | "embedded document meets end of file" |
//! | `__END__` alone on a line     | end of program, the rest is data      |
//! | `__END__ `, `__END__x`, `  __END__` | an identifier                   |
//!
//! Skipping lines is only safe where Ruby is not inside a multi-line literal,
//! so the second half of this file pins the refusals of every literal opener
//! Ruby might read (LANG84 R1b plan, "Fail closed"), including the `def` head
//! modes, and checks that no token kind can leave `defparams` except `)`.

use std::collections::BTreeSet;
use std::path::Path;

use coding_adventures_ruby_parser::{tokenize_ruby_at, FrontendBudgets, RubyFrontendError};
use grammar_tools::token_grammar::{parse_token_grammar, TransitionAction};
use lexer::token::TokenType;

const MODERN: [&str; 3] = ["3.3.5", "3.3.6", "3.3.7"];

fn lex(release: &str, source: &str) -> Result<Vec<(String, String, usize)>, RubyFrontendError> {
    let (_, tokens) = tokenize_ruby_at(source, Some(release), &FrontendBudgets::default())?;
    Ok(tokens
        .into_iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(|t| (t.effective_type_name().to_string(), t.value, t.line))
        .collect())
}

/// `KIND:value` for each token, space separated.
fn stream(release: &str, source: &str) -> String {
    lex(release, source)
        .unwrap_or_else(|e| panic!("{release} {source:?}: {e}"))
        .into_iter()
        .map(|(kind, value, _)| format!("{kind}:{value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The lexer error message, which must exist.
fn refused(release: &str, source: &str) -> String {
    match lex(release, source) {
        Err(RubyFrontendError::Lex { message, .. }) => message,
        other => panic!("{release} {source:?}: expected a Lex refusal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Embedded documents
// ---------------------------------------------------------------------------

#[test]
fn an_embedded_document_is_skipped_with_the_rest_of_its_first_and_last_lines() {
    for r in MODERN {
        let puts13 = "NAME:puts NUMBER:1 NEWLINE:\\n NAME:puts NUMBER:3 NEWLINE:\\n";
        assert_eq!(stream(r, "puts 1\n=begin\nputs 2\n=end\nputs 3\n"), puts13);
        // Text after the words, separated by any of Ruby's ISSPACE bytes.
        assert_eq!(stream(r, "puts 1\n=begin foo\nputs 2\n=end bar\nputs 3\n"), puts13);
        assert_eq!(stream(r, "puts 1\n=begin\tx\nputs 2\n=end\tz\nputs 3\n"), puts13);
        assert_eq!(stream(r, "puts 1\n=begin\u{b}x\nputs 2\n=end\u{c}\nputs 3\n"), puts13);
        // NUL, ^D and ^Z also end the word (word_match_p).
        assert_eq!(stream(r, "puts 1\n=begin\0x\nputs 2\n=end\u{1a}\nputs 3\n"), puts13);
        // `=endx` does not close the document; the next `=end` does.
        assert_eq!(stream(r, "puts 1\n=begin\nputs 2\n=endx\n=end\nputs 3\n"), puts13);
        // `__END__` inside a document is part of it.
        assert_eq!(stream(r, "puts 1\n=begin\n__END__\n=end\nputs 3\n"), puts13);
    }
}

#[test]
fn crlf_and_the_edges_of_the_input() {
    for r in MODERN {
        assert_eq!(
            stream(r, "puts 1\r\n=begin\r\nputs 2\r\n=end\r\nputs 3\r\n"),
            "NAME:puts NUMBER:1 NEWLINE:\\n NAME:puts NUMBER:3 NEWLINE:\\n"
        );
        // A document as the whole input, with and without the final newline.
        assert_eq!(stream(r, "=begin\n=end"), "");
        assert_eq!(stream(r, "=begin\n=end\n"), "");
        // Offset 0 is a line start.
        assert_eq!(stream(r, "=begin\nx\n=end\nputs 3"), "NAME:puts NUMBER:3");
    }
}

#[test]
fn tokens_after_a_document_keep_their_line_numbers() {
    for r in MODERN {
        let tokens = lex(r, "puts 1\n=begin\na\nb\n=end\nputs 3\n").unwrap();
        let three = tokens.iter().find(|(_, v, _)| v == "3").unwrap();
        assert_eq!(three.2, 6, "{r}");
    }
}

#[test]
fn a_line_continuation_ends_the_line_for_a_document_too() {
    for r in MODERN {
        // ruby 3.3.6 prints 3: the document sits between `+` and `2`.
        assert_eq!(
            stream(r, "puts 1 + \\\n=begin\nx\n=end\n2\n"),
            "NAME:puts NUMBER:1 PLUS:+ NUMBER:2 NEWLINE:\\n"
        );
    }
}

#[test]
fn begin_not_at_a_line_start_or_not_a_whole_word_is_not_a_document() {
    for r in MODERN {
        // Ruby: syntax errors. Here: ordinary tokens, so the program is a
        // parse error too; nothing is skipped.
        assert_eq!(stream(r, "=beginx"), "EQUALS:= NAME:beginx");
        assert_eq!(stream(r, "  =begin\n=end"), "EQUALS:= KEYWORD:begin NEWLINE:\\n EQUALS:= KEYWORD:end");
        assert_eq!(stream(r, "x =begin\n=end"), "NAME:x EQUALS:= KEYWORD:begin NEWLINE:\\n EQUALS:= KEYWORD:end");
    }
}

#[test]
fn an_unterminated_document_is_refused() {
    for r in MODERN {
        for source in ["=begin", "=begin\n", "puts 1\n=begin\nputs 2\n", "=begin x\n=endx\n"] {
            assert!(refused(r, source).contains("EMBDOC_UNTERMINATED"), "{r} {source:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// __END__
// ---------------------------------------------------------------------------

#[test]
fn an_end_line_ends_the_program() {
    for r in MODERN {
        assert_eq!(stream(r, "puts 1\n__END__\nputs 2\n"), "NAME:puts NUMBER:1 NEWLINE:\\n");
        assert_eq!(stream(r, "puts 1\n__END__\r\nputs 2\n"), "NAME:puts NUMBER:1 NEWLINE:\\n");
        assert_eq!(stream(r, "puts 1\n__END__"), "NAME:puts NUMBER:1 NEWLINE:\\n");
        assert_eq!(stream(r, "__END__\nputs 2\n"), "");
        // Data after `__END__` is never lexed, so it may be anything.
        assert_eq!(stream(r, "puts 1\n__END__\n'unterminated \u{0} /x/ <<A\n"), "NAME:puts NUMBER:1 NEWLINE:\\n");
        // `=begin` after `__END__` is data.
        assert_eq!(stream(r, "puts 1\n__END__\n=begin\n"), "NAME:puts NUMBER:1 NEWLINE:\\n");
    }
}

#[test]
fn end_that_is_not_a_whole_line_is_an_identifier() {
    for r in MODERN {
        // Ruby: NameError for `__END__`/`__END__x` at run time, after `1`.
        assert_eq!(stream(r, "puts 1\n__END__ \nputs 2"), "NAME:puts NUMBER:1 NEWLINE:\\n NAME:__END__ NEWLINE:\\n NAME:puts NUMBER:2");
        assert_eq!(stream(r, "__END__x"), "NAME:__END__x");
        assert_eq!(stream(r, "  __END__\n"), "NAME:__END__ NEWLINE:\\n");
        assert_eq!(stream(r, "__END__\r"), "NAME:__END__");
        assert_eq!(stream(r, "x = __END__"), "NAME:x EQUALS:= NAME:__END__");
    }
}

// ---------------------------------------------------------------------------
// Fail closed: literal openers
// ---------------------------------------------------------------------------

#[test]
fn heredoc_openers_are_refused_in_every_mode() {
    for r in MODERN {
        for source in [
            "puts <<A\n=begin\nA\n",
            "x = <<~EOS",
            "x = <<-EOS",
            "x = <<'EOS'",
            "x = <<\"EOS\"",
            "x<<y",
            "puts <<1",
        ] {
            assert!(refused(r, source).contains("HEREDOC_OPEN"), "{r} {source:?}");
        }
        // `<<` followed by a space or `=` is a shift (or `<<=`).
        assert_eq!(stream(r, "a << b"), "NAME:a LEFT_SHIFT:<< NAME:b");
        assert_eq!(stream(r, "a <<= 1"), "NAME:a LEFT_SHIFT_EQUALS:<<= NUMBER:1");
        assert_eq!(stream(r, "a <<- 1"), "NAME:a LEFT_SHIFT:<< MINUS:- NUMBER:1");
    }
}

#[test]
fn an_opener_with_no_space_after_it_is_refused_in_every_mode() {
    for r in MODERN {
        for source in [
            "foo /x/",        // IS_SPCARG: a regexp argument
            "foo %(x)",       // a %-literal argument
            "alias %s(a) %s(b)",
            "x = 1 /2",
            "x/2",
            "a%b",
            "a %=b",
            "foo /#/; exit(1)", // the `#` would otherwise start a comment
        ] {
            let message = refused(r, source);
            assert!(message.contains("SLASH") || message.contains("PERCENT"), "{r} {source:?}: {message}");
        }
    }
}

#[test]
fn an_opener_in_expression_start_is_refused() {
    for r in MODERN {
        for source in ["x = / x/", "x = % x ", "x = %\nfoo\n", "puts(1, / x/)", "x = %= x=", "x = /= x/"] {
            assert!(refused(r, source).contains("in mode 'beg'"), "{r} {source:?}");
        }
        // After a newline Ruby is in EXPR_BEG again.
        assert!(refused(r, "puts 1\n/ x/\n").contains("in mode 'beg'"));
        assert!(refused(r, "puts 1\n% x \n").contains("in mode 'beg'"));
    }
}

#[test]
fn spaced_operators_after_a_value_still_lex() {
    for r in MODERN {
        assert_eq!(stream(r, "a / b"), "NAME:a SLASH:/ NAME:b");
        assert_eq!(stream(r, "puts((1 - 1) % -1)"), "NAME:puts LPAREN:( LPAREN:( NUMBER:1 MINUS:- NUMBER:1 RPAREN:) PERCENT:% MINUS:- NUMBER:1 RPAREN:)");
        assert_eq!(stream(r, "a /= 2; a %= 3"), "NAME:a SLASH_EQUALS:/= NUMBER:2 SEMICOLON:; NAME:a PERCENT_EQUALS:%= NUMBER:3");
        // An operator at the end of a line continues it.
        assert_eq!(stream(r, "puts 7 /\n2"), "NAME:puts NUMBER:7 SLASH:/ NEWLINE:\\n NUMBER:2");
    }
}

#[test]
fn the_def_head_gives_beg_after_its_parameters() {
    for r in MODERN {
        // ruby 3.3.6 reads each `/ x/` as a regexp: `)` of `def` parameters
        // sets EXPR_BEG (parse.y f_paren_args), a newline after `def` or
        // `def self.` is ignored, and so is one inside the parameters.
        for source in [
            "def foo() / x/ end",
            "def foo(a) / x/ end",
            "def foo (a) / x/ end",
            "def foo(a\n) / x/ end",
            "def\nfoo() / x/ end",
            "def self.\nfoo() / x/ end",
            "def self.foo(a, b = 1) % x end",
            "def Foo::bar() / x/ end",
            "def foo=(a) / x/ end",
            "def [](a) / x/ end",
            "def []=(k, v) / x/ end",
            "def +(a) / x/ end",
            "def <=>(a) / x/ end",
            "def end() / x/ end",
        ] {
            assert!(refused(r, source).contains("in mode 'beg'"), "{r} {source:?}");
        }
        // Openers inside the head, nested brackets in the parameters and a
        // `(expr)` singleton are refused outright.
        for (source, mode) in [
            ("def foo = / x/", "defn"),
            ("def foo(a = / x/)", "defparams"),
            ("def foo(a = (1))", "defparams"),
            ("def foo(a = [1])", "defparams"),
            ("def foo(a = {})", "defparams"),
            ("def (x).foo() / y/ end", "deffname"),
        ] {
            assert!(refused(r, source).contains(&format!("in mode '{mode}'")), "{r} {source:?}");
        }
        // The head does not change the tokens: the R0 corpus's `def` lines.
        assert_eq!(
            stream(r, "def save!(a, b); end"),
            "KEYWORD:def FID:save! LPAREN:( NAME:a COMMA:, NAME:b RPAREN:) SEMICOLON:; KEYWORD:end"
        );
        assert_eq!(stream(r, "def self.build; end"), "KEYWORD:def KEYWORD:self DOT:. NAME:build SEMICOLON:; KEYWORD:end");
        // After the name a newline ends the head (ruby: `def foo⏎(1) / 2`
        // returns 0, a division).
        assert_eq!(
            stream(r, "def foo\n(1) / 2\nend"),
            "KEYWORD:def NAME:foo NEWLINE:\\n LPAREN:( NUMBER:1 RPAREN:) SLASH:/ NUMBER:2 NEWLINE:\\n KEYWORD:end"
        );
        // A symbol `:def` is not a def head.
        assert_eq!(stream(r, "x = :def\n(1) / 2"), "NAME:x EQUALS:= SYMBEG:: KEYWORD:def NEWLINE:\\n LPAREN:( NUMBER:1 RPAREN:) SLASH:/ NUMBER:2");
    }
}

/// Exploits a reviewer would try: valid Ruby that runs `exit(1)` while a
/// naive R1b lexer would hide it inside a skipped document.
#[test]
fn literal_bodies_cannot_hide_code_behind_a_document() {
    for r in MODERN {
        for source in [
            // heredoc bodies holding `=begin` and `=end`
            "puts <<A\n=begin\nA\nexit(1)\np <<B\n=end\nB\n",
            // a regexp spanning lines, in expression start
            "x = / \n=begin\n/\nexit(1)\ny = / \n=end\n/\n",
            // a regexp after the def parameters
            "def foo() / \n=begin\n/ end\nexit(1)\ndef bar() / \n=end\n/ end\n",
            // a %-literal argument and a %-literal statement
            "p %(\n=begin\n)\nexit(1)\np %(\n=end\n)\n",
            "puts 1\n% \n=begin\n\nexit(1)\n% \n=end\n\n",
            // the same with __END__
            "x = / \n__END__\n/\nexit(1)\n",
        ] {
            assert!(lex(r, source).is_err(), "{r} {source:?} was not refused");
        }
    }
}

/// R1b security review, F1. A `?` written straight before a quote, a
/// backtick or `#` is refused in every mode: our flat modes only approximate
/// Ruby's lexer state, and where they disagree (after `.end`, after a local)
/// a CHAR or a QUESTION plus a comment could put a string or comment
/// boundary where Ruby has none, and let a line-start document hide code.
#[test]
fn a_question_mark_before_a_quote_or_hash_is_refused_in_every_mode() {
    for r in MODERN {
        for source in [
            // beg, arg, default and the def head
            "?\"", "?'", "?`", "?#",
            "puts ?\"", "puts ?#", "x = 1 ?'", "puts(1) ?`",
            "def foo(a = ?#) end",
            // the review's two programs: Ruby runs both `puts`/`p` lines
            "o = Object.new\ndef o.end(*) = nil\no.end ?#; puts 111\nputs 222\n",
            "x = 1\ny = x ?\"\n=begin\n\" : 0\np 111\nz = x ?\"\n=end\n#\" : 0\np 222\n",
        ] {
            assert!(refused(r, source).contains("QUESTION_OPENER"), "{r} {source:?}");
        }
        // Every other character literal still lexes, and a bare quote or
        // backtick is still a lexer error (strings are not lexed).
        assert_eq!(stream(r, "puts ?a"), "NAME:puts CHAR:?a");
        assert_eq!(stream(r, "x = ?1"), "NAME:x EQUALS:= CHAR:?1");
        assert_eq!(stream(r, "p ?$, ?(, ?/"), "NAME:p CHAR:?$ COMMA:, CHAR:?( COMMA:, CHAR:?/");
        for source in ["puts \"a\"", "puts 'a'", "puts `a`"] {
            assert!(lex(r, source).is_err(), "{r} {source:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// The defparams coverage check
// ---------------------------------------------------------------------------

/// Every token kind a 3.3.x pair can emit must have a rule `in defparams`,
/// or the lexer could leave the parameter list on that token and read the
/// closing `)` as an ordinary one (Ruby: EXPR_BEG after it; here `default`).
#[test]
fn every_token_kind_has_a_rule_in_defparams() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ruby");
    for r in MODERN {
        let text = std::fs::read_to_string(dir.join(format!("ruby{r}.tokens"))).unwrap();
        let grammar = parse_token_grammar(&text).unwrap();
        let mut kinds: BTreeSet<String> = BTreeSet::new();
        let definitions = grammar
            .definitions
            .iter()
            .chain(grammar.groups.values().flat_map(|g| g.definitions.iter()));
        for d in definitions {
            kinds.insert(d.alias.clone().unwrap_or_else(|| d.name.clone()));
        }
        kinds.insert("KEYWORD".to_string());
        kinds.insert("NEWLINE".to_string());
        let covered: BTreeSet<String> = grammar
            .transitions
            .iter()
            .filter(|t| t.in_mode.as_deref() == Some("defparams") && t.conditions.is_empty())
            .flat_map(|t| t.on_tokens.iter().cloned())
            .collect();
        // Refused before any mode is read: the two always-refused openers.
        let always_refused: BTreeSet<String> = grammar
            .transitions
            .iter()
            .filter(|t| t.in_mode.is_none() && t.conditions.is_empty() && t.actions == [TransitionAction::Refuse])
            .flat_map(|t| t.on_tokens.iter().cloned())
            .collect();
        let missing: Vec<&String> = kinds
            .iter()
            .filter(|k| !covered.contains(*k) && !always_refused.contains(*k))
            .collect();
        assert!(missing.is_empty(), "{r}: no rule in defparams for {missing:?}");
    }
}
