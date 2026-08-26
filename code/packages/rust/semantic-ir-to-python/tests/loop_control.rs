//! SIR16 addendum execution proof: bare loop-control statements
//! (`Stmt::Break`/`Stmt::Continue`, `Feature::LoopControl`) on the Python
//! backend, task #63 — hand-builds a module using each construct, emits
//! Python, runs it with a real `python3`/`python` interpreter (with
//! `coding-adventures-sir-runtime-core` on `PYTHONPATH`), and asserts
//! stdout. Mirrors `semantic-ir-to-javascript`'s (task #62) and
//! `semantic-ir-to-go`'s (task #63) own identically-shaped execution
//! proofs; skips (does not fail) when no usable Python interpreter is on
//! `PATH`. Harness mirrors this crate's own `tests/sir22_array.rs`.
//!
//! Each `if` used as a bare statement to hold a `break`/`continue` also
//! exercises the `Stmt::ExprStmt`/`Expr::If` special case `emit_stmt_inner`
//! gained alongside `Feature::LoopControl`: without it, this program would
//! route through the value-position ternary/walrus-tuple codegen, which
//! has no way to represent a Python `break`/`continue` at all (they are
//! statements, not expressions) — see that arm's own doc comment in
//! `src/emit.rs`.

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
};

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn let_binding(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}
fn assign(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Local, value, span: s() }
}
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
                expr,
            ],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// `if (cond) { then_stmts } else {}` as a bare statement — the exact
/// shape whose value-position codegen used to hit `emit_block_as_expr`'s
/// own `Stmt::Break`/`Continue` panic before `emit_stmt_inner`'s own
/// `Stmt::ExprStmt`/`Expr::If` special case fixed it.
fn if_stmt(cond: Expr, then_stmts: Vec<Stmt>) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(Block {
                stmts: then_stmts,
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            else_branch: Box::new(Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() }),
            span: s(),
        },
        span: s(),
    }
}

fn loop_control_module(name: &str, main_stmts: Vec<Stmt>) -> Module {
    Module {
        name: name.into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::MutableBindings,
            Feature::Loops,
            Feature::LoopControl,
            Feature::Sequences,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Probe whether `exe` is a usable Python interpreter — see
/// `tests/sir22_array.rs`'s own identically-named helper for why this
/// distinguishes a genuinely-absent interpreter from a Windows Store stub.
fn python_is_runnable(exe: &str) -> bool {
    std::process::Command::new(exe)
        .args(["-c", "pass"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a hand-built module through a real Python interpreter, returning
/// stdout. `None` when no usable interpreter is on `PATH`.
fn run_via_python(m: &Module, tag: &str) -> Option<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let exe = ["python3", "python"].into_iter().find(|e| python_is_runnable(e))?;

    let source = semantic_ir_to_python::compile(m).expect("python emit").source;

    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let pythonpath = std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
    ])
    .expect("join PYTHONPATH");

    let nonce = SEQ.fetch_add(1, Ordering::Relaxed);
    let file =
        std::env::temp_dir().join(format!("sir_py_lc_{tag}_{}_{}.py", std::process::id(), nonce));
    std::fs::write(&file, &source).expect("write temp python");
    let out = std::process::Command::new(exe)
        .arg(&file)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python");
    let _ = std::fs::remove_file(&file);

    assert!(
        out.status.success(),
        "emitted python failed under {exe} for `{tag}`:\n{}\n--- source ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n").trim_end().to_string())
}

#[test]
fn while_loop_continue_skips_five_and_break_stops_past_seven() {
    // n = 0; sum = 0
    // while _sir_truthy(n < 10):
    //     n = n + 1
    //     if _sir_truthy(n = 5):
    //         continue
    //     if _sir_truthy(n > 7):
    //         break
    //     sum = sum + n
    // print(sum)  -> 1+2+3+4 (skip 5) +6+7 = 23, then break at n=8
    let body = Block {
        stmts: vec![
            assign("n", call("+", vec![local("n"), ilit(1)])),
            if_stmt(call("=", vec![local("n"), ilit(5)]), vec![Stmt::Continue { span: s() }]),
            if_stmt(call(">", vec![local("n"), ilit(7)]), vec![Stmt::Break { span: s() }]),
            assign("sum", call("+", vec![local("sum"), local("n")])),
        ],
        value: Expr::NilLit { span: s() },
        span: s(),
    };
    let main_stmts = vec![
        let_binding("n", ilit(0)),
        let_binding("sum", ilit(0)),
        Stmt::While { cond: call("<", vec![local("n"), ilit(10)]), body, span: s() },
        print_stmt(local("sum")),
    ];
    let module = loop_control_module("loop_control_while", main_stmts);
    if let Some(stdout) = run_via_python(&module, "loop_control_while") {
        assert_eq!(stdout, "23");
    } else {
        eprintln!("skipping while_loop_continue_skips_five_and_break_stops_past_seven: no usable python interpreter");
    }
}

#[test]
fn for_each_loop_break_stops_iteration_before_the_matching_element() {
    // sum = 0
    // for x in [1, 2, 3, 4, 5]:
    //     if _sir_truthy(x = 3):
    //         break
    //     sum = sum + x
    // print(sum)  -> 1 + 2 = 3 (the loop never adds 3, 4, or 5)
    let body = Block {
        stmts: vec![
            if_stmt(call("=", vec![local("x"), ilit(3)]), vec![Stmt::Break { span: s() }]),
            assign("sum", call("+", vec![local("sum"), local("x")])),
        ],
        value: Expr::NilLit { span: s() },
        span: s(),
    };
    let main_stmts = vec![
        let_binding("sum", ilit(0)),
        Stmt::ForEach {
            var: "x".into(),
            iter: Expr::SeqLit {
                items: vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)],
                span: s(),
            },
            body,
            span: s(),
        },
        print_stmt(local("sum")),
    ];
    let module = loop_control_module("loop_control_foreach", main_stmts);
    if let Some(stdout) = run_via_python(&module, "loop_control_foreach") {
        assert_eq!(stdout, "3");
    } else {
        eprintln!("skipping for_each_loop_break_stops_iteration_before_the_matching_element: no usable python interpreter");
    }
}
