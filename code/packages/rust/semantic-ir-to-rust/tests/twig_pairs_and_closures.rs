//! # Twig pairs, function values and strict closures, compiled with `rustc`
//!
//! Twig source → `twig-to-semantic-ir` → this backend → `rustc` → run, for
//! the programs from issues #18460 and #18461:
//!
//! ```text
//! program                                                  before                        now
//! (print (car (cons 1 2)))                                 refused: no `pairs`           1
//! (define (h x) (* x 3)) (define g h) (print (g 4))        "apply on non-closure" panic  12
//! (define g (begin (lambda (x y) (+ x y)))) (print (g 4))  4 (y padded with nil)         arity error
//! ((lambda (x) x) 1 2)                                     the extra 2 silently dropped  arity error
//! ```
//!
//! Every `semantic-ir-to-*` backend carries a copy of this table, so a
//! program means the same thing whichever backend runs it.  Without `rustc`
//! (or a usable linker) each case still lowers and emits Rust; only
//! compiling and running are skipped.

use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Lower and emit `src` — with **no** manifest patching, so a missing
/// feature declaration fails here — then build it with `rustc` and run it.
/// `None` when `rustc` or a linker is unavailable.
fn run(src: &str, tag: &str) -> Option<Output> {
    let module = twig_to_semantic_ir::compile_source(src, "t")
        .unwrap_or_else(|e| panic!("lower `{src}`: {e}"));
    let artifact =
        semantic_ir_to_rust::compile(&module).unwrap_or_else(|e| panic!("emit `{src}`: {e:?}"));
    if !rustc_available() {
        eprintln!("note: `rustc` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let stem = format!("sir_rs_twig_pc_{tag}_{}_{n}", std::process::id());
    let src_path = dir.join(format!("{stem}.rs"));
    let bin_path = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&src_path, &artifact.source).expect("write temp rust");
    let mut cmd = Command::new("rustc");
    cmd.args(["--edition", "2021", "-C", "debuginfo=0"]);
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let built = cmd
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke rustc");
    let _ = std::fs::remove_file(&src_path);
    if !built.status.success() {
        let stderr = String::from_utf8_lossy(&built.stderr);
        if stderr.contains("linker")
            && (stderr.contains("not found") || stderr.contains("No such file"))
        {
            eprintln!("note: no usable linker — skipping execution for `{tag}`");
            return None;
        }
        panic!(
            "`{src}` failed to compile:\n{stderr}\n--- source ---\n{}",
            artifact.source
        );
    }
    let out = Command::new(&bin_path)
        .output()
        .expect("run compiled binary");
    let _ = std::fs::remove_file(&bin_path);
    Some(out)
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
