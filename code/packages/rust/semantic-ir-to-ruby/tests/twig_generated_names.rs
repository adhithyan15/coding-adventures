//! # User names that spell generated names, run with `ruby` (#18466)
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
//! program                                                                before
//! (define (f __sir_args) (lambda (x) (* x __sir_args))) (print ((f 10) 2)) the wrapper's argument array
//! (define (raise a b) 0) … a wrong-arity lambda call                      no error: the user's raise
//!                                                                         replaced Kernel#raise
//! ```
//!
//! Every `semantic-ir-to-*` backend carries this table, so the names each
//! one reserves are exercised against all the others' too.  The last test
//! checks that a strict closure's arity error is an `ArgumentError` that a
//! `rescue ArgumentError` catches (and a `rescue TypeError` does not) — the
//! same class on every backend (SIR10, "Strict closures").  Without the
//! Ruby toolchain each case still lowers and emits; only running is
//! skipped.

use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{Expr, Feature, Module, RescueClause, Span, Stmt};

fn ruby_available() -> bool {
    Command::new("ruby")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn lower(src: &str) -> Module {
    twig_to_semantic_ir::compile_source(src, "t").unwrap_or_else(|e| panic!("lower `{src}`: {e}"))
}

/// Emit `module` and run it with `ruby`.  `None` without Ruby.
fn run_module(module: &Module, label: &str, tag: &str) -> Option<Output> {
    let artifact =
        semantic_ir_to_ruby::compile(module).unwrap_or_else(|e| panic!("emit `{label}`: {e:?}"));
    if !ruby_available() {
        eprintln!("note: `ruby` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "sir_rb_twig_gn_{tag}_{}_{n}.rb",
        std::process::id()
    ));
    std::fs::write(&path, &artifact.source).expect("write temp ruby");
    let out = Command::new("ruby")
        .arg(&path)
        .output()
        .expect("spawn ruby");
    let _ = std::fs::remove_file(&path);
    Some(out)
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
    // Security review of #18466: more names a user could spell.
    (
        "go_predeclared",
        "(define (any x) x) (define (min a b) a) (print (any (min 4 5)))",
        "4",
    ),
    (
        "ts_module_reserved",
        "(define (f await implements) (+ await implements)) (print (f 1 2))",
        "3",
    ),
    ("rust_wildcard", "(define (f _) _) (print (f 5))", "5"),
    (
        "py_debug",
        "(define (f __debug__) __debug__) (print (f 6))",
        "6",
    ),
    (
        "c_typedef_local",
        "(define (f int64_t size_t) (+ int64_t size_t)) (print (f 3 4))",
        "7",
    ),
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

/// The body of function `name` in the lowered `src`, as a statement list —
/// a handler to splice into a `rescue`.
fn body_stmts(src: &str, name: &str) -> Vec<Stmt> {
    let m = lower(src);
    let f = m
        .functions
        .iter()
        .find(|f| f.name == name)
        .expect("function");
    let mut stmts = f.body.stmts.clone();
    stmts.push(Stmt::ExprStmt {
        expr: f.body.value.clone(),
        span: Span::synthetic(),
    });
    stmts
}

/// Wrap function `name`'s body in `begin … rescue ArgumentError` with the
/// given handler.
fn rescue_in(m: &mut Module, name: &str, handler: Vec<Stmt>) {
    let f = m
        .functions
        .iter_mut()
        .find(|f| f.name == name)
        .expect("function");
    let mut body = std::mem::take(&mut f.body.stmts);
    body.push(Stmt::ExprStmt {
        expr: f.body.value.clone(),
        span: Span::synthetic(),
    });
    f.body.stmts = vec![Stmt::TryCatch {
        body,
        rescues: vec![RescueClause {
            exception_types: vec!["ArgumentError".to_string()],
            binding: None,
            body: handler,
            span: Span::synthetic(),
        }],
        ensure_body: None,
        span: Span::synthetic(),
    }];
    f.body.value = Expr::NilLit {
        span: Span::synthetic(),
    };
    m.manifest.add(Feature::Exceptions);
}

/// Security review of #18466 (F2): a `rescue` body reads the user's own
/// variables.  Go bound the recovered panic to a plain `r` around every
/// rescue clause, so the handler's `r` read the panic payload.  The other
/// names are C's: the emitted `rescue` calls `setjmp(…)` and uses `jmp_buf`.
#[test]
fn a_rescue_body_reads_the_users_variables() {
    let mut m = lower(
        "(define g (begin (lambda (x y) (+ x y)))) \
         (define (f r setjmp jmp_buf) (g 4)) (f 1 2 3)",
    );
    const HANDLER: &str = "(define (h r setjmp jmp_buf) (print (+ r setjmp jmp_buf)))";
    rescue_in(&mut m, "f", body_stmts(HANDLER, "h"));
    // The spliced handler brings the handler module's features with it.
    for feature in lower(HANDLER).manifest.iter() {
        m.manifest.add(feature);
    }
    let Some(out) = run_module(&m, "rescue reads r", "rescue_reads_r") else {
        return;
    };
    let got = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && got.trim_end() == "6",
        "want 6; exit {:?}, printed {got:?}, stderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Security review of #18466: a bare re-raise inside a `rescue` must raise,
/// even where the user has a variable named `raise`.  Ruby reads a bare
/// `raise` as that local, so the error vanished and the program exited 0.
#[test]
fn a_bare_reraise_is_never_a_variable_read() {
    let mut m = lower("(define g (begin (lambda (x y) (+ x y)))) (define (f raise) (g 4)) (f 7)");
    let reraise = vec![Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".to_string(),
            args: vec![],
            effects: semantic_ir::EffectSet::PURE,
            span: Span::synthetic(),
        },
        span: Span::synthetic(),
    }];
    rescue_in(&mut m, "f", reraise);
    let Some(out) = run_module(&m, "bare re-raise", "bare_reraise") else {
        return;
    };
    assert!(
        !out.status.success() && out.stdout.is_empty(),
        "the re-raise must escape; exit {:?}, printed {:?}, stderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Security review of #18466: a non-finite float literal is an expression
/// no user name can reach.  TypeScript and Python wrote Rust's `{:?}`
/// spelling (`inf`, `-inf`, `NaN`), which is not a literal in either and
/// which a user `nan`/`NaN` definition could capture.
#[test]
fn non_finite_float_literals_name_nothing() {
    let mut m = lower(
        "(define (inf) 0) (define (nan) 0) (define (NaN) 0) (define (Infinity) 0) \
         (print 1) (print 2) (print 3)",
    );
    let values = [f64::INFINITY, f64::NEG_INFINITY, f64::NAN];
    let main = m
        .functions
        .iter_mut()
        .find(|f| f.name == "main")
        .expect("main");
    let mut prints: Vec<&mut Expr> = main
        .body
        .stmts
        .iter_mut()
        .filter_map(|s| match s {
            Stmt::ExprStmt { expr, .. } => Some(expr),
            _ => None,
        })
        .collect();
    prints.push(&mut main.body.value);
    assert_eq!(prints.len(), 3, "three prints");
    for (print, v) in prints.into_iter().zip(values) {
        let Expr::BuiltinCall { args, .. } = print else {
            panic!("print: {print:?}");
        };
        *args.last_mut().expect("value") = Expr::FloatLit {
            value: v,
            span: Span::synthetic(),
        };
    }
    m.manifest.add(Feature::Floats);
    let Some(out) = run_module(&m, "non-finite floats", "non_finite") else {
        return;
    };
    let got = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    assert!(
        out.status.success() && got.contains("inf") && got.contains("nan") && !got.contains('0'),
        "want inf, -inf and nan; exit {:?}, printed {got:?}, stderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}
