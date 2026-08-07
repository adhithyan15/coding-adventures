//! End-to-end proof for the **Numeric (`Integer`/`Float`) method dispatch**
//! runtime in the Rust backend.
//!
//! Method calls reach every backend as the narrow-waist envelope
//! `BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`.  This
//! backend emits `__sir::call_method(recv, "meth", vec![…])` into an inline
//! `__sir` runtime whose `numeric_method` implements the Ruby
//! `Integer`/`Float` catalog by an EXPLICIT `name` match — never reflection
//! ([[dynamic-dispatch-rce]]) — ported from the Python/TypeScript
//! `sir-runtime-oop` reference for behavioural parity.
//!
//! This test hand-builds a SIR module that exercises the freshly added
//! predicate / conversion / arithmetic methods (`even?`/`odd?`/`positive?`/
//! `negative?`/`abs`/`succ`/`pred`/`floor`/`ceil`/`round`/`gcd`/`pow`/`**`/
//! `digits`), emits Rust, compiles it with `rustc`, runs the binary, and
//! diffs stdout against the values the Python/TS reference produces for the
//! SAME SIR module.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip
//! rather than failing; a missing host tool must never redden a build.  The
//! host can point the test at a working linker via `SIR_TEST_RUSTC_LINKER`
//! (e.g. the toolchain's bundled `rust-lld`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
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

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `-n` — unary minus via the variadic `minus` builtin (single operand).
fn neg(inner: Expr) -> Expr {
    call("-", vec![inner])
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, Expr::StrLit { value: name.into(), span: s() }];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

// The 3.14 / 3.14159 float literals are test inputs exercising `to_i`/`round`
// numeric methods, not approximations of PI to be swapped for a constant.
#[allow(clippy::approx_constant)]
fn numeric_demo() -> Module {
    let main_stmts = vec![
        // predicates ---------------------------------------------------
        print_stmt(method(ilit(4), "even?", vec![])), // true
        print_stmt(method(ilit(3), "odd?", vec![])),  // true
        print_stmt(method(ilit(0), "zero?", vec![])), // true
        print_stmt(method(ilit(5), "positive?", vec![])), // true
        print_stmt(method(neg(ilit(5)), "negative?", vec![])), // true
        // conversions --------------------------------------------------
        print_stmt(method(ilit(7), "to_s", vec![])), // "7"
        print_stmt(method(flit(3.14), "to_i", vec![])), // 3
        print_stmt(method(ilit(5), "to_f", vec![])), // 5.0
        // magnitude / neighbours --------------------------------------
        print_stmt(method(neg(ilit(5)), "abs", vec![])), // 5
        print_stmt(method(ilit(5), "succ", vec![])),     // 6
        print_stmt(method(ilit(5), "pred", vec![])),     // 4
        // float roundings ---------------------------------------------
        print_stmt(method(flit(3.7), "floor", vec![])), // 3
        print_stmt(method(flit(3.2), "ceil", vec![])),  // 4
        print_stmt(method(flit(2.5), "round", vec![])), // 3 (half away from zero)
        // number theory / power ---------------------------------------
        print_stmt(method(ilit(10), "gcd", vec![ilit(15)])), // 5
        print_stmt(method(ilit(2), "pow", vec![ilit(10)])),  // 1024
        print_stmt(method(ilit(2), "**", vec![ilit(8)])),    // 256
        print_stmt(method(ilit(123), "digits", vec![])),     // [3, 2, 1]
        // numeric breadth (N1) ----------------------------------------
        print_stmt(method(flit(3.14159), "round", vec![ilit(2)])), // 3.14
        print_stmt(method(ilit(1250), "round", vec![ilit(-2)])),   // 1300 (half away)
        print_stmt(method(ilit(13), "divmod", vec![ilit(4)])),     // [3, 1]
        print_stmt(method(ilit(13), "divmod", vec![ilit(-4)])),    // [-4, -3]
        print_stmt(method(ilit(7), "fdiv", vec![ilit(2)])),        // 3.5
        print_stmt(method(ilit(1), "fdiv", vec![ilit(0)])),        // inf (never raises)
        print_stmt(method(ilit(5), "clamp", vec![ilit(1), ilit(10)])),   // 5
        print_stmt(method(ilit(-3), "clamp", vec![ilit(1), ilit(10)])),  // 1
        print_stmt(method(ilit(99), "clamp", vec![ilit(1), ilit(10)])),  // 10
        print_stmt(method(ilit(5), "between?", vec![ilit(1), ilit(10)])), // #t
        print_stmt(method(ilit(0), "between?", vec![ilit(1), ilit(10)])), // #f
        // overflow-degrade: i64::MAX.round(-1) returns self, not a wrapped garbage value.
        print_stmt(method(ilit(9223372036854775807), "round", vec![ilit(-1)])),
        // i64::MIN.divmod(-1) must NOT panic (plain `%` traps on MIN % -1 in
        // both debug and release); wrapping_rem yields 0 so the quotient wraps
        // to i64::MIN and the remainder is 0.
        print_stmt(method(ilit(i64::MIN), "divmod", vec![ilit(-1)])),
        // i64::MIN.divmod(3): the floored quotient makes the true q*d exceed i64
        // range, so the remainder reconstruction must wrap (not a checked `-`).
        print_stmt(method(ilit(i64::MIN), "divmod", vec![ilit(3)])),
    ];

    Module {
        name: "numeric_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Floats,
            Feature::DynamicTyping,
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

#[test]
fn numeric_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&numeric_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_numeric_methods_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_numeric_methods_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

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
        if stderr.contains("linker")
            && (stderr.contains("not found") || stderr.contains("No such file"))
        {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Diff against the Python/TS reference output for the same SIR module.
    assert_eq!(
        lines,
        vec![
            // This backend's `print` renders booleans Scheme-style (`#t`).
            "#t",        // 4.even?
            "#t",        // 3.odd?
            "#t",        // 0.zero?
            "#t",        // 5.positive?
            "#t",        // (-5).negative?
            "7",         // 7.to_s
            "3",         // 3.14.to_i
            "5.0",       // 5.to_f
            "5",         // (-5).abs
            "6",         // 5.succ
            "4",         // 5.pred
            "3",         // 3.7.floor
            "4",         // 3.2.ceil
            "3",         // 2.5.round (half away from zero)
            "5",         // 10.gcd(15)
            "1024",      // 2.pow(10)
            "256",       // 2 ** 8
            "[3, 2, 1]", // 123.digits
            "3.14",      // 3.14159.round(2)
            "1300",      // 1250.round(-2) — half away from zero
            "[3, 1]",    // 13.divmod(4)
            "[-4, -3]",  // 13.divmod(-4) — divisor-signed remainder
            "3.5",       // 7.fdiv(2)
            "inf",       // 1.fdiv(0) — never raises
            "5",         // 5.clamp(1, 10)
            "1",         // (-3).clamp(1, 10)
            "10",        // 99.clamp(1, 10)
            "#t",        // 5.between?(1, 10)
            "#f",        // 0.between?(1, 10)
            "9223372036854775807", // i64::MAX.round(-1) — overflow-degrade to self
            "[-9223372036854775808, 0]", // i64::MIN.divmod(-1) — no panic (wrapping_rem)
            "[-3074457345618258603, 1]", // i64::MIN.divmod(3) — no panic (wrapping_sub)
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
