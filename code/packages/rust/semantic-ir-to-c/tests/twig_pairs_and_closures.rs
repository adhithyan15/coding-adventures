//! # Twig pairs, function values and strict closures, compiled with a real C compiler
//!
//! Twig source → `twig-to-semantic-ir` → this backend → `cc` → run, for the
//! programs from issues #18460 and #18461:
//!
//! ```text
//! program                                                  before                  now
//! (print (car (cons 1 2)))                                 refused: no `pairs`     1
//! (define (mk k) (lambda (a) (k a)))
//! (print ((mk (lambda (a) 77)) 5))                         gcc: undeclared thunk   77
//! (define (h x) (* x 3)) (define g h) (print (g 4))        "non-closure" error     12
//! (define g (begin (lambda (x y) (+ x y)))) (print (g 4))  4 (y read as nil = 0)   arity error
//! ```
//!
//! Every `semantic-ir-to-*` backend carries a copy of this table, so a
//! program means the same thing whichever backend runs it.  Without a C
//! compiler each case still lowers and emits C; only compiling and running
//! are skipped (the convention `compile_and_run.rs` documents).

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    ["cc", "clang", "gcc"]
        .into_iter()
        .map(str::to_string)
        .find(|cand| {
            Command::new(cand)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Lower and emit `src` — with **no** manifest patching, so a missing
/// feature declaration fails here — then compile and run it.  A compile
/// failure panics (that is bug #18461 item 1's symptom); `None` when no C
/// compiler is available.
fn run(src: &str, tag: &str) -> Option<Output> {
    let module = twig_to_semantic_ir::compile_source(src, "t")
        .unwrap_or_else(|e| panic!("lower `{src}`: {e}"));
    let artifact =
        semantic_ir_to_c::compile(&module).unwrap_or_else(|e| panic!("emit `{src}`: {e:?}"));
    let Some(cc) = find_cc() else {
        eprintln!("note: no C compiler — skipping execution for `{tag}`");
        return None;
    };
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_twig_pc_{tag}_{}_{n}", std::process::id());
    let dir = std::env::temp_dir();
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &artifact.source).expect("write .c");
    let built = Command::new(&cc)
        .args(["-std=c99", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
        .output()
        .expect("spawn C compiler");
    let _ = std::fs::remove_file(&cpath);
    assert!(
        built.status.success(),
        "`{src}` failed to compile:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&built.stderr),
        artifact.source
    );
    let ran = Command::new(&exe).output().expect("run emitted program");
    let _ = std::fs::remove_file(&exe);
    Some(ran)
}

/// What this backend's runtime prints when a strict closure is called with
/// the wrong number of arguments.
const ARITY_ERROR_TEXT: &str = "arity mismatch";

/// `(tag, program, expected stdout)` — programs that must run and print.
const PRINTS: &[(&str, &str, &str)] = &[
    // #18460: pair builtins pass validation and run.
    ("car", "(print (car (cons 1 2)))", "1"),
    ("cdr", "(print (cdr (cons 1 2)))", "2"),
    ("pair_yes", "(print (if (pair? (cons 1 2)) 1 0))", "1"),
    ("pair_no", "(print (if (pair? 5) 1 0))", "0"),
    // #18461 item 1: a lambda that appears only in callee position.
    (
        "callee_lambda",
        "(define (mk k) (lambda (a) (k a))) (print ((mk (lambda (a) 77)) 5))",
        "77",
    ),
    (
        "nested_callee_lambda",
        "(define (f g) ((lambda () ((lambda () (g 1)))))) (print (f (lambda (x) (+ x 41))))",
        "42",
    ),
    // #18461 item 2: a top-level function used as a value.
    (
        "fn_value",
        "(define (h x) (* x 3)) (define g h) (print (g 4))",
        "12",
    ),
    (
        "fn_argument",
        "(define (h x) (* x 3)) (define (app f v) (f v)) (print (app h 5))",
        "15",
    ),
    // #18461 item 5: a value define, read back.
    ("value_define", "(define x 5) (print x)", "5"),
];

/// `(tag, program)` — closures called with the wrong number of arguments
/// (#18461 item 4).  Each must stop with an arity error, not print.
const ARITY_ERRORS: &[(&str, &str)] = &[
    (
        "too_few",
        "(define g (begin (lambda (x y) (+ x y)))) (print (g 4))",
    ),
    ("too_many", "(print ((lambda (x) x) 1 2))"),
    (
        "fn_value_too_few",
        "(define (h x) x) (define g h) (print (g))",
    ),
];

/// Every row is checked and every failure collected before asserting, so one
/// run reports all the rows that fail — not just the first (see
/// `lessons.d/a-table-driven-test-that-stops-at-its-first-failing-case-cannot.md`).
#[test]
fn twig_programs_print_the_expected_values() {
    let mut failures = Vec::new();
    for (tag, src, want) in PRINTS {
        let Some(out) = run(src, tag) else { continue };
        let got = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            failures.push(format!("`{src}` exited non-zero:\n{stderr}"));
        } else if got.trim_end() != *want {
            failures.push(format!("`{src}` printed {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_closure_called_with_the_wrong_arity_raises() {
    let mut failures = Vec::new();
    for (tag, src) in ARITY_ERRORS {
        let Some(out) = run(src, tag) else { continue };
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() || !stderr.contains(ARITY_ERROR_TEXT) || !stdout.is_empty() {
            failures.push(format!(
                "`{src}` should stop with {ARITY_ERROR_TEXT:?}; exit {:?}, \
                 printed {stdout:?}, stderr:\n{stderr}",
                out.status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Security review of the #18461 fix (F1): a `MakeClosure` that supplies
/// fewer capture values than its body declares made this backend emit
/// `_sir_make_closure(_sir_thunk___lambda_0, 0)` — `caps` is NULL — for a
/// thunk that reads `caps[0]`.  The frontend can no longer produce one, but
/// a hand-written module can; the shared validator now refuses it before a
/// line of C is emitted.
#[test]
fn a_closure_missing_its_captures_is_refused_before_emission() {
    let mut module =
        twig_to_semantic_ir::compile_source("(define (adder n) (lambda (x) (+ x n)))", "t")
            .expect("lower");
    // Strip the capture value from every `MakeClosure` (here: adder's one).
    fn strip(e: &mut semantic_ir::Expr) {
        match e {
            semantic_ir::Expr::MakeClosure { captures, .. } => captures.clear(),
            semantic_ir::Expr::BuiltinCall { args, .. } => args.iter_mut().for_each(strip),
            _ => {}
        }
    }
    for f in &mut module.functions {
        strip(&mut f.body.value);
    }
    let err = semantic_ir_to_c::compile(&module).expect_err("must refuse");
    assert_eq!(
        err.kind,
        semantic_ir::BackendErrorKind::InvalidModule,
        "{err:?}"
    );
    assert!(err.message.contains("capture value(s)"), "{err:?}");
}
