//! # Twig name resolution, run end to end (issue #18456)
//!
//! Twig source → `twig-to-semantic-ir` → this backend → `node`.
//!
//! TW00 resolves every name in one order, called or not: lexical
//! bindings, then the program's top-level definitions, then builtins.
//! `twig-to-semantic-ir` used to check the builtin table first in call
//! position, so the programs below printed the *builtin's* answer:
//!
//! ```text
//! (define (car p) 42) (print (car (cons 1 2)))      before: 1    after: 42
//! (define (helper x) (* x 100))
//! (define (f helper) (helper 1))
//! (print (f (lambda (x) (+ x 1))))                    before: 100  after: 2
//! ```
//!
//! The frontend's own `tests/name_resolution.rs` pins the lowered IR
//! shape; this file proves the shape *runs* to the right answer.  Node is
//! optional: without it each test still lowers and compiles the program,
//! and skips only the execution check.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lower and compile `src`, run it under `node`, and return stdout with
/// trailing newlines trimmed — or `None` when Node is unavailable.
///
/// The module is given `Feature::Pairs` before compiling.  SIR11 says the
/// frontend should declare it whenever it emits a `cons` / `car` / `cdr` /
/// `pair?` builtin call, but it does not yet, and the validator then
/// rejects the module.  That gap predates #18456 and needs its own
/// fix; declaring the feature here keeps these tests about name
/// resolution.  Declaring a feature a module does not use is harmless.
fn run(src: &str, tag: &str) -> Option<String> {
    let mut module = twig_to_semantic_ir::compile_source(src, "shadow")
        .unwrap_or_else(|e| panic!("lower `{src}`: {e}"));
    module.manifest.add(semantic_ir::Feature::Pairs);
    let artifact = compile(&module).unwrap_or_else(|e| panic!("compile `{src}`: {e:?}"));
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_twig_shadow_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{src}`:\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

/// `(program, tag, expected stdout)`.  The builtin's answer is noted where
/// it differs, so a regression reads off the failure message directly.
const CASES: &[(&str, &str, &str)] = &[
    // Row 2 beats row 4: a top-level function named like a builtin.
    ("(define (car p) 42) (print (car (cons 1 2)))", "fn_car", "42"), // builtin: 1
    ("(define (cons a b) 9) (print (+ (cons 1 2) 1))", "fn_cons", "10"),
    ("(define (+ a b) 99) (print (+ 1 2))", "fn_plus", "99"), // builtin: 3
    ("(define (= a b) 7) (print (+ (= 1 1) 1))", "fn_eq", "8"),
    // The use comes before the definition, and inside a lambda.
    ("(define (g) (car (cons 1 2))) (define (car p) 5) (print (g))", "fn_before", "5"),
    ("(define (car p) 5) (print ((lambda () (car (cons 1 2)))))", "fn_lambda", "5"),
    // Only the defined name is shadowed.
    ("(define (car p) 42) (print (cdr (cons 1 2)))", "fn_per_name", "2"),
    // Row 3 beats row 4: a top-level value named like a builtin, called.
    ("(define car (let ((k 7)) (lambda (p) k))) (print (car (cons 1 2)))", "val_car", "7"),
    ("(define + (let ((k 100)) (lambda (a b) k))) (print (+ 1 2))", "val_plus", "100"),
    (
        "(define (g p) (car p)) (define car (let ((k 7)) (lambda (p) k))) (print (g (cons 1 2)))",
        "val_before",
        "7",
    ),
    // Row 1 beats rows 2-4: parameters, let / let* / lambda bindings, captures.
    ("(define (f car) (car 1)) (print (f (lambda (x) (+ x 41))))", "param_car", "42"),
    (
        "(define (helper x) (* x 100)) (define (f helper) (helper 1)) \
         (print (f (lambda (x) (+ x 1))))",
        "param_fn", // TW00's own example; the global gives 100
        "2",
    ),
    (
        "(define (show x) (print x)) (define (f print) (print 5)) \
         (show (f (lambda (x) (+ x 1))))",
        "param_print",
        "6",
    ),
    ("(define (f) (let ((car (lambda (p) 9))) (car (cons 1 2)))) (print (f))", "let_car", "9"),
    (
        "(define (f) (let* ((k 9) (cons (lambda (a b) k))) (cons 1 2))) (print (f))",
        "letstar_cons",
        "9",
    ),
    ("(print ((lambda (cons) (cons 1 2)) (lambda (a b) 3)))", "lambda_param", "3"),
    (
        "(define (f car) (lambda () (car 1))) (print ((f (lambda (x) (+ x 10)))))",
        "capture_car",
        "11",
    ),
    // A parallel `let`'s right-hand sides see the OUTER `car` (the
    // builtin); its body sees the new one.  x = 1, (car x) = 9.
    (
        "(define (f) (let ((car (lambda (p) 9)) (x (car (cons 1 2)))) (+ x (car x)))) (print (f))",
        "parallel_let",
        "10",
    ),
    // All three rows: the let beats the user `car`, which beats the builtin.
    (
        "(define (car p) 42) (define (f) (let ((car (lambda (p) 9))) (car 1))) \
         (print (+ (f) (car 1)))",
        "three_rows",
        "51",
    ),
];

#[test]
fn shadowing_programs_print_the_shadowing_definitions_answer() {
    for (src, tag, want) in CASES {
        if let Some(got) = run(src, tag) {
            assert_eq!(got, *want, "`{src}`");
        }
    }
}
