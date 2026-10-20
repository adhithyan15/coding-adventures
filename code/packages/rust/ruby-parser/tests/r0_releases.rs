//! Per-release R0 lexing: the 1994 pairs against their own `yylex()`, the
//! LANG84 gates on both sides (`&.` from 2.3, `->` from 1.9, `@@` from 1.6,
//! `0b`/`0o`/`0d` and `?:`), and the documented R0 divergences.
//!
//! There is no runnable oracle for Ruby 0.49 or 0.50, so the expected streams
//! below are read off the tagged parse.y (blob IDs in each pair's header).
//! The comment on each case names the yylex path it follows.

use coding_adventures_ruby_parser::{tokenize_ruby_at, FrontendBudgets, RubyFrontendError};
use lexer::token::TokenType;

fn lex(release: &str, source: &str) -> Result<Vec<(String, String)>, RubyFrontendError> {
    let (_, tokens) = tokenize_ruby_at(source, Some(release), &FrontendBudgets::default())?;
    Ok(tokens
        .into_iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(|t| (t.effective_type_name().to_string(), t.value))
        .collect())
}

fn stream(release: &str, source: &str) -> String {
    lex(release, source)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(kind, value)| format!("{kind}:{value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

const EARLY: [&str; 2] = ["0.49", "0.50"];
const MODERN: [&str; 3] = ["3.3.5", "3.3.6", "3.3.7"];

#[test]
fn early_lexer_folds_a_sign_into_a_number_only_in_expression_start() {
    for r in EARLY {
        // `case '-'`: in EXPR_BEG/EXPR_MID with a digit next, goto start_num.
        assert_eq!(stream(r, "puts(-7 / 2)"), "NAME:puts LPAREN:( NUMBER:-7 SLASH:/ NUMBER:2 RPAREN:)");
        assert_eq!(stream(r, "x = +1.5e3"), "NAME:x EQUALS:= FLOAT:+1.5e3");
        // After a value (EXPR_END) `-` is the binary operator.
        assert_eq!(stream(r, "x = 1 -1"), "NAME:x EQUALS:= NUMBER:1 MINUS:- NUMBER:1");
        // Not followed by a digit: an ordinary operator.
        assert_eq!(stream(r, "x = - 1"), "NAME:x EQUALS:= MINUS:- NUMBER:1");
        // start_num with a sign skips the `0x`/octal checks.
        assert_eq!(stream(r, "x = -0x1"), "NAME:x EQUALS:= NUMBER:-0 NAME:x1");
    }
}

#[test]
fn early_numeric_literals_follow_start_num() {
    for r in EARLY {
        // `_` is ignored anywhere in a decimal literal; nothing checks the
        // character after a number.
        assert_eq!(stream(r, "1_000 1__2 1_"), "NUMBER:1_000 NUMBER:1__2 NUMBER:1_");
        // `0x` reads hex digits (none required); `0` + digit reads an octal
        // run of 0-9; `0b` and `0o` do not exist yet.
        assert_eq!(stream(r, "0x1F 0x 017 09"), "HEX_INTEGER:0x1F HEX_INTEGER:0x OCTAL_INTEGER:017 OCTAL_INTEGER:09");
        assert_eq!(stream(r, "0b1"), "NUMBER:0 NAME:b1");
        // Fraction and exponent in either order, exponent digits optional,
        // and a bare `.digit` is a float.
        assert_eq!(stream(r, "1.5 1e5 1e 2e3.5 .5"), "FLOAT:1.5 FLOAT:1e5 FLOAT:1e FLOAT:2e3.5 FLOAT:.5");
        assert_eq!(stream(r, "1..5"), "NUMBER:1 DOT2:.. NUMBER:5");
    }
}

#[test]
fn early_identifiers_variables_and_symbols() {
    for r in EARLY {
        // Constants are `%Name` at expression start; upper-case names are
        // plain identifiers. After a value `%` is the modulo operator.
        assert_eq!(stream(r, "x = %Foo % Bar"), "NAME:x EQUALS:= CONSTANT:%Foo PERCENT:% NAME:Bar");
        // No @@class variables: `@` then `@b`.
        assert_eq!(stream(r, "@a @@b"), "IVAR:@a AT:@ IVAR:@b");
        // One-character specials win before the identifier form.
        assert_eq!(stream(r, "$_foo $0 $stdout $"), "GVAR:$_ NAME:foo GVAR:$0 GVAR:$stdout DOLLAR:$");
        // Symbols are written `\sym`; `:` is only an operator.
        assert_eq!(stream(r, "x = \\foo"), "NAME:x EQUALS:= BACKSLASH:\\ NAME:foo");
        // `?` always starts a character literal: there is no ternary.
        assert_eq!(stream(r, "x ? 1 : 2"), "NAME:x CHAR:?  NUMBER:1 COLON:: NUMBER:2");
        // Identifiers never take a `?`/`!` suffix.
        assert_eq!(stream(r, "a.nil?(1)"), "NAME:a DOT:. KEYWORD:nil CHAR:?( NUMBER:1 RPAREN:)");
        // `__END__` (token 0) ends the program wherever it appears.
        assert_eq!(stream(r, "puts(1)\n__END__ puts(\"x\""), "NAME:puts LPAREN:( NUMBER:1 RPAREN:) NEWLINE:\\n");
    }
}

#[test]
fn early_keyword_tables_differ_between_049_and_050() {
    assert_eq!(stream("0.49", "func __FILE__"), "KEYWORD:func NAME:__FILE__");
    assert_eq!(stream("0.50", "func __FILE__"), "NAME:func KEYWORD:__FILE__");
    for r in EARLY {
        assert_eq!(stream(r, "resque protect using true"), "KEYWORD:resque KEYWORD:protect KEYWORD:using NAME:true");
    }
    // 0.49 has a backtick token; from 0.50 a backtick opens an xstring,
    // which is outside R0, so the pair refuses it.
    assert_eq!(stream("0.49", "`"), "BACKTICK:`");
    assert!(matches!(lex("0.50", "`ls`"), Err(RubyFrontendError::Lex { .. })));
}

#[test]
fn gate_safe_navigation_arrives_in_2_3() {
    for r in EARLY {
        assert_eq!(stream(r, "a&.b"), "NAME:a AMPERSAND:& DOT:. NAME:b");
    }
    for r in MODERN {
        assert_eq!(stream(r, "a&.b"), "NAME:a SAFE_NAVIGATION:&. NAME:b");
    }
}

#[test]
fn gate_lambda_arrow_arrives_in_1_9() {
    for r in EARLY {
        assert_eq!(stream(r, "f = ->(x)"), "NAME:f EQUALS:= MINUS:- GREATER_THAN:> LPAREN:( NAME:x RPAREN:)");
    }
    for r in MODERN {
        assert_eq!(stream(r, "f = ->(x)"), "NAME:f EQUALS:= LAMBDA:-> LPAREN:( NAME:x RPAREN:)");
    }
}

#[test]
fn gates_for_class_variables_radix_prefixes_and_ternary() {
    for r in MODERN {
        assert_eq!(stream(r, "@a @@b"), "IVAR:@a CVAR:@@b");
        assert_eq!(
            stream(r, "0b1 0o7 0d9 017 0_7 0x1F"),
            "BINARY_INTEGER:0b1 OCTAL_INTEGER:0o7 DECIMAL_PREFIXED_INTEGER:0d9 OCTAL_INTEGER:017 OCTAL_INTEGER:0_7 HEX_INTEGER:0x1F"
        );
        assert_eq!(stream(r, "x ? 1 : 2"), "NAME:x QUESTION:? NUMBER:1 COLON:: NUMBER:2");
        assert_eq!(stream(r, "x = :a"), "NAME:x EQUALS:= SYMBEG:: NAME:a");
        // Ruby rejects these at lex time; so does the pair.
        for bad in ["1_", "1__2", "0x", "08", "0b2", "1r", "2i", "@1"] {
            assert!(lex(r, bad).is_err(), "{r}: {bad:?}");
        }
    }
}

/// Each documented divergence in the 3.3.x pair header, pinned so that a
/// later slice that fixes one has to update the header too.
#[test]
fn documented_modern_divergences() {
    for r in MODERN {
        // No lookahead: `x!=y` without a space lexes as FID `x!` then `=`.
        assert_eq!(stream(r, "x!=y"), "FID:x! EQUALS:= NAME:y");
        assert_eq!(stream(r, "x != y"), "NAME:x NOT_EQUALS:!= NAME:y");
        // Operator and variable symbols are not SYMBEG + operand.
        assert_eq!(stream(r, "x = :@iv"), "NAME:x EQUALS:= COLON:: IVAR:@iv");
        // Transitions do not fire on NEWLINE, so the mode carries over.
        assert_eq!(stream(r, "x = 1\n?a"), "NAME:x EQUALS:= NUMBER:1 NEWLINE:\\n QUESTION:? NAME:a");
        // `foo +1` would fold in Ruby (space before, none after).
        assert_eq!(stream(r, "foo +1"), "NAME:foo PLUS:+ NUMBER:1");
        // `1if` is valid Ruby but a lexer error here.
        assert!(lex(r, "x = 1if true").is_err());
        // Out-of-slice constructs: strings are refused, regexps lex as
        // division, heredocs as `<<`.
        assert!(lex(r, "puts 'x'").is_err());
        assert_eq!(stream(r, "a / b"), "NAME:a SLASH:/ NAME:b");
    }
    // The same NEWLINE carry-over in the 1994 lexer: yylex would fold `-2`.
    assert_eq!(stream("0.49", "x = 1\n-2"), "NAME:x EQUALS:= NUMBER:1 NEWLINE:\\n MINUS:- NUMBER:2");
}
