//! Reserved identifier prefixes (LANG83 PR7f-2 security review).
//!
//! The lowering passes after this compiler add functions of their own —
//! `__lisp_equal`, `__dyn_list_length`, … — and recognise them by name, so a
//! Twig program may not define, bind or reference a name that begins with
//! `__lisp_` or `__dyn_`. A second review added `__lambda_` (this compiler's
//! lifted lambdas) and `__twig_` (the native runtime). Each test below is a
//! program that, before the reservation, compiled and then silently changed
//! what a builtin meant.

use twig_ir_compiler::{compile_source, reserved_prefix, RESERVED_PREFIXES};

/// Compile `src` and return the error message, panicking if it compiled.
fn refused(src: &str) -> twig_ir_compiler::TwigCompileError {
    match compile_source(src, "m") {
        Ok(_) => panic!("{src:?} must be refused"),
        Err(e) => e,
    }
}

/// The two security-review repros: a program function named like the
/// `equal?` helper, and a program "calling" the inline `not` marker.
#[test]
fn the_security_review_repros_are_refused() {
    let e = refused("(define (__lisp_equal a b) 42) (equal? 1 2)");
    assert!(e.message.contains("\"__lisp_equal\" is reserved"), "{}", e.message);
    assert_eq!((e.line, e.column), (1, 10), "the definition's name is the position");

    let e = refused("(define (__lisp_not x) 0) (__lisp_not 5)");
    assert!(e.message.contains("__lisp_"), "{}", e.message);
    assert_eq!((e.line, e.column), (1, 10), "the first use, in source order");

    // A reference alone — no definition — is refused too: the compiler
    // would otherwise bind it to its own helper.
    let e = refused("(__lisp_equal 1 2)");
    assert_eq!((e.line, e.column), (1, 2));
}

/// The `__dyn_` prefix has the same exposure: `length` lowers to a call of a
/// module-local `__dyn_list_length` (injected only if no function of that
/// name exists), closures to `__dyn_call_closure`, and the C runtime exports
/// `__dyn_pair_new` and friends.
#[test]
fn the_dynamic_value_prefix_is_reserved_too() {
    for src in [
        "(define (__dyn_list_length l) 99) (length (list 1 2))",
        "(define (__dyn_call_closure c a) 0) 1",
        "(define (__dyn_pair_new a b) 0) 1",
    ] {
        let e = refused(src);
        assert!(e.message.contains("\"__dyn_\""), "{src}: {}", e.message);
    }
}

/// Every binding position is checked, not only top-level function names.
#[test]
fn every_kind_of_name_is_checked() {
    for src in [
        "(define __lisp_x 1) __lisp_x",                // a value define
        "(define (f __dyn_x) __dyn_x) (f 1)",          // a parameter
        "(let ((__lisp_y 1)) __lisp_y)",               // a let binding
        "((lambda (__dyn_z) __dyn_z) 1)",              // a lambda parameter
        "(define (f x) (__lisp_truthy x)) (f 1)",      // a reference in a body
        "(define (__LISP_EQUAL a b) 1) 1",             // case is ignored
    ] {
        refused(src);
    }
}

/// Quoted symbols are data, not names; names that merely contain the
/// prefix, or share its letters without the trailing underscore, are fine.
#[test]
fn symbols_and_lookalikes_are_not_reserved() {
    for src in [
        "(define s '__lisp_equal) 1",
        "(define s (quote __dyn_pair_new)) 1",
        "(define (my__lisp_x a) a) (my__lisp_x 1)",
        "(define (__lispy a) a) (__lispy 1)",
        "(define (_lisp_x a) a) (_lisp_x 1)",
    ] {
        compile_source(src, "m").unwrap_or_else(|e| panic!("{src}: {e}"));
    }
}

#[test]
fn reserved_prefix_reads_each_segment_and_ignores_case() {
    assert_eq!(RESERVED_PREFIXES, ["__lisp_", "__dyn_", "__lambda_", "__twig_"]);
    assert_eq!(reserved_prefix("__lisp_equal"), Some("__lisp_"));
    assert_eq!(reserved_prefix("__DYN_x"), Some("__dyn_"));
    assert_eq!(reserved_prefix("host/__dyn_x"), Some("__dyn_"));
    assert_eq!(reserved_prefix("__lispy"), None);
    assert_eq!(reserved_prefix("my__lisp_x"), None);
    assert_eq!(reserved_prefix("__twig_x"), Some("__twig_"));
    assert_eq!(reserved_prefix("__Lambda_3"), Some("__lambda_"));
    assert_eq!(reserved_prefix("__lambda"), None);
    assert_eq!(reserved_prefix("__twigx"), None);
}

/// The second security review's repros. `print` lowers to a call of the
/// runtime's `__twig_print_i64`, and a `(lambda …)` is lifted into a function
/// this compiler names `__lambda_N`; a program that could spell either name
/// could stand in for the runtime or for one of its own lambdas.
#[test]
fn runtime_and_lambda_names_are_reserved() {
    let e = refused("(define (__twig_print_i64 n) 0) (begin (print 5) 0)");
    assert!(e.message.contains("\"__twig_\""), "{}", e.message);
    assert_eq!((e.line, e.column), (1, 10));

    let e = refused(
        "(define (__lambda_0 y) (+ y 50)) (define (g f) (+ 0 (f 5))) \
         (+ (__lambda_0 1) (g (lambda (y) (+ y 1))))",
    );
    assert!(e.message.contains("\"__lambda_\""), "{}", e.message);

    // A reference alone is refused too: `(__lambda_0 1)` would call the
    // compiler's own lifted lambda.
    refused("(define (g f) (f 5)) (+ (g (lambda (y) y)) (__lambda_0 1))");
}

/// The lambda the compiler lifts keeps its `__lambda_N` name, and since no
/// source name can begin with `__lambda_`, it is the only function so named.
#[test]
fn a_lifted_lambda_is_the_only_function_with_its_name() {
    let m = compile_source("(define (g f) (+ 0 (f 5))) (+ (g (lambda (y) (+ y 1))) 1)", "m").expect("compiles");
    let lambdas: Vec<&str> =
        m.functions.iter().map(|f| f.name.as_str()).filter(|n| n.starts_with("__lambda_")).collect();
    assert_eq!(lambdas, ["__lambda_0"]);
}
