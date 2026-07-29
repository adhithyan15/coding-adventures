//! Execution proof for `Stmt::ForRange` on the C backend — hand-build a module
//! (bypassing any frontend, since ForRange is producer-agnostic and gated by
//! `Feature::Loops` alone), emit C, compile with a real gcc/clang-style
//! compiler, run it, assert stdout. Skips gracefully when no `cc` is present.
//!
//! ForRange was a PRE-EXISTING `unreachable!` panic: C accepts `Loops` but the
//! emitter did not lower ForRange. These prove it now runs and matches the
//! Go/Rust `_sir_range_cont` semantics (direction-aware exclusive stop).

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span,
    Stmt, CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        let ok = Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_run(cc: &str, module: &Module) -> String {
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_fr_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(cc)
        .args(["-std=c99", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .output()
        .expect("spawn C compiler");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let run = Command::new(&exe).output().expect("run emitted program");
    assert!(run.status.success(), "run failed (exit {:?})", run.status.code());
    String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
}

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args: vec![arg],
            effects: EffectSet::PURE,
            span: s(),
        },
        span: s(),
    }
}
fn forrange(var: &str, start: i64, stop: i64, step: i64, body: Vec<Stmt>) -> Stmt {
    Stmt::ForRange {
        var: var.into(),
        start: ilit(start),
        stop: ilit(stop),
        step: ilit(step),
        body: Block { stmts: body, value: Expr::NilLit { span: s() }, span: s() },
        span: s(),
    }
}
fn loops_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "frprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::Loops]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

#[test]
fn for_range_counts_up_exclusive() {
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler");
        return;
    };
    // `for i in 0, 3, 1: puts i` → 0,1,2 (stop is EXCLUSIVE).
    let out = compile_and_run(&cc, &loops_module(vec![forrange("i", 0, 3, 1, vec![puts(local("i"))])]));
    assert_eq!(out, "0\n1\n2\n");
}

#[test]
fn for_range_counts_down_with_negative_step() {
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler");
        return;
    };
    // `for i in 3, 0, -1: puts i` → 3,2,1 (descending, exclusive of 0).
    let out = compile_and_run(&cc, &loops_module(vec![forrange("i", 3, 0, -1, vec![puts(local("i"))])]));
    assert_eq!(out, "3\n2\n1\n");
}

#[test]
fn for_range_nested_distinct_counters() {
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler");
        return;
    };
    // Nested loops must not clobber each other's counter temporaries.
    let inner = forrange("j", 0, 2, 1, vec![puts(local("j"))]);
    let out = compile_and_run(&cc, &loops_module(vec![forrange("i", 0, 2, 1, vec![inner])]));
    assert_eq!(out, "0\n1\n0\n1\n"); // i=0:{j0,j1}, i=1:{j0,j1}
}

#[test]
fn for_range_var_is_block_scoped() {
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler");
        return;
    };
    // A loop var declared inside the loop body block does not clobber an
    // enclosing same-named local — `x = 99; for x in 0..3 {}; puts x` → 99.
    let out = compile_and_run(
        &cc,
        &loops_module(vec![
            Stmt::LetBinding { name: "x".into(), sir_type: None, value: ilit(99), span: s() },
            forrange("x", 0, 3, 1, vec![]),
            puts(local("x")),
        ]),
    );
    assert_eq!(out, "99\n");
}

#[test]
fn unsupported_builtin_in_while_body_is_rejected_not_panicked() {
    // Gap B: a `While` body was not scanned by the unsupported-builtin
    // pre-check, so an unknown builtin there hit the emitter's `unreachable!`.
    // It must now reject CLEANLY (a `BackendError`), never panic.
    let bad = Expr::BuiltinCall {
        name: "totally_unsupported_xyz".into(),
        args: vec![],
        effects: EffectSet::PURE,
        span: s(),
    };
    let while_stmt = Stmt::While {
        cond: Expr::BoolLit { value: false, span: s() },
        body: Block {
            stmts: vec![Stmt::ExprStmt { expr: bad, span: s() }],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        span: s(),
    };
    let result = semantic_ir_to_c::compile(&loops_module(vec![while_stmt]));
    assert!(result.is_err(), "unsupported builtin in a while body must reject cleanly, not panic");
}

#[test]
fn for_range_with_unused_counter_compiles_under_werror() {
    // Regression: an empty-body / unused-counter loop must not emit an unused
    // variable — a `-Werror` consumer would break. Compile a for-range whose
    // body never reads the counter, with `-Wall -Werror`.
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler");
        return;
    };
    let module = loops_module(vec![forrange("i", 0, 3, 1, vec![puts(ilit(7))])]); // body ignores `i`
    let artifact = semantic_ir_to_c::compile(&module).expect("compile");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_frwe_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    // `-Werror` only for the unused-variable class, to avoid failing on any
    // unrelated pre-existing runtime warning under a strict compiler.
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "unused-counter loop must compile under -Werror=unused-variable:\n{}",
        String::from_utf8_lossy(&out.stderr),
    );
}
