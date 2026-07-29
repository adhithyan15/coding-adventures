//! End-to-end proof for the **E4 exception runtime** in the Rust backend.
//!
//! Rust has no native exceptions.  This backend maps Ruby's
//! `begin/rescue/ensure` onto Rust's *unwinding panic* machinery:
//!
//!   * `raise Foo, "m"` → `__sir::raise("Foo", <m>)` → `panic_any(SirError{…})`
//!   * `begin … rescue … ensure … end` → a `std::panic::catch_unwind` region
//!     that downcasts the caught payload to a `SirError` and dispatches the
//!     rescue clauses via `__sir::rescue_matches` over a built-in + user
//!     ancestry table (parity with the TS `sir-runtime-exceptions`).
//!
//! Unit tests (in `emit.rs`) assert the emitted *shape*; this test hand-builds
//! SIR modules, emits Rust, compiles with `rustc`, runs the binary, and checks
//! stdout / exit status against the behaviour the Python/TS reference produces
//! for the same SIR module.  The five cases (per spec §Verification):
//!
//!   (a) `raise ArgumentError,"x"; rescue StandardError=>e` → prints a marker
//!       (built-in ancestry: `ArgumentError < StandardError`).
//!   (b) bare `rescue` catches anything.
//!   (c) an unmatched rescue re-raises → the process exits non-zero and the
//!       inner handler does NOT print.
//!   (d) `ensure` runs on the caught path AND on the uncaught (re-raised) path.
//!   (e) user ancestry: `class MyErr < StandardError; raise MyErr; rescue
//!       StandardError` catches (edge registered at init).
//!
//! `StrLit` interpolation is not accepted by the Rust backend, so — like the
//! other Rust exec-proof tests — assertions are on simple integer markers the
//! `print` path renders.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing.  The host points the test at a working linker via
//! `SIR_TEST_RUSTC_LINKER` (e.g. the toolchain's bundled `rust-lld`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    RescueClause, Scope, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn constref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Const, span: s() }
}

fn call(name: &str, args: Vec<Expr>, eff: EffectSet) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: eff, span: s() }
}

fn print_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: call("print", vec![e], EffectSet::PURE.with(Effect::MayPrint)),
        span: s(),
    }
}

/// `raise Class, "msg"` as an expression statement.
fn raise_stmt(cls: &str, msg: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: call(
            "raise",
            vec![constref(cls), slit(msg)],
            EffectSet::PURE.with(Effect::MayThrow).with(Effect::Divergent),
        ),
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

/// Assemble a single-`main` module from a body of statements.
fn module_from_main(stmts: Vec<Stmt>, features: &[Feature]) -> Module {
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
    let mut feats = vec![Feature::Exceptions, Feature::Strings];
    feats.extend_from_slice(features);
    Module {
        name: "exc_demo".into(),
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

/// Compile emitted Rust source and run the binary, returning
/// `(stdout, exit_success)`.  Returns `None` if the host has no usable
/// linker (a skip, never a failure).
fn compile_and_run(source: &str, tag: &str) -> Option<(String, bool)> {
    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), tag);
    let src_path = dir.join(format!("sir_exc_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_exc_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{source}"
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let stdout = String::from_utf8_lossy(&run_out.stdout).to_string();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some((stdout, ok))
}

/// (a) built-in ancestry: `rescue StandardError` catches a raised
/// `ArgumentError` (`ArgumentError < StandardError`).  Prints `1`.
#[test]
fn typed_rescue_catches_via_builtin_ancestry() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let tc = Stmt::TryCatch {
        body: vec![raise_stmt("ArgumentError", "x"), print_stmt(ilit(99))], // 99 unreached
        rescues: vec![rescue(&["StandardError"], Some("e"), vec![print_stmt(ilit(1))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(vec![tc], &[Feature::Constants]))
        .expect("compile")
        .source;
    let Some((stdout, ok)) = compile_and_run(&src, "typed") else { return };
    assert!(ok, "process should exit 0 (exception caught)");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["1"], "expected only the rescue marker; got {stdout:?}");
}

/// (b) bare `rescue` (empty type list) catches anything.  Prints `2`.
#[test]
fn bare_rescue_catches_anything() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let tc = Stmt::TryCatch {
        body: vec![raise_stmt("TypeError", "boom")],
        rescues: vec![rescue(&[], None, vec![print_stmt(ilit(2))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(vec![tc], &[Feature::Constants]))
        .expect("compile")
        .source;
    let Some((stdout, ok)) = compile_and_run(&src, "bare") else { return };
    assert!(ok, "process should exit 0 (bare rescue caught)");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["2"], "got {stdout:?}");
}

/// (c) an unmatched rescue re-raises: the process exits NON-zero and the inner
/// handler does not print.  We rescue only `TypeError` but raise
/// `ArgumentError` (not a `TypeError` subclass) → propagates.
#[test]
fn unmatched_rescue_reraises_nonzero() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let tc = Stmt::TryCatch {
        body: vec![raise_stmt("ArgumentError", "x")],
        // Rescue a DIFFERENT, unrelated class → no match → re-raise.
        rescues: vec![rescue(&["TypeError"], None, vec![print_stmt(ilit(7))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(vec![tc], &[Feature::Constants]))
        .expect("compile")
        .source;
    let Some((stdout, ok)) = compile_and_run(&src, "unmatched") else { return };
    assert!(!ok, "process should exit non-zero (exception unrescued)");
    assert!(
        !stdout.contains('7'),
        "the non-matching handler must NOT run; got stdout {stdout:?}"
    );
}

/// (d) `ensure` runs on BOTH the caught path and the uncaught (re-raised)
/// path.  Two programs:
///   * caught: rescue matches → ensure prints `9`, exit 0, stdout `1\n9`.
///   * uncaught: no match → ensure prints `9` before re-raise, exit non-zero,
///     stdout contains `9`.
#[test]
fn ensure_runs_on_caught_and_uncaught() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // Caught path.
    let caught = Stmt::TryCatch {
        body: vec![raise_stmt("ArgumentError", "x")],
        rescues: vec![rescue(&["StandardError"], None, vec![print_stmt(ilit(1))])],
        ensure_body: Some(vec![print_stmt(ilit(9))]),
        span: s(),
    };
    let src = compile(&module_from_main(vec![caught], &[Feature::Constants]))
        .expect("compile")
        .source;
    if let Some((stdout, ok)) = compile_and_run(&src, "ensure_caught") {
        assert!(ok, "caught path should exit 0");
        assert_eq!(
            stdout.lines().collect::<Vec<_>>(),
            vec!["1", "9"],
            "rescue then ensure; got {stdout:?}"
        );
    } else {
        return;
    }

    // Uncaught path: ensure still runs before the re-raise.
    let uncaught = Stmt::TryCatch {
        body: vec![raise_stmt("ArgumentError", "x")],
        rescues: vec![rescue(&["TypeError"], None, vec![print_stmt(ilit(1))])],
        ensure_body: Some(vec![print_stmt(ilit(9))]),
        span: s(),
    };
    let src = compile(&module_from_main(vec![uncaught], &[Feature::Constants]))
        .expect("compile")
        .source;
    if let Some((stdout, ok)) = compile_and_run(&src, "ensure_uncaught") {
        assert!(!ok, "uncaught path should exit non-zero");
        assert!(stdout.contains('9'), "ensure must run before re-raise; got {stdout:?}");
        assert!(!stdout.contains('1'), "non-matching rescue must not run; got {stdout:?}");
    }
}

/// (e) user-defined ancestry: `class MyErr < StandardError` makes a raised
/// `MyErr` catchable by `rescue StandardError` (edge registered at init).
/// Prints `5`.
#[test]
fn user_ancestry_matches_superclass() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let classdef = Stmt::ClassDef {
        name: "MyErr".into(),
        superclass: Some("StandardError".into()),
        body: vec![],
        span: s(),
    };
    let tc = Stmt::TryCatch {
        body: vec![raise_stmt("MyErr", "custom")],
        rescues: vec![rescue(&["StandardError"], Some("e"), vec![print_stmt(ilit(5))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(vec![classdef, tc], &[Feature::Constants, Feature::Classes]))
        .expect("compile")
        .source;
    let Some((stdout, ok)) = compile_and_run(&src, "user_ancestry") else { return };
    assert!(ok, "process should exit 0 (user-ancestry match caught)");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["5"], "got {stdout:?}");
}

/// Regression (security review, F5): a rescued exception used where a String
/// is expected — `"prefix " + e` — must raise a RESCUABLE `TypeError`, not an
/// uncatchable host panic.
///
/// When `rescue => e` still bound the message STRING, `"got: " + e`
/// concatenated fine.  Making `e` a real `Value::Exception` sends it to
/// `plus`'s String arm's reject path; that path used to `panic!`, whose
/// payload is a `&str` (not a `SirError`), so `exc_from_payload`
/// `resume_unwind`s it and NO `rescue` — not even a bare one — can catch it.
/// The program died with a host backtrace instead of a Ruby `TypeError`.
///
/// This pins the fix: an OUTER bare `rescue` around `"got: " + e` catches the
/// TypeError and prints "caught", so the process exits 0.  A regression to
/// `panic!` makes the outer rescue miss and the process exit non-zero.
#[test]
fn string_plus_rescued_exception_raises_rescuable_type_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    // begin
    //   begin
    //     raise ArgumentError, "boom"
    //   rescue => e
    //     puts("got: " + e)        # e is an Exception, not a String -> TypeError
    //   end
    // rescue => _
    //   puts "caught"
    // end
    let inner_concat = print_stmt(call(
        "+",
        vec![
            slit("got: "),
            Expr::VarRef { name: "e".into(), scope: Scope::Local, span: s() },
        ],
        EffectSet::PURE,
    ));
    let inner = Stmt::TryCatch {
        body: vec![raise_stmt("ArgumentError", "boom")],
        rescues: vec![rescue(&[], Some("e"), vec![inner_concat])],
        ensure_body: None,
        span: s(),
    };
    let outer = Stmt::TryCatch {
        body: vec![inner],
        rescues: vec![rescue(&[], None, vec![print_stmt(slit("caught"))])],
        ensure_body: None,
        span: s(),
    };
    let src = compile(&module_from_main(
        vec![outer],
        &[Feature::Constants, Feature::Strings],
    ))
    .expect("compile")
    .source;
    let Some((stdout, ok)) = compile_and_run(&src, "str_plus_exc") else { return };
    assert!(ok, "TypeError must be rescuable (exit 0), not an uncatchable panic");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["caught"], "got {stdout:?}");
}
