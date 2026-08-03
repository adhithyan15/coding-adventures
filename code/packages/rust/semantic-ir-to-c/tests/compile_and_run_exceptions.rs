//! Execution proof for SIR17 exceptions on the C backend — hand-build modules
//! (producer-agnostic), emit C, compile with a real gcc/clang-style compiler,
//! run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! C has no unwinding, so `begin … rescue … ensure … end` lowers to a
//! `setjmp`/`longjmp` handler stack with a baked-in exception-class ancestry
//! table. These tests drive the whole path through a real `cc`: catching a
//! raised message, class matching via the ancestry (`rescue StandardError`
//! catches a `RuntimeError`), the rescue binding, `ensure` on the normal AND the
//! exception path, and propagation of an unmatched exception through an outer
//! handler (with the inner `ensure` still running).

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, RescueClause,
    Span, Stmt, CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Compile + run; returns `(stdout, exit_success)`. Does NOT assert success (an
/// uncaught exception exits non-zero on purpose).
fn run_raw(module: &Module) -> Option<(String, bool)> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_exc_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let r = Command::new(&exe).output().expect("run");
    Some((
        String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"),
        r.status.success(),
    ))
}

/// Compile + run, asserting the program exits 0 (the common case).
fn run(module: &Module) -> Option<String> {
    run_raw(module).map(|(out, ok)| {
        assert!(ok, "program exited non-zero; stdout:\n{out}");
        out
    })
}

fn s() -> Span {
    Span::synthetic()
}
fn strlit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
fn raise_(arg: Option<Expr>) -> Stmt {
    Stmt::ExprStmt { expr: bc("raise", arg.into_iter().collect()), span: s() }
}
fn const_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: semantic_ir::Scope::Const, span: s() }
}
/// `raise Class, msg` (or a bare `raise Class` when `msg` is `None`): the first
/// argument is a `Const` CLASS NAME — the SIR shape the frontend emits for
/// `raise ArgumentError, "boom"`.
fn raise_class(class: &str, msg: Option<Expr>) -> Stmt {
    let mut args = vec![const_ref(class)];
    args.extend(msg);
    Stmt::ExprStmt { expr: bc("raise", args), span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: semantic_ir::Scope::Local, span: s() }
}
fn rescue_clause(types: Vec<&str>, binding: Option<&str>, body: Vec<Stmt>) -> RescueClause {
    RescueClause {
        exception_types: types.into_iter().map(String::from).collect(),
        binding: binding.map(String::from),
        body,
        span: s(),
    }
}
fn trycatch(body: Vec<Stmt>, rescues: Vec<RescueClause>, ensure_body: Option<Vec<Stmt>>) -> Stmt {
    Stmt::TryCatch { body, rescues, ensure_body, span: s() }
}
/// A `main` module running `stmts`, declaring Exceptions + Strings.
fn exc_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "excprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::Exceptions, Feature::Strings]),
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

/// A `main` module running `stmts`, declaring Exceptions + Strings + Classes +
/// Constants + ShortCircuit — the extra features a `raise Class` (a `Const`
/// class reference, observing `Feature::Constants`/`Classes`) and a compound
/// `and`-message need beyond the base [`exc_module`].
fn exc_class_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "excprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Exceptions,
            Feature::Strings,
            Feature::Classes,
            Feature::Constants,
            Feature::ShortCircuit,
        ]),
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
fn raise_named_class_with_message_is_caught_and_prints_the_message() {
    // Regression (the `puts(e)` conformance failure): `begin; raise ArgumentError,
    // "boom"; rescue ArgumentError => e; puts(e); end` must construct an
    // ArgumentError — NOT `_sir_const_get("ArgumentError")`, which the C runtime
    // has no builtin exception-class constant for (it raised `NameError:
    // uninitialized constant ArgumentError`).  The rescue matches via the
    // ancestry and `puts(e)` prints the message.
    match run(&exc_class_module(vec![trycatch(
        vec![raise_class("ArgumentError", Some(strlit("boom")))],
        vec![rescue_clause(vec!["ArgumentError"], Some("e"), vec![puts(local("e"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "boom\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn raise_bare_named_class_defaults_its_message_to_the_class_name() {
    // A bare `raise TypeError` carries no message, so `#message` (and `puts e`)
    // defaults to the class name.  `rescue TypeError => e; puts(e)` → "TypeError".
    match run(&exc_class_module(vec![trycatch(
        vec![raise_class("TypeError", None)],
        vec![rescue_clause(vec!["TypeError"], Some("e"), vec![puts(local("e"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "TypeError\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn raise_named_class_with_a_compound_message_still_constructs_the_exception() {
    // The message is a short-circuit `and` (non-simple), so the raise takes the
    // COMPOUND emit path.  The `Const` class must STILL build an exception there,
    // not a `_sir_const_get`: `raise ArgumentError, ("a" && "b")` raises with the
    // deciding operand "b" as its message, caught and printed.
    match run(&exc_class_module(vec![trycatch(
        vec![raise_class(
            "ArgumentError",
            Some(bc("and", vec![strlit("a"), strlit("b")])),
        )],
        vec![rescue_clause(vec!["ArgumentError"], Some("e"), vec![puts(local("e"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "b\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn bare_rescue_catches_a_raised_message() {
    // `begin; raise "boom"; rescue; puts "caught"; end` → the `RuntimeError` is
    // caught by the bare rescue; the program exits 0.
    match run(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("boom")))],
        vec![rescue_clause(vec![], None, vec![puts(strlit("caught"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "caught\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn rescue_standard_class_matches_a_runtime_error() {
    // `rescue StandardError` catches a `RuntimeError` via the ancestry table.
    match run(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec!["StandardError"], None, vec![puts(strlit("std"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "std\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn rescue_binds_the_caught_exception() {
    // `begin; raise "boom"; rescue => e; puts(e); end` — `e` prints as its
    // message (`_sir_fmt` of a `SIR_ERROR` shows the message).
    match run(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("boom")))],
        vec![rescue_clause(vec![], Some("e"), vec![puts(local("e"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "boom\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ensure_runs_on_both_the_normal_and_the_exception_path() {
    // Normal path: `begin; puts "body"; ensure; puts "cleanup"; end`.
    let normal = run(&exc_module(vec![trycatch(
        vec![puts(strlit("body"))],
        vec![],
        Some(vec![puts(strlit("cleanup"))]),
    )]));
    if let Some(out) = normal {
        assert_eq!(out, "body\ncleanup\n");
    }
    // Exception path: the body raises, a rescue catches, ensure STILL runs.
    let caught = run(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec![], None, vec![puts(strlit("caught"))])],
        Some(vec![puts(strlit("cleanup"))]),
    )]));
    match caught {
        Some(out) => assert_eq!(out, "caught\ncleanup\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn unmatched_exception_propagates_after_inner_ensure_runs() {
    // Inner `rescue TypeError` does NOT match a `RuntimeError`, so it propagates
    // — but the inner `ensure` runs first — and the OUTER bare rescue catches it.
    // `begin; begin; raise "x"; rescue TypeError; puts "inner"; ensure; puts
    // "cleanup"; end; rescue; puts "outer"; end` → `cleanup`, then `outer`.
    let inner = trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec!["TypeError"], None, vec![puts(strlit("inner"))])],
        Some(vec![puts(strlit("cleanup"))]),
    );
    match run(&exc_module(vec![trycatch(
        vec![inner],
        vec![rescue_clause(vec![], None, vec![puts(strlit("outer"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "cleanup\nouter\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn uncaught_exception_exits_non_zero() {
    // A `raise` with no enclosing handler is uncaught → the program exits
    // non-zero (Ruby's default). Nothing is printed to stdout.
    match run_raw(&exc_module(vec![raise_(Some(strlit("boom")))])) {
        Some((out, ok)) => {
            assert!(!ok, "an uncaught exception must exit non-zero");
            assert_eq!(out, "", "nothing on stdout for an uncaught exception");
        }
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ensure_that_handles_a_nested_exception_does_not_change_what_propagates() {
    // Regression (security review): the exception that ESCAPES an inner
    // begin/rescue must be snapshotted before `ensure` runs — an `ensure` body
    // that itself raises-and-rescues would otherwise overwrite the global
    // current-error and propagate the WRONG exception. The outer rescue must see
    // "original", not the ensure's "swallowed".
    let nested_in_ensure = trycatch(
        vec![raise_(Some(strlit("swallowed")))],
        vec![rescue_clause(vec![], None, vec![puts(strlit("ensure-handled"))])],
        None,
    );
    let inner = trycatch(
        vec![raise_(Some(strlit("original")))],
        vec![rescue_clause(vec!["TypeError"], None, vec![puts(strlit("wrong"))])],
        Some(vec![nested_in_ensure]),
    );
    match run(&exc_module(vec![trycatch(
        vec![inner],
        vec![rescue_clause(vec![], Some("e"), vec![puts(local("e"))])],
        None,
    )])) {
        Some(out) => assert_eq!(out, "ensure-handled\noriginal\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn exception_emits_the_setjmp_handler_shape() {
    // Emit-shape: the setjmp handler stack + ancestry-based matching are present.
    let src = semantic_ir_to_c::compile(&exc_module(vec![trycatch(
        vec![raise_(Some(strlit("x")))],
        vec![rescue_clause(vec!["StandardError"], Some("e"), vec![puts(local("e"))])],
        Some(vec![puts(strlit("done"))]),
    )]))
    .expect("compile")
    .source;
    assert!(src.contains("#include <setjmp.h>"), "setjmp header:\n{src}");
    assert!(src.contains("_sir_push_handler()"), "handler push:\n{src}");
    assert!(src.contains("_sir_rescue_matches("), "rescue matching:\n{src}");
    assert!(src.contains("\"StandardError\""), "quoted rescue type (no injection):\n{src}");
}
