//! # Name resolution: lexical scope wins, then the program, then builtins
//!
//! TW00's "Name resolution" section gives one order for every name,
//! whether it is used as a value (`car`) or called (`(car p)`):
//!
//! | Order | Binding                                              | `(name args…)` lowers to          |
//! |-------|------------------------------------------------------|-----------------------------------|
//! | 1     | lexical: param, `let`/`let*`, lambda param, capture  | `IndirectCall` on the local       |
//! | 2     | top-level function `define`                          | `DirectCall name`                 |
//! | 3     | top-level value `define`                             | `IndirectCall` on the `Global`    |
//! | 4     | builtin (`+`, `cons`, `car`, `print`, …)             | `BuiltinCall name`                |
//!
//! Issue #18456: this frontend checked row 4 *first* in call position,
//! then row 2, and only then rows 1 and 3.  So
//!
//! ```scheme
//! (define (car p) 42)
//! (car (cons 1 2))      ; called the builtin `car` → 1, not the user's → 42
//! ```
//!
//! and a parameter named like a top-level function lost to the function:
//!
//! ```scheme
//! (define (helper x) (* x 100))
//! (define (f helper) (helper 1))   ; called the global → 100, not the param
//! ```
//!
//! These tests pin the lowered *shape*.  The JavaScript backend's
//! `twig_name_resolution` test runs the same programs under `node` and
//! checks the printed results.
//!
//! ## How the tests read the IR
//!
//! [`calls`] walks every function with a [`semantic_ir::Visitor`] and
//! records each call node as a short string:
//!
//! ```text
//! builtin:car          BuiltinCall { name: "car" }
//! direct:car           DirectCall  { fn_name: "car" }
//! indirect:local:car   IndirectCall { target: VarRef { name: "car", scope: Local } }
//! indirect:<expr>      IndirectCall whose target is not a VarRef
//! ```
//!
//! so a test can say "no function in this module calls the builtin `car`"
//! with a single `contains`.

use semantic_ir::{walker, Expr, Module, Scope, Visitor};
use twig_to_semantic_ir::compile_source;

/// Lower `src` and check the module validates.
///
/// One validator complaint is tolerated: this frontend never declares
/// `Feature::Pairs` although SIR11's manifest rules say a `cons` / `car` /
/// `cdr` / `pair?` builtin call should.  That gap predates #18456, has
/// nothing to do with name resolution, and needs its own fix; every
/// other issue still fails the test.
fn lower(src: &str) -> Module {
    let m = compile_source(src, "t").unwrap_or_else(|e| panic!("{src}: {e}"));
    let r = semantic_ir::validate(&m);
    let issues: Vec<_> = r
        .issues
        .iter()
        .filter(|i| i.severity == semantic_ir::Severity::Error)
        .filter(|i| !i.message.contains("feature `pairs`"))
        .collect();
    assert!(issues.is_empty(), "{src}: module failed validation: {issues:?}");
    m
}

fn scope_tag(s: Scope) -> &'static str {
    match s {
        Scope::Local => "local",
        Scope::Param => "param",
        Scope::Capture => "capture",
        Scope::Global => "global",
        Scope::Builtin => "builtin",
        _ => "other",
    }
}

/// Collects every call in the visited functions, in walk order.
struct Calls(Vec<String>);

impl Visitor for Calls {
    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        match e {
            Expr::BuiltinCall { name, .. } => self.0.push(format!("builtin:{name}")),
            Expr::DirectCall { fn_name, .. } => self.0.push(format!("direct:{fn_name}")),
            Expr::IndirectCall { target, .. } => match target.as_ref() {
                Expr::VarRef { name, scope, .. } => {
                    self.0.push(format!("indirect:{}:{name}", scope_tag(*scope)))
                }
                _ => self.0.push("indirect:<expr>".to_string()),
            },
            _ => {}
        }
        walker::walk_expr_default(self, e, depth);
    }
}

/// Every call in `fn_name` (or in the whole module when `None`).
fn calls_in(m: &Module, fn_name: Option<&str>) -> Vec<String> {
    let mut v = Calls(Vec::new());
    for f in &m.functions {
        if fn_name.is_none_or(|n| n == f.name) {
            v.visit_function(f);
        }
    }
    v.0
}

/// Every call in the whole module.
fn calls(m: &Module) -> Vec<String> {
    calls_in(m, None)
}

fn has(v: &[String], s: &str) -> bool {
    v.iter().any(|c| c == s)
}

// ---------------------------------------------------------------------------
// Row 2 beats row 4: a top-level FUNCTION named like a builtin
// ---------------------------------------------------------------------------

/// The programs from twig-ir-compiler's `BUILTIN_SHADOWING_PROGRAMS`
/// (#18438) that fit this frontend's builtin table, plus the string-op
/// ones rewritten onto `+`/`car`/`cons` (SIR v0 Twig has no string
/// builtins, so `string-length` and friends would be unresolved names
/// rather than shadowed ones).  Each entry is `(program, shadowed name)`.
const BUILTIN_SHADOWING_PROGRAMS: &[(&str, &str)] = &[
    // The issue body: a value define and a function define named `car`.
    ("(define car 5) (car (cons 1 2))", "car"),
    ("(define (car p) 42) (car (cons 1 2))", "car"),
    ("(define car (let ((k 7)) (lambda (p) k))) (car (cons 1 2))", "car"),
    // A user function named like each arithmetic / comparison / pair builtin.
    ("(define (+ a b) 99) (car (cons (+ 0 1) 2))", "+"),
    ("(define (- a b) 99) (+ (- 3 1) 1)", "-"),
    ("(define (* a b) 99) (+ (* 1 1) 1)", "*"),
    ("(define (/ a b) 99) (+ (/ 2 2) 1)", "/"),
    ("(define (< a b) 99) (+ (< 1 2) 1)", "<"),
    ("(define (> a b) 99) (+ (> 1 2) 1)", ">"),
    ("(define (= a b) 7) (+ (= 1 1) 1)", "="),
    ("(define (car p) 5) (+ (car (cons 1 2)) 1)", "car"),
    ("(define (cdr p) 5) (+ (cdr (cons 1 2)) 1)", "cdr"),
    ("(define (cons a b) 9) (+ (cons 1 2) 1)", "cons"),
    ("(define (null? x) 9) (null? nil)", "null?"),
    ("(define (pair? x) 9) (pair? nil)", "pair?"),
    ("(define (number? x) 9) (number? 1)", "number?"),
    ("(define (symbol? x) 9) (symbol? 'a)", "symbol?"),
    // A user VALUE named like a builtin, called.
    ("(define + (let ((k 100)) (lambda (a b) k))) (+ 1 2)", "+"),
    ("(define cons (let ((k 3)) (lambda (a b) k))) (cons 1 2)", "cons"),
    // A define whose expression is a bare `lambda` is a FUNCTION define.
    ("(define cons (lambda (a b) 3)) (cons 1 2)", "cons"),
    // The use comes before the definition, and inside a function body
    // and a lambda.
    ("(define (g) (car (cons 1 2))) (define (car p) 5) (g)", "car"),
    ("(define (car p) 5) (define (g) (car (cons 1 2))) (g)", "car"),
    ("(define (car p) 5) ((lambda () (car (cons 1 2))))", "car"),
    (
        "(define (g p) (car p)) (define car (let ((k 7)) (lambda (p) k))) (g (cons 1 2))",
        "car",
    ),
];

#[test]
fn a_shadowed_builtin_is_never_called_as_a_builtin() {
    for (src, name) in BUILTIN_SHADOWING_PROGRAMS {
        let m = lower(src);
        let c = calls(&m);
        assert!(!has(&c, &format!("builtin:{name}")), "{src}: {c:?}");
        // …and the call went somewhere: the user's function or value.
        assert!(
            has(&c, &format!("direct:{name}")) || has(&c, &format!("indirect:global:{name}")),
            "{src}: {c:?}"
        );
    }
}

#[test]
fn user_function_named_like_a_builtin_is_a_direct_call() {
    let m = lower("(define (car p) 42) (car (cons 1 2))");
    assert_eq!(calls_in(&m, Some("main")), ["direct:car", "builtin:cons"]);
}

#[test]
fn user_function_named_like_a_builtin_is_a_global_in_value_position() {
    // Value position already resolved rows 2/3 before row 4; pin it.
    let m = lower("(define (car p) 42) car");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    match &main.body.value {
        Expr::VarRef { name, scope, .. } => {
            assert_eq!(name, "car");
            assert_eq!(*scope, Scope::Global);
        }
        other => panic!("expected VarRef Global, got {other:?}"),
    }
}

#[test]
fn shadowing_is_per_name() {
    // Only the defined name loses its builtin: `cdr` and `cons` stay
    // builtins next to a user `car`.
    let m = lower("(define (car p) 1) (cdr (cons 1 2))");
    assert_eq!(calls_in(&m, Some("main")), ["builtin:cdr", "builtin:cons"]);
}

#[test]
fn user_function_named_print_is_called_not_written() {
    // `print` is special-cased to SIR28's `__sys_write__`; a user `print`
    // must win over that too.
    let m = lower("(define (print x) 7) (print 1)");
    let c = calls(&m);
    assert_eq!(calls_in(&m, Some("main")), ["direct:print"]);
    assert!(!has(&c, "builtin:__sys_write__"), "{c:?}");
    assert!(!m.manifest.contains(semantic_ir::Feature::ConsoleIO));
}

// ---------------------------------------------------------------------------
// Row 3 beats row 4: a top-level VALUE named like a builtin
// ---------------------------------------------------------------------------

#[test]
fn user_value_named_like_a_builtin_is_an_indirect_call_on_the_global() {
    // `(define car 5)` makes `car` read 5, and `(car p)` calls that 5 —
    // a run-time "not a closure" error, as in Scheme — rather than quietly
    // calling the builtin.
    for src in [
        "(define car 5) (car (cons 1 2))",
        "(define car (let ((k 7)) (lambda (p) k))) (car (cons 1 2))",
    ] {
        let m = lower(src);
        assert_eq!(calls_in(&m, Some("main")), ["indirect:global:car", "builtin:cons"], "{src}");
        assert!(m.manifest.contains(semantic_ir::Feature::Closures), "{src}");
    }
    let m = lower("(define + (let ((k 100)) (lambda (a b) k))) (+ 1 2)");
    assert_eq!(calls_in(&m, Some("main")), ["indirect:global:+"]);
}

#[test]
fn user_value_named_print_is_called_not_written() {
    // `(define print (lambda (x) x))` would be a *function* define here — a
    // define whose expression is a bare `lambda` becomes a `Function` — so
    // wrap the lambda in a `let` to make `print` a genuine value.
    let m = lower("(define print (let ((k 0)) (lambda (x) k))) (print 1)");
    assert_eq!(calls_in(&m, Some("main")), ["indirect:global:print"]);
    assert!(!has(&calls(&m), "builtin:__sys_write__"));
    // And the bare-lambda spelling is a function define: a direct call.
    let m = lower("(define print (lambda (x) x)) (print 1)");
    assert_eq!(calls_in(&m, Some("main")), ["direct:print"]);
}

// ---------------------------------------------------------------------------
// Row 1 beats everything: lexical bindings
// ---------------------------------------------------------------------------

#[test]
fn parameter_named_like_a_builtin_is_called() {
    let m = lower("(define (f car) (car 1))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:param:car"]);
}

#[test]
fn parameter_named_print_is_called_not_written() {
    let m = lower("(define (f print) (print 5))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:param:print"]);
    assert!(!m.manifest.contains(semantic_ir::Feature::ConsoleIO));
}

#[test]
fn parameter_named_like_a_user_function_is_called() {
    // TW00's own example: `f`'s `helper` is the parameter, not the global.
    let m = lower(
        "(define (helper x) (* x 100)) (define (f helper) (helper 1)) (f (lambda (x) (+ x 1)))",
    );
    assert_eq!(calls_in(&m, Some("f")), ["indirect:param:helper"]);
}

#[test]
fn parameter_named_like_a_user_value_is_called() {
    let m = lower("(define g (lambda (x) x)) (define (f g) (g 1))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:param:g"]);
}

#[test]
fn let_binding_named_like_a_builtin_is_called() {
    let m = lower("(define (f) (let ((car (lambda (p) 9))) (car (cons 1 2))))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:local:car", "builtin:cons"]);
}

#[test]
fn let_star_binding_named_like_a_builtin_is_called() {
    let m = lower("(define (f) (let* ((k 9) (cons (lambda (a b) k))) (cons 1 2)))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:local:cons"]);
}

#[test]
fn let_binding_named_like_a_user_function_is_called() {
    let m = lower("(define (g x) x) (define (f) (let ((g (lambda (x) 0))) (g 1)))");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:local:g"]);
}

#[test]
fn parallel_let_rhs_still_sees_the_outer_builtin() {
    // In `(let ((car …) (x (car …))) …)` the RHS of `x` is evaluated in the
    // OUTER scope, where `car` is still the builtin; only the body sees the
    // new binding.
    let m = lower(
        "(define (f) (let ((car (lambda (p) 9)) (x (car (cons 1 2)))) (+ x (car x))))",
    );
    assert_eq!(
        calls_in(&m, Some("f")),
        ["builtin:car", "builtin:cons", "builtin:+", "indirect:local:car"]
    );
}

#[test]
fn the_shadow_ends_with_its_scope() {
    // The second `(car …)` is outside the `let`, so it is the builtin again.
    let m = lower("(define (f) (+ (let ((car (lambda (p) 9))) (car 1)) (car (cons 1 2))))");
    assert_eq!(
        calls_in(&m, Some("f")),
        ["builtin:+", "indirect:local:car", "builtin:car", "builtin:cons"]
    );
}

#[test]
fn lambda_parameter_named_like_a_builtin_is_called() {
    // The arguments are lowered before the callee, so the callee lambda is
    // not necessarily `__lambda_0`: find it by its parameter.
    let m = lower("((lambda (cons) (cons 1 2)) (lambda (a b) 3))");
    let callee = m
        .functions
        .iter()
        .find(|f| f.params.iter().any(|p| p.name == "cons"))
        .expect("the lambda with a `cons` parameter");
    assert_eq!(calls_in(&m, Some(&callee.name)), ["indirect:param:cons"]);
}

#[test]
fn captured_binding_named_like_a_builtin_is_called() {
    // The lambda captures `f`'s `car` parameter; inside the lambda the call
    // goes through the capture, not the builtin.
    let m = lower("(define (f car) (lambda () (car 1)))");
    let lam = m.functions.iter().find(|f| f.name == "__lambda_0").unwrap();
    assert_eq!(lam.captures.len(), 1);
    assert_eq!(lam.captures[0].name, "car");
    assert_eq!(calls_in(&m, Some("__lambda_0")), ["indirect:capture:car"]);
}

#[test]
fn captured_binding_named_like_a_user_function_is_called() {
    let m = lower("(define (g x) x) (define (f g) (lambda () (g 1)))");
    assert_eq!(calls_in(&m, Some("__lambda_0")), ["indirect:capture:g"]);
}

#[test]
fn local_shadows_a_user_function_that_shadows_a_builtin() {
    // All three rows at once: the `let` beats the user `car`, which beats
    // the builtin `car`.
    let m = lower("(define (car p) 42) (define (f) (let ((car (lambda (p) 9))) (car 1))) (car 1)");
    assert_eq!(calls_in(&m, Some("f")), ["indirect:local:car"]);
    assert_eq!(calls_in(&m, Some("main")), ["direct:car"]);
}

// ---------------------------------------------------------------------------
// Unchanged: builtins and special forms
// ---------------------------------------------------------------------------

#[test]
fn unshadowed_builtins_are_still_builtin_calls() {
    let m = lower("(car (cons (+ 1 2) nil))");
    assert_eq!(calls_in(&m, Some("main")), ["builtin:car", "builtin:cons", "builtin:+"]);
    let m = lower("(print 1)");
    assert_eq!(calls_in(&m, Some("main")), ["builtin:__sys_write__"]);
}

#[test]
fn special_forms_stay_syntax_next_to_shadowed_builtins() {
    // `if`, `let`, `lambda`, `begin` and `quote` are parsed as syntax, so
    // they lower to their own nodes whatever the program defines.
    let m = lower(
        "(define (car p) 1) \
         (begin (if #t (let ((x 'q)) x) ((lambda (y) y) 2)) (car 3))",
    );
    assert_eq!(calls_in(&m, Some("main")), ["indirect:<expr>", "direct:car"]);
}

#[test]
fn unresolved_call_is_still_an_error() {
    let err = compile_source("(ghost 1)", "t").expect_err("ghost is unbound");
    assert!(err.message.contains("unresolved name `ghost`"), "{}", err.message);
}

// ---------------------------------------------------------------------------
// Records and unions
// ---------------------------------------------------------------------------

#[test]
fn records_named_like_builtins_are_still_rejected() {
    // TW00 row 2 also covers the constructors, predicates and accessors a
    // `record` or `union` generates.  SIR v0 Twig rejects `record` and
    // `union` outright, so an accessor named `car` cannot fall through to
    // the builtin: the declaration is an error before any call is lowered.
    let err = compile_source("(record car (x : int)) (car 1)", "t").expect_err("records rejected");
    assert!(err.message.contains("record"), "{}", err.message);
    let err = compile_source("(union T (cons (a : int))) (cons 1)", "t")
        .expect_err("unions rejected");
    assert!(err.message.contains("union"), "{}", err.message);
}
