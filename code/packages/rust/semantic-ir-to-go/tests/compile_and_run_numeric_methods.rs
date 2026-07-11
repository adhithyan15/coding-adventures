//! Execution proof for the Ruby `Integer`/`Float` (Numeric) method catalog on
//! the Go backend — parity-fill with the Python/TS `sir-runtime-oop` runtimes.
//!
//! Each program builds a SIR module that calls a numeric method (via the
//! `__method__` builtin the frontend emits for `recv.meth(args…)`), emits Go,
//! runs it with `go run`, and asserts the printed value equals what the
//! reference runtimes yield for the identical operation.  Booleans print as the
//! runtime's `#t`/`#f`, arrays as `[a, b, …]`.
//!
//! Covered here (the methods ADDED in this change, plus a couple of pre-existing
//! ones as controls): `even?`, `odd?`, `abs`, `to_s`, `to_i`, `gcd`, `pow`,
//! `**`, `digits`, `succ`, `floor`, `ceil`, `round`, and the block iterators
//! `upto`/`downto`/`step`.
//!
//! A missing `go` toolchain logs a skip rather than reddening the build
//! (mirrors `compile_and_run_coll_methods.rs`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param,
    ParamKind, Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(n: i64) -> Expr {
    Expr::IntLit { value: n, span: s() }
}

fn flit(x: f64) -> Expr {
    Expr::FloatLit { value: x, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn var_p(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
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
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// One-parameter lambda `fn_name(x) -> body`, registered as a top-level
/// function; the caller builds a `MakeClosure` referencing it.
fn lambda_fn(fn_name: &str, param: &str, body: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params: vec![Param {
            name: param.into(),
            kind: ParamKind::Required,
            sir_type: None,
            default: None,
            span: s(),
        }],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

fn manifest() -> FeatureManifest {
    FeatureManifest::from_features(&[
        Feature::Closures,
        Feature::Sequences,
        Feature::Strings,
        Feature::Floats,
        Feature::MutableBindings,
        Feature::DynamicTyping,
    ])
}

fn program(functions: Vec<Function>) -> Module {
    Module {
        name: "numeric_methods_demo".into(),
        manifest: manifest(),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// The catalog demo `main`.  Each printed line's expected value is what the
/// Python/TS reference runtime yields for the identical operation.
fn catalog_module() -> Module {
    // `puts`-style print lambda the block iterators drive (prints each yielded
    // value on its own line).
    let show = lambda_fn("__lam_show", "x", builtin("print", vec![var_p("x")]));

    let stmts = vec![
        // 4.even? → #t ; 3.odd? → #t
        print_stmt(method(ilit(4), "even?", vec![])),
        print_stmt(method(ilit(3), "odd?", vec![])),
        // (-5).abs → 5
        print_stmt(method(ilit(-5), "abs", vec![])),
        // 7.to_s → "7"
        print_stmt(method(ilit(7), "to_s", vec![])),
        // 3.14.to_i → 3   (truncate toward zero)
        print_stmt(method(flit(3.14), "to_i", vec![])),
        // 10.gcd(15) → 5
        print_stmt(method(ilit(10), "gcd", vec![ilit(15)])),
        // 2.pow(10) → 1024
        print_stmt(method(ilit(2), "pow", vec![ilit(10)])),
        // 2 ** 8 → 256   (via the `**` method name)
        print_stmt(method(ilit(2), "**", vec![ilit(8)])),
        // 123.digits → [3, 2, 1]  (least-significant first)
        print_stmt(method(ilit(123), "digits", vec![])),
        // 5.succ → 6
        print_stmt(method(ilit(5), "succ", vec![])),
        // 3.7.floor → 3 ; 3.2.ceil → 4 ; 2.5.round → 3 (half away from zero)
        print_stmt(method(flit(3.7), "floor", vec![])),
        print_stmt(method(flit(3.2), "ceil", vec![])),
        print_stmt(method(flit(2.5), "round", vec![])),
        // Numeric breadth (N1): round(ndigits) / divmod / fdiv / clamp / between?
        // 3.14159.round(2) → 3.14 ; 1250.round(-2) → 1300 (half away from zero)
        print_stmt(method(flit(3.14159), "round", vec![ilit(2)])),
        print_stmt(method(ilit(1250), "round", vec![ilit(-2)])),
        // 13.divmod(4) → [3, 1] ; 13.divmod(-4) → [-4, -3] (divisor-signed rem)
        print_stmt(method(ilit(13), "divmod", vec![ilit(4)])),
        print_stmt(method(ilit(13), "divmod", vec![ilit(-4)])),
        // 7.fdiv(2) → 3.5 ; 1.fdiv(0) → Infinity (never raises)
        print_stmt(method(ilit(7), "fdiv", vec![ilit(2)])),
        print_stmt(method(ilit(1), "fdiv", vec![ilit(0)])),
        // 5.clamp(1, 10) → 5 ; (-3).clamp(1, 10) → 1 ; 99.clamp(1, 10) → 10
        print_stmt(method(ilit(5), "clamp", vec![ilit(1), ilit(10)])),
        print_stmt(method(ilit(-3), "clamp", vec![ilit(1), ilit(10)])),
        print_stmt(method(ilit(99), "clamp", vec![ilit(1), ilit(10)])),
        // 5.between?(1, 10) → #t ; 0.between?(1, 10) → #f
        print_stmt(method(ilit(5), "between?", vec![ilit(1), ilit(10)])),
        print_stmt(method(ilit(0), "between?", vec![ilit(1), ilit(10)])),
        // 1.upto(3) { |i| print i } → 1,2,3 (three lines).  The block itself
        // prints; the iterator's return value (the receiver) is discarded, so
        // this is a bare ExprStmt, NOT wrapped in another print.
        Stmt::ExprStmt {
            expr: method(ilit(1), "upto", vec![ilit(3), closure("__lam_show")]),
            span: s(),
        },
        // 3.downto(1) { |i| print i } → 3,2,1 (three lines)
        Stmt::ExprStmt {
            expr: method(ilit(3), "downto", vec![ilit(1), closure("__lam_show")]),
            span: s(),
        },
        // 0.step(4, 2) { |i| print i } → 0,2,4 (three lines)
        Stmt::ExprStmt {
            expr: method(ilit(0), "step", vec![ilit(4), ilit(2), closure("__lam_show")]),
            span: s(),
        },
    ];

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    program(vec![show, main])
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_go(source: &str, tag: &str) -> std::process::Output {
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_numeric_{tag}_{nonce}.go"));
    std::fs::write(&src_path, source).expect("write temp source");
    let out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    out
}

#[test]
fn numeric_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&catalog_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "catalog");
    if !run_out.status.success() {
        panic!(
            "emitted Go failed:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "#t",        // 4.even?
            "#t",        // 3.odd?
            "5",         // (-5).abs
            "7",         // 7.to_s
            "3",         // 3.14.to_i
            "5",         // 10.gcd(15)
            "1024",      // 2.pow(10)
            "256",       // 2 ** 8
            "[3, 2, 1]", // 123.digits
            "6",         // 5.succ
            "3",         // 3.7.floor
            "4",         // 3.2.ceil
            "3",         // 2.5.round
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
            "1", "2", "3", // 1.upto(3)
            "3", "2", "1", // 3.downto(1)
            "0", "2", "4", // 0.step(4, 2)
        ],
        "unexpected stdout:\n{stdout}"
    );
}
