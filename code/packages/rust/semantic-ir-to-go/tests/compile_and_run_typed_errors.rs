//! Execution proof for T4 (sir-typed-runtime-errors): a *faulting runtime
//! operation* must raise the CORRECT typed `SirError` so a translated
//! `rescue ZeroDivisionError`/`IndexError`/`KeyError`/`NoMethodError` catches
//! it — identically to Ruby.  We emit Go, run it with `go run`, and assert
//! the observable behaviour end-to-end (build SIR → emit → `go run`).
//!
//! Cases (all through the native Go toolchain):
//!   (a) `begin; 1/0; rescue ZeroDivisionError => e; …; end` catches;
//!   (b) `arr.fetch(oob)` raises `IndexError` (caught);
//!   (c) `h.fetch(missing)` raises `KeyError` (caught);
//!   (d) `obj.undefined` raises `NoMethodError` (caught);
//!   (e) REGRESSION: `arr[oob]`/`h[miss]` still yield `nil` (NO over-raise) —
//!       Ruby's index operators do not raise, only `.fetch` does.
//!
//! A `KeyError` is ALSO catchable by `rescue IndexError` (Ruby's `KeyError <
//! IndexError`), which case (c') checks — proving ancestry, not just an exact
//! class-name match.
//!
//! A missing `go` toolchain logs a skip rather than reddening the build
//! (mirrors `compile_and_run_exceptions.rs`).

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, MapEntry, Metadata, Module,
    RescueClause, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

/// A single-entry map literal `{ <key>: <value> }`.
fn map1(key: Expr, value: Expr) -> Expr {
    Expr::MapLit {
        entries: vec![MapEntry { key, value }],
        span: s(),
    }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(extra…)` → `BuiltinCall("__method__", [recv, "meth", …extra])`.
fn method(recv: Expr, name: &str, extra: Vec<Expr>) -> Expr {
    let mut args = vec![recv, slit(name)];
    args.extend(extra);
    builtin("__method__", args)
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: builtin("print", vec![expr]),
        span: s(),
    }
}

fn print_marker(m: &str) -> Stmt {
    print_stmt(slit(m))
}

fn expr_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt { expr, span: s() }
}

/// A `begin; <faulting>; print("should-not-print"); rescue <ty> => e;
/// print("<marker>"); end` — proves the fault is raised AND caught as `ty`.
fn rescue_of(faulting: Stmt, ty: &str, marker: &str) -> Stmt {
    Stmt::TryCatch {
        body: vec![faulting, print_marker("should-not-print")],
        rescues: vec![RescueClause {
            exception_types: vec![ty.into()],
            binding: Some("e".into()),
            body: vec![print_marker(marker)],
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    }
}

fn module_from(stmts: Vec<Stmt>) -> Module {
    let features = vec![
        Feature::Exceptions,
        Feature::Constants,
        Feature::Strings,
        Feature::Sequences,
        Feature::Maps,
        Feature::Symbols,
        Feature::Closures,
        Feature::MutableBindings,
        Feature::DynamicTyping,
    ];
    Module {
        name: "typed_err_demo".into(),
        manifest: FeatureManifest::from_features(&features),
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
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Emit `m` to Go, run it, and return `(success, stdout)`.
fn emit_and_run(m: &Module, nonce: &str) -> (bool, String) {
    let artifact = compile(m).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let src_path = dir.join(format!("sir_go_typederr_{pid}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        if stderr.contains("cannot") || stderr.contains("syntax error") || stderr.contains("undefined:") {
            let _ = std::fs::remove_file(&src_path);
            panic!(
                "emitted Go failed to COMPILE:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
                artifact.source
            );
        }
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    (ok, stdout)
}

/// (a) `1 / 0` raises `ZeroDivisionError` and is caught.
#[test]
fn divide_by_zero_is_zero_division_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let div = expr_stmt(builtin("/", vec![ilit(1), ilit(0)]));
    let tc = rescue_of(div, "ZeroDivisionError", "zde");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "a");
    assert!(ok, "rescue ZeroDivisionError should catch 1/0; stdout:\n{out}");
    assert_eq!(out.trim(), "zde", "1/0 must raise ZeroDivisionError, not print 'should-not-print'");
}

/// (a2) A generic `rescue StandardError` also catches it (ZeroDivisionError <
/// StandardError) — the divide fault is now a TYPED SirError, not a raw panic.
#[test]
fn divide_by_zero_caught_as_standard_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let div = expr_stmt(builtin("/", vec![ilit(7), ilit(0)]));
    let tc = rescue_of(div, "StandardError", "std");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "a2");
    assert!(ok, "ZeroDivisionError descends from StandardError; stdout:\n{out}");
    assert_eq!(out.trim(), "std");
}

/// (b) `[10, 20].fetch(5)` raises `IndexError` and is caught.
#[test]
fn array_fetch_oob_is_index_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let f = expr_stmt(method(seq(vec![ilit(10), ilit(20)]), "fetch", vec![ilit(5)]));
    let tc = rescue_of(f, "IndexError", "ie");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "b");
    assert!(ok, "rescue IndexError should catch arr.fetch(oob); stdout:\n{out}");
    assert_eq!(out.trim(), "ie");
}

/// (c) `{a:1}.fetch("missing")` raises `KeyError` and is caught.
#[test]
fn hash_fetch_missing_is_key_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let map = map1(slit("a"), ilit(1));
    let f = expr_stmt(method(map, "fetch", vec![slit("missing")]));
    let tc = rescue_of(f, "KeyError", "ke");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "c");
    assert!(ok, "rescue KeyError should catch h.fetch(miss); stdout:\n{out}");
    assert_eq!(out.trim(), "ke");
}

/// (c') The same `KeyError` is caught by `rescue IndexError` — proving the
/// ancestry (`KeyError < IndexError`), not just an exact-name match.
#[test]
fn hash_fetch_missing_caught_as_index_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let map = map1(slit("a"), ilit(1));
    let f = expr_stmt(method(map, "fetch", vec![slit("missing")]));
    let tc = rescue_of(f, "IndexError", "ie-from-key");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "cp");
    assert!(ok, "KeyError descends from IndexError; stdout:\n{out}");
    assert_eq!(out.trim(), "ie-from-key");
}

/// (d) An unknown method (`5.bogus`) raises `NoMethodError` and is caught.
#[test]
fn unknown_method_is_no_method_error() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let call = expr_stmt(method(ilit(5), "bogus", vec![]));
    let tc = rescue_of(call, "NoMethodError", "nme");
    let (ok, out) = emit_and_run(&module_from(vec![tc]), "d");
    assert!(ok, "rescue NoMethodError should catch obj.undefined; stdout:\n{out}");
    assert_eq!(out.trim(), "nme");
}

/// (e) REGRESSION: the hash index OPERATOR `h[missing]` (MapGet) still yields
/// `nil` — only `.fetch` raises.  Proves the typed-error work did not turn a
/// benign missing-key read into a KeyError (no over-raise on the `[]` path).
#[test]
fn hash_index_operator_still_returns_nil() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // h["missing"] — MapGet on a missing key → nil (unchanged).
    let map = map1(slit("a"), ilit(1));
    let map_get = print_stmt(Expr::MapGet {
        map: Box::new(map),
        key: Box::new(slit("missing")),
        span: s(),
    });
    let (ok, out) = emit_and_run(&module_from(vec![map_get]), "e");
    assert!(ok, "h[missing] must not raise; stdout:\n{out}");
    assert_eq!(out.trim(), "nil", "h[miss] must be nil (no over-raise)");
}

/// (e2) REGRESSION: `.fetch` with a DEFAULT does NOT raise — an out-of-bounds
/// `arr.fetch(oob, d)` returns `d`, and a missing `h.fetch(k, d)` returns `d`.
/// This proves the typed IndexError/KeyError only fire when NO default is
/// given (Ruby's exact `.fetch` contract — no over-raise).
#[test]
fn fetch_with_default_does_not_raise() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // [1, 2].fetch(9, 42) → 42 (no IndexError).
    let arr_default = print_stmt(method(
        seq(vec![ilit(1), ilit(2)]),
        "fetch",
        vec![ilit(9), ilit(42)],
    ));
    // {a:1}.fetch("missing", 7) → 7 (no KeyError).
    let map = map1(slit("a"), ilit(1));
    let hash_default = print_stmt(method(map, "fetch", vec![slit("missing"), ilit(7)]));
    // [1, 2].fetch(0) → 1 (in-bounds, no default needed, no raise).
    let arr_valid = print_stmt(method(seq(vec![ilit(1), ilit(2)]), "fetch", vec![ilit(0)]));
    let (ok, out) = emit_and_run(&module_from(vec![arr_default, hash_default, arr_valid]), "e2");
    assert!(ok, "fetch-with-default and in-bounds fetch must not raise; stdout:\n{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["42", "7", "1"], "fetch defaults / valid index must not over-raise");
}
