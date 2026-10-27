//! # Pairs, function values, strict closures and the reserved entry names
//!
//! Four gaps in this frontend, found while fixing #18456 and filed as
//! #18460 and #18461.  Each section below pins one of them at the IR level;
//! every `semantic-ir-to-*` backend has a `twig_pairs_and_closures` test that
//! runs the same programs and checks what they print.
//!
//! ```text
//! gap                                   before                         now
//! ------------------------------------  -----------------------------  ------------------------------
//! (car (cons 1 2))                      no Feature::Pairs: validating  Pairs declared; module
//!                                       backends refuse the module     validates cleanly
//! (define (h x) …) (define g h)         VarRef h, Scope::Global, but   a capture-less closure over h
//!                                       functions are not globals,
//!                                       so g reads an empty slot
//! (define main 5) (main)                calls the generated `main`,    positioned "reserved" error;
//!                                       which calls itself forever     `(main)` alone is unresolved
//! ((lambda (x y) …) 4)                  each backend did something     the closure is strict: every
//!                                       different (nil / NaN / …)      backend raises an arity error
//! ```
//!
//! ## How a strict closure is spelled in SIR
//!
//! SIR already has a marker for "this closure checks its arity": the Ruby
//! frontend lowers `lambda { … }` / `->(){ … }` to
//! `BuiltinCall("lambda", [MakeClosure …])`, while a bare `MakeClosure` is a
//! proc-lenient block (extra arguments dropped, missing ones `nil`).  Twig has
//! no blocks — every Twig closure is a lambda — so this frontend now wraps
//! every `MakeClosure` it emits in that marker.

use semantic_ir::{Expr, Feature, Module, Severity};
use twig_to_semantic_ir::compile_source;

/// Lower `src` and require a module the validator accepts with **no**
/// errors at all — the bar every backend's own validation applies.
fn lower(src: &str) -> Module {
    let m = compile_source(src, "t").unwrap_or_else(|e| panic!("{src}: {e}"));
    let r = semantic_ir::validate(&m);
    let errors: Vec<_> = r
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "{src}: module failed validation: {errors:?}"
    );
    m
}

fn function<'m>(m: &'m Module, name: &str) -> &'m semantic_ir::Function {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no function `{name}`"))
}

/// If `e` is the strict-closure shape `BuiltinCall("lambda", [MakeClosure])`,
/// return the closure's target function and its capture names.
fn strict_closure(e: &Expr) -> Option<(&str, Vec<&str>)> {
    match e {
        Expr::BuiltinCall { name, args, .. } if name == "lambda" && args.len() == 1 => {
            match &args[0] {
                Expr::MakeClosure {
                    fn_name, captures, ..
                } => Some((
                    fn_name.as_str(),
                    captures.iter().map(|c| c.name.as_str()).collect(),
                )),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The value `_init` stores into global `name`.
fn global_init_value<'m>(m: &'m Module, name: &str) -> &'m Expr {
    let init = function(m, "_init");
    for s in &init.body.stmts {
        if let semantic_ir::Stmt::ExprStmt {
            expr: Expr::BuiltinCall { name: b, args, .. },
            ..
        } = s
        {
            if b == "global_set" && matches!(&args[0], Expr::SymLit { name: n, .. } if n == name) {
                return &args[1];
            }
        }
    }
    panic!("_init never sets `{name}`")
}

// ---------------------------------------------------------------------------
// #18460 — Feature::Pairs
// ---------------------------------------------------------------------------

#[test]
fn every_pair_builtin_declares_the_pairs_feature() {
    for src in [
        "(car (cons 1 2))",
        "(cdr (cons 1 2))",
        "(pair? 5)",
        "(cons 1 nil)",
        // Inside a function body and inside a lambda, not just in `main`.
        "(define (f p) (car p)) (f (cons 1 2))",
        "((lambda (p) (cdr p)) (cons 1 2))",
        // Inside a value define's initialiser (`_init`).
        "(define p (cons 1 2)) (car p)",
    ] {
        let m = lower(src);
        assert!(m.manifest.contains(Feature::Pairs), "{src}: {}", m.manifest);
    }
}

#[test]
fn a_program_without_pair_builtins_does_not_declare_pairs() {
    // The declaration tracks use; it is not switched on for everyone.
    for src in ["(+ 1 2)", "(define (f x) (* x 2)) (f 3)", "(null? nil)"] {
        let m = lower(src);
        assert!(
            !m.manifest.contains(Feature::Pairs),
            "{src}: {}",
            m.manifest
        );
    }
}

#[test]
fn a_user_function_named_car_is_not_a_pair_operation() {
    // `(car x)` here calls the user's `car` (#18456), so the only pair
    // builtin left is `cons`; `Pairs` is still declared for that one.
    let m = lower("(define (car p) 42) (car (cons 1 2))");
    assert!(m.manifest.contains(Feature::Pairs));
    // With no `cons` either, nothing is a pair operation.
    let m = lower("(define (car p) 42) (car 1)");
    assert!(!m.manifest.contains(Feature::Pairs), "{}", m.manifest);
}

// ---------------------------------------------------------------------------
// #18461 item 2 — a top-level function used as a value
// ---------------------------------------------------------------------------

#[test]
fn a_function_used_as_a_value_becomes_a_captureless_closure() {
    let m = lower("(define (h x) (* x 3)) (define g h) (g 4)");
    let v = global_init_value(&m, "g");
    assert_eq!(strict_closure(v), Some(("h", vec![])), "{v:?}");
}

#[test]
fn a_function_passed_as_an_argument_becomes_a_closure() {
    let m = lower("(define (h x) (* x 3)) (define (app f v) (f v)) (app h 5)");
    let main = function(&m, "main");
    match &main.body.value {
        Expr::DirectCall { fn_name, args, .. } => {
            assert_eq!(fn_name, "app");
            assert_eq!(
                strict_closure(&args[0]),
                Some(("h", vec![])),
                "{:?}",
                args[0]
            );
        }
        other => panic!("expected DirectCall app, got {other:?}"),
    }
}

#[test]
fn a_function_named_inside_a_lambda_is_not_captured() {
    // `h` is a top-level function, reachable from any body: the lambda
    // needs no capture for it, and the reference itself is the closure.
    let m = lower("(define (h x) x) (define (f) (lambda () h)) ((f))");
    let lam = m
        .functions
        .iter()
        .find(|f| f.name.starts_with("__lambda_"))
        .unwrap();
    assert!(lam.captures.is_empty(), "{:?}", lam.captures);
    assert_eq!(strict_closure(&lam.body.value), Some(("h", vec![])));
}

#[test]
fn a_function_referring_to_itself_as_a_value_is_a_closure() {
    let m = lower("(define (self-ref) self-ref) (self-ref)");
    let f = function(&m, "self-ref");
    assert_eq!(strict_closure(&f.body.value), Some(("self-ref", vec![])));
}

#[test]
fn a_value_define_is_still_read_from_its_global() {
    // Only *functions* changed; a value define stays a global read.
    let m = lower("(define x 5) x");
    match &function(&m, "main").body.value {
        Expr::VarRef { name, scope, .. } => {
            assert_eq!(name, "x");
            assert_eq!(*scope, semantic_ir::Scope::Global);
        }
        other => panic!("expected VarRef Global, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// #18461 item 4 — every Twig closure is strict
// ---------------------------------------------------------------------------

#[test]
fn an_inline_lambda_is_a_strict_closure() {
    let m = lower("(define (adder n) (lambda (x) (+ x n)))");
    let (target, caps) = strict_closure(&function(&m, "adder").body.value)
        .expect("adder returns BuiltinCall(lambda, [MakeClosure])");
    assert_eq!(caps, ["n"]);
    assert_eq!(function(&m, target).params.len(), 1);
}

#[test]
fn no_bare_make_closure_survives() {
    // A bare MakeClosure would be proc-lenient in the backends.  Every one
    // this frontend emits must sit directly inside the `lambda` marker.
    struct Bare(usize);
    impl semantic_ir::Visitor for Bare {
        fn visit_expr(&mut self, e: &Expr, depth: usize) {
            match e {
                // Skip the marker's own MakeClosure, but walk its captures.
                Expr::BuiltinCall { name, args, .. } if name == "lambda" => {
                    if let Some(Expr::MakeClosure { captures, .. }) = args.first() {
                        for c in captures {
                            self.visit_expr(&c.value, depth + 1);
                        }
                    }
                    return;
                }
                Expr::MakeClosure { .. } => self.0 += 1,
                _ => {}
            }
            semantic_ir::walker::walk_expr_default(self, e, depth);
        }
    }
    let m = lower(
        "(define (h x) x)
         (define (mk k) (lambda (a) (k a)))
         (define g (let ((q h)) (lambda (y) (q y))))
         ((mk (lambda (a) 77)) ((lambda () h)))",
    );
    let mut v = Bare(0);
    semantic_ir::Visitor::visit_module(&mut v, &m);
    assert_eq!(v.0, 0, "bare MakeClosure nodes found");
}

// ---------------------------------------------------------------------------
// #18461 item 3 — `main`, `_init` and `__lambda_N` are the compiler's names
// ---------------------------------------------------------------------------

#[test]
fn defining_a_reserved_name_is_a_positioned_error() {
    for (src, name, line, column) in [
        ("(define main 5) (main)", "main", 1, 1),
        ("(define (main) 1)", "main", 1, 1),
        ("(+ 1 2)\n  (define _init 3)", "_init", 2, 3),
        ("(define (_init x) x)", "_init", 1, 1),
        // The gensym'd lambda names are the compiler's too.
        (
            "(define (__lambda_0 x) x) ((lambda () 1))",
            "__lambda_0",
            1,
            1,
        ),
        ("(define __lambda_7 1)", "__lambda_7", 1, 1),
    ] {
        let err = compile_source(src, "t").expect_err(src);
        assert!(
            err.message.starts_with(&format!("`{name}` is reserved")),
            "{src}: {}",
            err.message
        );
        assert_eq!((err.line, err.column), (line, column), "{src}: {err}");
    }
}

#[test]
fn names_that_only_look_reserved_are_ordinary() {
    // Only the exact entry names and the `__lambda_<digits>` gensyms are
    // reserved; these are a user's to define.
    let m = lower(
        "(define (main2 x) x) (define _init_count 1) (define (__lambda_x) 2) \
         (define (my-main) 3) (main2 (+ _init_count (__lambda_x) (my-main)))",
    );
    assert!(m.functions.iter().any(|f| f.name == "main2"));
}

#[test]
fn the_generated_entry_points_are_not_user_visible_names() {
    // Before: `function_names` held `main`/`_init`, so these resolved to the
    // generated functions — `(main)` from inside `main` recursed forever.
    for (src, name) in [
        ("(main)", "main"),
        ("(print main)", "main"),
        ("(_init)", "_init"),
    ] {
        let err = compile_source(src, "t").expect_err(src);
        assert!(err.message.contains("unresolved"), "{src}: {}", err.message);
        assert!(
            err.message.contains(&format!("`{name}`")),
            "{src}: {}",
            err.message
        );
    }
}

#[test]
fn a_local_named_main_is_still_allowed() {
    // A parameter or `let` named `main` shadows nothing the compiler owns.
    lower("(define (f main) (main 1)) (f (lambda (x) x))");
    lower("(let ((main 5)) main)");
}
