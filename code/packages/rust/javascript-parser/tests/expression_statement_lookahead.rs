//! ECMA-262 ExpressionStatement lookahead (F12 phase 0).
//!
//! An ExpressionStatement may not start with `{`, `function`, `class`,
//! `let [` or `async function`: at the start of a statement those begin a
//! Block or a declaration. Before the lookahead, a block that ASI had not yet
//! fixed up (`{ a }` with no `;`) fell through to `expression_statement` and
//! parsed as an object literal, so `{ a }⏎/foo/g.exec(s)` compiled to
//! `({a})/foo/g.exec(s)`. With the lookahead such input fails to parse until
//! the parser can pick the lexical goal itself (F12 phase 3). That is fail
//! closed: a refusal instead of a different program.

use coding_adventures_javascript_parser::parse_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;

fn parses(src: &str, version: EsVersion) -> bool {
    parse_javascript_typed(src, version).is_ok()
}

#[test]
fn a_statement_starting_with_a_brace_is_never_an_object_literal() {
    for version in [EsVersion::Es5, EsVersion::Es2015, EsVersion::Es2025] {
        // Each of these used to parse as `({…}) / …`.
        for src in ["{ a } / 2;", "{} / 2;", "{ a: 1 } / 2;"] {
            assert!(!parses(src, version), "{version:?}: {src:?} must not parse");
        }
        // Parenthesised, the object literal is an expression again.
        for src in ["({a: 1}) / 2;", "({}).toString();", "x = {a: 1} / 2;"] {
            assert!(parses(src, version), "{version:?}: {src:?} must parse");
        }
        // Blocks are still blocks, ASI included.
        for src in ["{ a }", "{a: 1}", "{a, b}", "do { x() } while (0)"] {
            assert!(parses(src, version), "{version:?}: {src:?} must parse");
        }
    }
}

#[test]
fn a_function_in_a_block_is_a_declaration_or_refused() {
    // ES2015+ blocks hold declarations (#17877). ES5's Block holds only
    // statements, so the declaration used to slip through as a function
    // *expression* that binds nothing; now it is refused.
    assert!(parses("{ function q() {} }", EsVersion::Es2025));
    assert!(!parses("{ function q() {} }", EsVersion::Es5));
}

#[test]
fn a_block_then_a_regex_fails_closed_until_the_parser_picks_the_goal() {
    // ES2025's lexer guesses division after `}`, so the regex is lost and the
    // program is refused (F12 phase 3 will re-lex it). Editions without the
    // regex/division modes always lex `/foo/g` as a regex, so they parse it.
    assert!(!parses("{ a }\n/foo/g.exec(s)", EsVersion::Es2025));
    assert!(parses("{ a }\n/foo/g.exec(s)", EsVersion::Es5));
}

#[test]
fn function_and_class_at_statement_start_are_declarations() {
    // Closure refuses both of these; they used to parse as expression
    // statements, and `if (x) function f(){}` compiled to `x&&function f(){}`.
    for src in ["if (x) function f(){}", "l: function f(){} f();"] {
        assert!(!parses(src, EsVersion::Es2025), "{src:?} must not parse");
    }
    assert!(!parses("function () {} ();", EsVersion::Es5));
    assert!(!parses("if (x) class A {}", EsVersion::Es2025));
    for src in [
        "function f() {} f();",
        "class A {} new A();",
        "(function () {})();",
        "(class {});",
    ] {
        assert!(parses(src, EsVersion::Es2025), "{src:?} must parse");
    }
}

#[test]
fn let_bracket_and_async_function_start_declarations_from_es2015_and_es2017() {
    // `let [` starts a lexical declaration; as an expression (`let[0]`) it
    // is refused.
    assert!(!parses("let [0];", EsVersion::Es2025));
    assert!(!parses("if (x) async function f() {}", EsVersion::Es2025));
    for src in ["var g = async () => 1;", "async () => 1;", "let [a] = [1];"] {
        assert!(parses(src, EsVersion::Es2025), "{src:?} must parse");
    }
}
