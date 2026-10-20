//! The compiler walks the parse tree (LANG84 decision 6).
//!
//! Since v0.9 the only input of the IIR compiler is the generic
//! `GrammarASTNode` CST that `mccarthy_lisp_parser::parse_to_cst` builds
//! from `mccarthy_lisp.grammar`.  This file pins that down three ways:
//!
//! 1. **A source check** — `src/lib.rs` never names the parser crate's
//!    typed-AST entry points (`parse`, `extract_program`, `LispExpr`), so
//!    no future edit can quietly put the typed lift back on the IIR path.
//! 2. **The public CST entry point** — `compile_ast` accepts a tree from
//!    `parse_to_cst`, agrees with `compile_source`, and rejects malformed
//!    hand-built trees with the same `parse:` errors the old extractor gave.
//! 3. **CST edge cases, run end to end** on `mccarthy-lisp-vm` — the spots
//!    where the old typed AST hid a sugar expansion the compiler must now
//!    perform itself: `'` quotes (including nested ones), the empty list,
//!    dotted pairs (including dotted spellings of calls), and comments.

use dynval_runtime::{name_of, LispyValue};
use mccarthy_lisp_iir_compiler::{compile_ast, compile_source};
use mccarthy_lisp_parser::{parse_to_cst, MAX_AST_DEPTH};
use mccarthy_lisp_vm::run;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ============================================================
// 1. Source check: no typed lift on the IIR path
// ============================================================

/// The compiler's own source, as compiled into this test binary.
const LIB_RS: &str = include_str!("../src/lib.rs");

/// The *code* lines of `src/lib.rs`: everything outside `//` comments
/// (doc comments included).  Comments may — and do — talk about the old
/// typed-AST pipeline; only code matters here.
fn code_lines() -> Vec<&'static str> {
    LIB_RS
        .lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .filter(|l| !l.trim().is_empty())
        .collect()
}

#[test]
fn compiler_source_never_names_the_typed_ast() {
    // Built with `concat!` so this test's own source does not contain the
    // forbidden words as plain literals.
    let forbidden = [concat!("extract", "_program"), concat!("Lisp", "Expr")];
    for line in code_lines() {
        for word in forbidden {
            assert!(
                !line.contains(word),
                "src/lib.rs code mentions `{word}` — the IIR compiler must walk the \
                 GrammarASTNode, not the typed AST:\n{line}"
            );
        }
    }
}

#[test]
fn compiler_source_imports_only_cst_entry_points_from_the_parser_crate() {
    // Every item the compiler names from `mccarthy_lisp_parser` must be on
    // this list.  `parse` (CST → typed AST) is deliberately absent.
    let allowed = ["parse_to_cst", "ParseError", "MAX_AST_DEPTH"];
    let prefix = "mccarthy_lisp_parser::";
    let mut seen = 0;
    for line in code_lines() {
        let mut rest = line;
        while let Some(i) = rest.find(prefix) {
            rest = &rest[i + prefix.len()..];
            // Either a single path segment or a `{a, b, c}` group.
            let names: Vec<&str> = if let Some(group) = rest.strip_prefix('{') {
                let end = group.find('}').expect("closing brace on the same line");
                group[..end].split(',').map(str::trim).filter(|s| !s.is_empty()).collect()
            } else {
                let end = rest
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                vec![&rest[..end]]
            };
            for name in names {
                seen += 1;
                assert!(
                    allowed.contains(&name),
                    "src/lib.rs uses `{prefix}{name}`; only {allowed:?} are allowed on the \
                     IIR path (LANG84: the compiler's only input is the parse tree)"
                );
            }
        }
    }
    assert!(seen > 0, "expected the compiler to import parse_to_cst from the parser crate");
}

// ============================================================
// 2. compile_ast — the CST entry point
// ============================================================

#[test]
fn compile_ast_agrees_with_compile_source() {
    for src in [
        "(CAR '(A B C))",
        "((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ('T (FF (CAR X)))))) '((A B) C))",
        "(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)",
        "",
    ] {
        let cst = parse_to_cst(src).expect("parse");
        let a = compile_ast(&cst, "m").expect("compile_ast");
        let b = compile_source(src, "m").expect("compile_source");
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "IIR differs for {src:?}");
    }
}

#[test]
fn compile_ast_rejects_a_non_program_root() {
    // Hand the compiler a `sexpr` node instead of the `program` root.
    let cst = parse_to_cst("'A").unwrap();
    let ASTNodeOrToken::Node(sexpr) = &cst.children[0] else { panic!("expected a node") };
    let e = compile_ast(sexpr, "m").unwrap_err();
    assert!(e.message.starts_with("parse: "), "{}", e.message);
    assert!(e.message.contains("expected 'program' root, got \"sexpr\""), "{}", e.message);
}

#[test]
fn integer_overflow_is_still_a_parse_error() {
    // The CST keeps literals as text, so this tree builds fine; the
    // compiler's validation pass must reject it exactly as the old
    // extractor did (same `parse:` prefix, same position, same words).
    let cst = parse_to_cst("(CONS 'A 99999999999999999999)").expect("CST holds the text");
    let e = compile_ast(&cst, "m").unwrap_err();
    assert_eq!(
        e.message,
        "parse: parse error at 1:10: integer literal \"99999999999999999999\" does not fit in i64"
    );
}

#[test]
fn integer_overflow_is_reported_before_any_lowering_error() {
    // `X` is unbound (a lowering error) but the bad literal comes later in
    // the source: validation runs first, so the literal wins — as before.
    let e = compile_source("X 99999999999999999999", "m").unwrap_err();
    assert!(e.message.contains("does not fit in i64"), "{}", e.message);
}

/// Wrap `inner` (a `sexpr`) in `levels` more `quoted` layers by hand,
/// reusing a real `quoted` node from the parser as the template — so no
/// token has to be fabricated.
fn hand_built_quote_chain(levels: usize) -> GrammarASTNode {
    let template = parse_to_cst("'A").unwrap();
    let ASTNodeOrToken::Node(outer) = &template.children[0] else { unreachable!() };
    let mut node = outer.clone(); // sexpr → quoted → (QUOTE, sexpr A)
    for _ in 0..levels {
        let mut wrapper = outer.clone();
        let ASTNodeOrToken::Node(quoted) = &mut wrapper.children[0] else { unreachable!() };
        quoted.children[1] = ASTNodeOrToken::Node(node);
        node = wrapper;
    }
    let mut program = template.clone();
    program.children = vec![ASTNodeOrToken::Node(node)];
    program
}

#[test]
fn hand_built_over_deep_tree_is_rejected() {
    // `parse_to_cst` would never produce this (its nesting caps are far
    // lower), but `compile_ast` is public, so it guards itself too.  Run on
    // a big stack: the point is the clean error, not stack headroom.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let deep = hand_built_quote_chain(MAX_AST_DEPTH);
            let e = compile_ast(&deep, "m").unwrap_err();
            assert!(e.message.contains("MAX_AST_DEPTH"), "{}", e.message);
            // A shallow hand-built chain is fine: ''''A.
            let ok = hand_built_quote_chain(3);
            assert!(compile_ast(&ok, "m").is_ok());
        })
        .unwrap()
        .join()
        .unwrap();
}

// ============================================================
// 3. CST edge cases, end to end
// ============================================================

fn eval(src: &str) -> LispyValue {
    let module = compile_source(src, "e2e").unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    run(&module).unwrap_or_else(|e| panic!("run {src:?}: {e}"))
}

fn err(src: &str) -> String {
    compile_source(src, "t").expect_err("expected a compile error").message
}

fn iir(src: &str) -> String {
    format!("{:?}", compile_source(src, "t").unwrap_or_else(|e| panic!("{src:?}: {e}")))
}

fn car(v: LispyValue) -> LispyValue {
    dynval_runtime::builtins::car(&[v]).expect("car")
}
fn cdr(v: LispyValue) -> LispyValue {
    dynval_runtime::builtins::cdr(&[v]).expect("cdr")
}

/// Render a value as McCarthy dotted-pair text: `(A . (B . NIL))`.
fn show(v: LispyValue) -> String {
    if v.is_nil() {
        "NIL".into()
    } else if let Some(id) = v.as_symbol() {
        name_of(id).expect("interned")
    } else if let Some(n) = v.as_int() {
        n.to_string()
    } else {
        format!("({} . {})", show(car(v)), show(cdr(v)))
    }
}

// ---- quote ----

#[test]
fn quote_sugar_and_spelled_out_quote_emit_identical_iir() {
    // `'X` has no `QUOTE` token in the tree — the compiler supplies it.
    assert_eq!(iir("'A"), iir("(QUOTE A)"));
    assert_eq!(iir("'(A B)"), iir("(QUOTE (A B))"));
    assert_eq!(iir("(CAR '(A B))"), iir("(CAR (QUOTE (A B)))"));
}

#[test]
fn nested_quotes_are_data() {
    // ''A is (QUOTE (QUOTE A)): its value is the two-element list (QUOTE A).
    assert_eq!(show(eval("''A")), "(QUOTE . (A . NIL))");
    assert_eq!(show(eval("'''A")), "(QUOTE . ((QUOTE . (A . NIL)) . NIL))");
    assert_eq!(show(eval("(CAR ''A)")), "QUOTE");
    assert_eq!(show(eval("(CAR (CDR ''A))")), "A");
}

#[test]
fn quote_inside_a_quoted_list_is_data() {
    // '(A 'B) is (A (QUOTE B)).
    assert_eq!(show(eval("'(A 'B)")), "(A . ((QUOTE . (B . NIL)) . NIL))");
    // A quote as a dotted tail: '(A . 'B) is (A . (QUOTE B)) = (A QUOTE B).
    assert_eq!(show(eval("'(A . 'B)")), "(A . (QUOTE . (B . NIL)))");
}

#[test]
fn quote_arity_is_checked_like_any_form() {
    assert!(err("(QUOTE A B)").contains("`QUOTE` expects 1 argument(s), got 2"));
    assert!(err("(QUOTE)").contains("`QUOTE` expects 1 argument(s), got 0"));
}

#[test]
fn quote_shadowed_by_a_parameter_is_still_an_application() {
    // `'A` *means* `(QUOTE A)`; if `QUOTE` is a parameter, that is a dynamic
    // application of the parameter to the (unbound) variable `A` — exactly
    // what the typed-AST pipeline did, kept for IIR identity.
    assert!(err("((LAMBDA (QUOTE) 'A) 'B)").contains("unbound variable `A`"));
}

#[test]
fn a_quote_inside_a_lambda_body_captures_nothing() {
    // Quoted data has no variable references, so the inner lambda's
    // reference to X inside '(X Y) is not a capture.
    let m = compile_source("((LAMBDA (X) (LAMBDA (Y) '(X Y))) 'A)", "t").unwrap();
    let inner = m.get_function("lambda_1").unwrap();
    assert_eq!(inner.param_names(), vec!["Y"]);
}

// ---- empty list ----

#[test]
fn empty_list_is_nil_quoted_or_not() {
    // `()` has an empty `list_body` node in the tree.
    assert!(eval("()").is_nil());
    assert!(eval("'()").is_nil());
    assert_eq!(iir("()"), iir("'()"));
    assert_eq!(show(eval("'(())")), "(NIL . NIL)");
    assert!(eval("(ATOM '())").is_true());
}

#[test]
fn empty_list_cannot_be_applied() {
    assert!(err("(() 'A)").contains("cannot apply a empty list"));
}

// ---- dotted pairs ----

#[test]
fn dotted_pairs_are_data() {
    assert_eq!(show(eval("'(A . B)")), "(A . B)");
    assert_eq!(show(eval("'(A B . C)")), "(A . (B . C))");
    assert_eq!(show(eval("'((A . B) . (C . D))")), "((A . B) . (C . D))");
    assert_eq!(show(eval("'(1 . -2)")), "(1 . -2)");
}

#[test]
fn a_dotted_list_tail_is_the_same_list() {
    // (A . (B C)) and (A B C) are the same S-expression.
    assert_eq!(iir("'(A . (B C))"), iir("'(A B C)"));
    assert_eq!(iir("'(A B . ())"), iir("'(A B)"));
}

#[test]
fn dotted_spellings_of_calls_are_the_same_calls() {
    // McCarthy reads code as cons structure, so these are ordinary calls.
    assert_eq!(iir("(CAR . ('(A B)))"), iir("(CAR '(A B))"));
    assert_eq!(iir("(CONS 'A . ('B))"), iir("(CONS 'A 'B)"));
    assert_eq!(iir("((LAMBDA . ((X) X)) 'A)"), iir("((LAMBDA (X) X) 'A)"));
    assert_eq!(show(eval("(CAR . ('(A B)))")), "A");
    // A quote as the tail of a call: (CAR . 'X) is (CAR QUOTE X).
    assert!(err("(CAR . 'X)").contains("`CAR` expects 1 argument(s), got 2"));
}

#[test]
fn improper_forms_are_not_callable() {
    assert!(err("(A . B)").contains("improper list in call position"));
    assert!(err("(CONS 'A . 'B)").contains("`CONS` expects 2 argument(s), got 3"));
    assert!(err("(COND ('T 'A) . 'B)").contains("a COND clause must be a list"));
    assert!(err("((LAMBDA (X . Y) X) 'A)").contains("parameter list must be a proper list"));
}

// ---- comments ----

#[test]
fn comments_never_reach_the_tree() {
    // COMMENT is a skip pattern: the tree (and so the IIR) is the same with
    // or without comments, wherever they sit.
    let commented = "; leading comment\n\
                     (CAR ; after the head\n\
                       '(A ; inside a quoted list\n\
                         B)) ; trailing\n\
                     ; final comment";
    assert_eq!(iir(commented), iir("(CAR '(A B))"));
    assert_eq!(show(eval(commented)), "A");
    // A program that is only comments is the empty program → nil.
    assert!(eval("; nothing here\n; at all").is_nil());
}
