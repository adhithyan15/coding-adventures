//! # User names that spell generated names, compiled with the system C compiler (#18466)
//!
//! A backend invents names of its own — a renamed entry point, runtime
//! helpers, the argument list of a closure wrapper, temporaries.  A user
//! program must not be able to spell any of them: if it can, the user's
//! name and the generated one become one variable (or one definition).
//! Each `semantic-ir-to-*` backend now escapes every name it reserves, the
//! way the C backend's `sir_esc_` marker already escaped C keywords.
//!
//! Before #18466 this backend failed these rows:
//!
//! ```text
//! program                                              before
//! (define (free x) x) (print (free 3))                 gcc: conflicting types for 'free'
//! (define (exit x) (+ x 1)) (print (exit 3))           gcc: conflicting types for 'exit'
//! (define (f EOF errno) (+ EOF errno)) (print (f 1 2)) gcc: EOF expands to (-1)
//! (define (SirValue x) x) (print (SirValue 9))         gcc: redeclares the runtime type
//! ```
//!
//! A `lambda` marker without exactly one closure (the last test) compiled
//! to a runtime `_sir_unknown_builtin("lambda")`.
//!
//! Every `semantic-ir-to-*` backend carries this table, so the names each
//! one reserves are exercised against all the others' too.  The last test
//! checks that a strict closure's arity error is an `ArgumentError` that a
//! `rescue ArgumentError` catches (and a `rescue TypeError` does not) — the
//! same class on every backend (SIR10, "Strict closures").  Without the
//! C toolchain each case still lowers and emits; only running is
//! skipped.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{Expr, Feature, Module, RescueClause, Span, Stmt};

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

fn lower(src: &str) -> Module {
    twig_to_semantic_ir::compile_source(src, "t").unwrap_or_else(|e| panic!("lower `{src}`: {e}"))
}

/// Emit `module`, compile it with `-std=c99` and run it.  A compile failure
/// comes back as the compiler's own (failed) output so the caller can report
/// it alongside every other failing row.  `None` without a C compiler.
fn run_module(module: &Module, label: &str, tag: &str) -> Option<Output> {
    let artifact =
        semantic_ir_to_c::compile(module).unwrap_or_else(|e| panic!("emit `{label}`: {e:?}"));
    let Some(cc) = find_cc() else {
        eprintln!("note: no C compiler — skipping execution for `{tag}`");
        return None;
    };
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_twig_gn_{tag}_{}_{n}", std::process::id());
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
    if !built.status.success() {
        return Some(built);
    }
    let ran = Command::new(&exe).output().expect("run emitted program");
    let _ = std::fs::remove_file(&exe);
    Some(ran)
}

/// `(tag, program, expected stdout)`.  Each row names, as a user identifier,
/// something one of the backends generates.  The table is the same in every
/// backend's copy of this file.
const PRINTS: &[(&str, &str, &str)] = &[
    // Item 1: the renamed `main` of each backend.
    (
        "main_py_go",
        "(define (_sir_user_main) 42) (print (_sir_user_main))",
        "42",
    ),
    (
        "main_rust",
        "(define (__sir_user_main) 42) (print (__sir_user_main))",
        "42",
    ),
    ("main_c", "(define (user_main) 7) (print (user_main))", "7"),
    (
        "main_ruby",
        "(define (sir_user_main) 7) (print (sir_user_main))",
        "7",
    ),
    (
        "init_c_ruby",
        "(define (sir_user_init) 8) (print (sir_user_init))",
        "8",
    ),
    // Item 2: runtime helpers, as a function and as a local.
    (
        "helper_fn",
        "(define (_sir_global_set k v) 0) (define x 5) (print x)",
        "5",
    ),
    (
        "helper_plus",
        "(define (_sir_plus a b) 0) (print (+ 2 3))",
        "5",
    ),
    (
        "helper_local",
        "(define (f _sir_plus) (+ _sir_plus 1)) (print (f 2))",
        "3",
    ),
    // Item 3: the closure wrappers' argument lists.
    // `*`, not `+`: JS `1 + [1]` is the string "11", which would hide the bug.
    (
        "wrapper_js",
        "(define (f _a) (lambda (x) (* x _a))) (print ((f 10) 2))",
        "20",
    ),
    (
        "wrapper_ruby",
        "(define (f __sir_args) (lambda (x) (* x __sir_args))) (print ((f 10) 2))",
        "20",
    ),
    (
        "wrapper_ruby_renamed",
        "(define (f sir_args) (lambda (x) (* x sir_args))) (print ((f 10) 2))",
        "20",
    ),
    // Item 4: the Rust capture temporaries.
    (
        "capture_temp",
        "(define (f A __cap_A) (lambda () (+ A __cap_A))) (print ((f 1 10)))",
        "11",
    ),
    (
        "keyword_capture",
        "(define (f fn loop) (lambda () (+ fn loop))) (print ((f 2 3)))",
        "5",
    ),
    (
        "adapter_temps",
        "(define (f __args __it __l __exc) (lambda () (+ __args __it __l __exc))) \
         (print ((f 1 2 3 4)))",
        "10",
    ),
    // Item 5: libc, and the C runtime's own type and macro names.
    ("libc_free", "(define (free x) x) (print (free 3))", "3"),
    (
        "libc_exit",
        "(define (exit x) (+ x 1)) (print (exit 3))",
        "4",
    ),
    (
        "libc_macros",
        "(define (f EOF errno) (+ EOF errno)) (print (f 1 2))",
        "3",
    ),
    (
        "c_runtime_type",
        "(define (SirValue x) x) (print (SirValue 9))",
        "9",
    ),
    // The same hazard on Go: imported packages, runtime types, `init`.
    (
        "go_package",
        "(define (fmt x) x) (define (strings x) x) (print (fmt (strings 4)))",
        "4",
    ),
    (
        "go_runtime_type",
        "(define (Value x) x) (print (Value 6))",
        "6",
    ),
    ("go_init", "(define (init x) x) (print (init 8))", "8"),
    // JS/TS: globals the runtime and the emitted code reach by name.  A
    // top-level `function Map` is hoisted over the whole script.
    (
        "js_globals",
        "(define (Map x) x) (define (Object x) x) (define (console x) x) (define (String x) x) \
         (print (Map (Object (console (String 3)))))",
        "3",
    ),
    // Python: builtins the emitted module names bare.
    (
        "py_builtins",
        "(define (len x) x) (define (list x) x) (define (range x) x) (define (Exception x) x) \
         (print (len (list (range (Exception 4)))))",
        "4",
    ),
];

/// Every row is checked and every failure collected before asserting, so one
/// run reports all the rows that fail — not just the first (see
/// `lessons.d/a-table-driven-test-that-stops-at-its-first-failing-case-cannot.md`).
#[test]
fn user_names_never_collide_with_generated_names() {
    let mut failures = Vec::new();
    for (tag, src, want) in PRINTS {
        let Some(out) = run_module(&lower(src), src, tag) else {
            continue;
        };
        let got = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            failures.push(format!("[{tag}] `{src}` failed:\n{stderr}"));
        } else if got.trim_end() != *want {
            failures.push(format!("[{tag}] `{src}` printed {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Lower `src`, then wrap `main`'s body in `begin … rescue <class>` whose
/// handler prints `99`.  Twig has no `rescue`, so the handler is spliced in
/// as SIR — the same `TryCatch` the Ruby frontend lowers `rescue` to.
fn with_rescue(src: &str, class: &str) -> Module {
    let mut m = lower(src);
    let handler_src = lower("(print 99)");
    let handler_main = handler_src
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("handler main");
    let mut handler = handler_main.body.stmts.clone();
    handler.push(Stmt::ExprStmt {
        expr: handler_main.body.value.clone(),
        span: Span::synthetic(),
    });
    let main = m
        .functions
        .iter_mut()
        .find(|f| f.name == "main")
        .expect("main");
    let mut body = std::mem::take(&mut main.body.stmts);
    body.push(Stmt::ExprStmt {
        expr: main.body.value.clone(),
        span: Span::synthetic(),
    });
    main.body.stmts = vec![Stmt::TryCatch {
        body,
        rescues: vec![RescueClause {
            exception_types: vec![class.to_string()],
            binding: None,
            body: handler,
            span: Span::synthetic(),
        }],
        ensure_body: None,
        span: Span::synthetic(),
    }];
    main.body.value = Expr::NilLit {
        span: Span::synthetic(),
    };
    m.manifest.add(Feature::Exceptions);
    m
}

/// Item 7: a strict closure's arity error is an `ArgumentError` — caught by
/// `rescue ArgumentError`, and not by `rescue TypeError`.
#[test]
fn a_closure_arity_error_is_an_argument_error() {
    const SRC: &str = "(define g (begin (lambda (x y) (+ x y)))) (print (g 4))";
    let mut failures = Vec::new();

    if let Some(out) = run_module(&with_rescue(SRC, "ArgumentError"), SRC, "rescue_arg") {
        let got = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() || got.trim_end() != "99" {
            failures.push(format!(
                "`rescue ArgumentError` should catch the arity error and print 99; \
                 exit {:?}, printed {got:?}, stderr:\n{}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    if let Some(out) = run_module(&with_rescue(SRC, "TypeError"), SRC, "rescue_type") {
        let got = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() || !got.is_empty() || !stderr.contains("arity mismatch") {
            failures.push(format!(
                "`rescue TypeError` must not catch the arity error; exit {:?}, \
                 printed {got:?}, stderr:\n{stderr}",
                out.status.code()
            ));
        }
    }

    // A user function spelled like a runtime helper must not intercept the
    // error.  On Ruby a top-level `def raise` used to replace `Kernel#raise`
    // for the runtime too, so the arity check "raised" by returning 0.
    const KERNEL: &str = "(define (raise a b) 0) (define (format a b) 0) \
                          (define g (begin (lambda (x y) (+ x y)))) (print (g 4))";
    if let Some(out) = run_module(&lower(KERNEL), KERNEL, "kernel_raise") {
        let got = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() || !got.is_empty() || !stderr.contains("arity mismatch") {
            failures.push(format!(
                "a user `raise` must not swallow the arity error; exit {:?}, \
                 printed {got:?}, stderr:\n{stderr}",
                out.status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Item 6: SIR10's `lambda` marker wraps exactly one closure.  A marker with
/// any other argument count used to compile to a runtime
/// `_sir_unknown_builtin("lambda")`; it is now refused at compile time.
/// Twig never builds one, so the marker is edited by hand.
#[test]
fn a_lambda_marker_without_exactly_one_closure_is_refused() {
    let mut failures = Vec::new();
    for count in [0usize, 2] {
        let mut m = lower("(define (f) (lambda (x) x)) (print 1)");
        let f = m.functions.iter_mut().find(|f| f.name == "f").expect("f");
        let Expr::BuiltinCall { name, args, .. } = &mut f.body.value else {
            panic!("f's body should be the lambda marker: {:?}", f.body.value);
        };
        assert_eq!(name, "lambda");
        let closure = args[0].clone();
        *args = vec![closure; count];
        match semantic_ir_to_c::compile(&m) {
            Ok(a) => failures.push(format!(
                "a `lambda` with {count} argument(s) compiled:\n{}",
                a.source
            )),
            Err(e) if !e.message.contains("lambda") => {
                failures.push(format!("refused, but not for the lambda: {e:?}"))
            }
            Err(_) => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
