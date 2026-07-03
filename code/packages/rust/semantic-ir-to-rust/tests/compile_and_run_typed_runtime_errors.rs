//! End-to-end proof for the **T5 typed runtime errors** in the Rust backend
//! (cascade `sir-typed-runtime-errors`).
//!
//! Ruby raises a *typed* exception for four high-frequency runtime faults, and
//! a translated `begin/rescue <Class>` must catch the matching one:
//!
//!   * `1 / 0` (and `1.0 / 0`)   → `ZeroDivisionError`
//!   * `arr.fetch(oob)`          → `IndexError`
//!   * `hash.fetch(missing)`     → `KeyError`
//!   * `obj.undefined_method`    → `NoMethodError`
//!
//! …while the LENIENT index reads still return `nil` and must NOT raise:
//!
//!   * `arr[oob]`                → `nil`
//!   * `hash[missing]`           → `nil`
//!
//! Before T5 these faults were an uncatchable host `panic!` (division) or a
//! silent `nil` floor (unknown method).  The Rust backend now surfaces each as
//! a typed `SirError` (`panic_any(SirError{ class, msg })`), so the existing
//! `catch_unwind` + `rescue_matches` machinery dispatches it to the right
//! `rescue` clause.
//!
//! Like the sibling exec-proof tests, we hand-build SIR modules, emit Rust,
//! compile with `rustc`, run the binary, and assert on the integer markers the
//! `print` path renders.  A missing `rustc`/linker is a *skip*, never a
//! failure; the host points us at a working linker via `SIR_TEST_RUSTC_LINKER`.

use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    RescueClause, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn flit(v: f64) -> Expr {
    Expr::FloatLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn call(name: &str, args: Vec<Expr>, eff: EffectSet) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: eff, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(name)];
    all.append(&mut args);
    call("__method__", all, EffectSet::PURE)
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn map_lit(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries.into_iter().map(|(key, value)| MapEntry { key, value }).collect(),
        span: s(),
    }
}

fn map_get(m: Expr, k: Expr) -> Expr {
    Expr::MapGet { map: Box::new(m), key: Box::new(k), span: s() }
}

fn print_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: call("print", vec![e], EffectSet::PURE.with(Effect::MayPrint)),
        span: s(),
    }
}

fn rescue(types: &[&str], binding: Option<&str>, body: Vec<Stmt>) -> RescueClause {
    RescueClause {
        exception_types: types.iter().map(|t| (*t).to_string()).collect(),
        binding: binding.map(|b| b.to_string()),
        body,
        span: s(),
    }
}

/// Assemble a single-`main` module from a body of statements.  `Sequences`,
/// `Maps`, and `Closures` features are always on so the collection catalog and
/// the `__method__` envelope are available.
fn module_from_main(stmts: Vec<Stmt>, extra: &[Feature]) -> Module {
    let main_fn = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };
    let mut feats = vec![
        Feature::Exceptions,
        Feature::Strings,
        Feature::Sequences,
        Feature::Maps,
        Feature::Closures,
        Feature::Floats,
    ];
    feats.extend_from_slice(extra);
    Module {
        name: "typed_rt_err_demo".into(),
        manifest: FeatureManifest::from_features(&feats),
        imports: vec![],
        exports: vec![],
        functions: vec![main_fn],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile emitted Rust and run it, returning `(stdout, exit_success)`.
/// `None` = the host has no usable linker (a skip, never a failure).
fn compile_and_run(source: &str, tag: &str) -> Option<(String, bool)> {
    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), tag);
    let src_path = dir.join(format!("sir_rterr_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_rterr_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return None;
        }
        panic!("emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{source}");
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let stdout = String::from_utf8_lossy(&run_out.stdout).to_string();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some((stdout, ok))
}

/// Wrap a single faulting expression in `begin <fault>; rescue <class> => e;
/// print(marker); end`, compile+run, and assert the process exits 0 (the
/// exception was CAUGHT) printing only the marker (the fault line never runs
/// to completion, so it prints nothing).
fn assert_typed_rescue_catches(fault: Expr, rescue_class: &str, marker: i64, tag: &str) {
    let tc = Stmt::TryCatch {
        // The fault sits inside a `print`, so if it did NOT raise we'd see its
        // (wrong) value on stdout — but it must raise before printing.
        body: vec![print_stmt(fault)],
        rescues: vec![rescue(&[rescue_class], Some("e"), vec![print_stmt(ilit(marker))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(vec![tc], &[])).expect("compile").source;
    let Some((stdout, ok)) = compile_and_run(&src, tag) else { return };
    assert!(ok, "[{tag}] process should exit 0 (exception caught); stdout={stdout:?}");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![marker.to_string()],
        "[{tag}] expected only the rescue marker {marker}; got {stdout:?}"
    );
}

/// `1 / 0` raises `ZeroDivisionError` (int path) → caught → prints `1`.
#[test]
fn int_divide_by_zero_raises_zero_division_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = call("/", vec![ilit(1), ilit(0)], EffectSet::PURE);
    assert_typed_rescue_catches(fault, "ZeroDivisionError", 1, "int_div0");
}

/// `1.0 / 0` raises `ZeroDivisionError` (float path — Ruby raises here too,
/// it does NOT hand back `inf`) → caught → prints `2`.
#[test]
fn float_divide_by_zero_raises_zero_division_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = call("/", vec![flit(1.0), ilit(0)], EffectSet::PURE);
    assert_typed_rescue_catches(fault, "ZeroDivisionError", 2, "float_div0");
}

/// `[10, 20].fetch(5)` (out of bounds) raises `IndexError` → caught → `3`.
#[test]
fn array_fetch_oob_raises_index_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = method(seq(vec![ilit(10), ilit(20)]), "fetch", vec![ilit(5)]);
    assert_typed_rescue_catches(fault, "IndexError", 3, "arr_fetch_oob");
}

/// `{"a" => 1}.fetch("z")` (missing key) raises `KeyError` → caught → `4`.
#[test]
fn hash_fetch_missing_raises_key_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = method(map_lit(vec![(slit("a"), ilit(1))]), "fetch", vec![slit("z")]);
    assert_typed_rescue_catches(fault, "KeyError", 4, "hash_fetch_miss");
}

/// `[1].undefined_method` raises `NoMethodError` → caught → `6`.
#[test]
fn unknown_method_raises_no_method_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = method(seq(vec![ilit(1)]), "undefined_method", vec![]);
    assert_typed_rescue_catches(fault, "NoMethodError", 6, "no_method");
}

/// `NoMethodError` also matches its superclass `NameError` (ancestry) → `7`.
#[test]
fn no_method_error_matches_name_error_ancestor() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = method(ilit(42), "not_a_real_method", vec![]);
    assert_typed_rescue_catches(fault, "NameError", 7, "no_method_ancestor");
}

/// The LENIENT reads do NOT raise: `arr[oob]` and `hash[miss]` both yield
/// `nil`.  Program prints `nil` twice and exits 0 — proving no over-raise.
#[test]
fn lenient_index_reads_return_nil_no_overraise() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let stmts = vec![
        // arr[oob] via the OO-surface `[]` method → nil (NOT IndexError).
        print_stmt(method(seq(vec![ilit(10), ilit(20)]), "[]", vec![ilit(9)])),
        // hash[miss] via the native `MapGet` → nil (NOT KeyError).
        print_stmt(map_get(map_lit(vec![(slit("a"), ilit(1))]), slit("z"))),
    ];
    let src = compile(&module_from_main(stmts, &[])).expect("compile").source;
    let Some((stdout, ok)) = compile_and_run(&src, "lenient_nil") else { return };
    assert!(ok, "process should exit 0 (no over-raise); stdout={stdout:?}");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["nil", "nil"],
        "arr[oob] and hash[miss] must both be nil; got {stdout:?}"
    );
}

/// `fetch` WITH a default arg does not raise: `[1].fetch(9, 42)` → `42`, and
/// `{}.fetch("k", 42)` → `42`.  Confirms we do not over-raise when Ruby would
/// return the default.
#[test]
fn fetch_with_default_does_not_raise() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let stmts = vec![
        print_stmt(method(seq(vec![ilit(1)]), "fetch", vec![ilit(9), ilit(42)])),
        print_stmt(method(map_lit(vec![(slit("a"), ilit(1))]), "fetch", vec![slit("k"), ilit(7)])),
    ];
    let src = compile(&module_from_main(stmts, &[])).expect("compile").source;
    let Some((stdout, ok)) = compile_and_run(&src, "fetch_default") else { return };
    assert!(ok, "fetch-with-default should not raise; stdout={stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["42", "7"], "got {stdout:?}");
}
