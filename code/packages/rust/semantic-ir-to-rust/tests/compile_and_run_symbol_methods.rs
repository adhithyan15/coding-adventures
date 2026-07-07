//! End-to-end proof for the **Ruby `Symbol` method catalog** in the Rust
//! backend (parity with the Python/TypeScript `sir-runtime-oop` reference).
//!
//! A Ruby `Symbol` reaches this backend as a `Value::Sym` (an interned
//! `Rc<str>`), and a call `:sym.meth(…)` arrives as the narrow-waist
//! envelope `BuiltinCall("__method__", [recv, StrLit("meth"), …])`, which the
//! emitter turns into `__sir::call_method(recv, "meth", vec![…])`.  The
//! inline `__sir` runtime routes a `Value::Sym` receiver to `symbol_method`,
//! an EXPLICIT `(name)` match — never reflection — ported from the reference.
//!
//! This test hand-builds SIR modules exercising the catalog, emits Rust,
//! compiles it with `rustc`, runs the binary, and diffs stdout against the
//! values Ruby (and the Python/TS reference) produce for the SAME module:
//!
//!   * `:hello.to_s`                     → `"hello"`  (a String)
//!   * `:hi.length`                      → `2`
//!   * `:abc.upcase.inspect`             → `":ABC"`   (upcase returns a Symbol)
//!   * `:x.inspect`                      → `":x"`
//!   * `:hELLo.capitalize.inspect`       → `":Hello"` (capitalize → Symbol)
//!   * `[1,2,3].map(&:to_s).join(",")`   → `"1,2,3"`  (`Symbol#to_proc`)
//!
//! Note on the `upcase`/`capitalize` assertions: a bare `Value::Sym` prints
//! as its *name* (`ABC`), with no leading `:`, so those results are chained
//! through `.inspect` (which prefixes `:`) to prove the method returned a
//! **Symbol** and not a String.  The `map(&:to_s)` line proves `to_proc` is
//! runtime-reachable: the frontend lowers `&:to_s` to
//! `block_pass(SymLit("to_s"))`, which the emitter turns into
//! `sym_to_proc(intern("to_s"))` — the same closure `:to_s.to_proc` builds —
//! re-entering `call_method(x, "to_s", [])` through explicit dispatch.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing; a missing host tool must never redden a build.  The host can
//! point the test at a working linker via `SIR_TEST_RUSTC_LINKER`.

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

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn symlit(v: &str) -> Expr {
    Expr::SymLit { name: v.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(name)];
    all.append(&mut args);
    Expr::BuiltinCall {
        name: "__method__".into(),
        args: all,
        effects: EffectSet::PURE,
        span: s(),
    }
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

fn demo_module(main_stmts: Vec<Stmt>) -> Module {
    let functions = vec![Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }];

    Module {
        name: "symbol_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Symbols,
            Feature::Closures,
            Feature::DynamicTyping,
        ]),
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

fn full_demo() -> Module {
    let main_stmts = vec![
        // 1. :hello.to_s  → "hello"
        print_stmt(method(symlit("hello"), "to_s", vec![])),
        // 2. :hi.length  → 2
        print_stmt(method(symlit("hi"), "length", vec![])),
        // 3. :abc.upcase.inspect  → ":ABC"  (upcase returns a Symbol)
        print_stmt(method(method(symlit("abc"), "upcase", vec![]), "inspect", vec![])),
        // 4. :x.inspect  → ":x"
        print_stmt(method(symlit("x"), "inspect", vec![])),
        // 5. :hELLo.capitalize.inspect  → ":Hello"  (capitalize returns a Symbol)
        print_stmt(method(method(symlit("hELLo"), "capitalize", vec![]), "inspect", vec![])),
        // 6. [1,2,3].map(&:to_s).join(",")  → "1,2,3"  (Symbol#to_proc reachable)
        print_stmt(method(
            method(
                seq(vec![ilit(1), ilit(2), ilit(3)]),
                "map",
                vec![call("block_pass", vec![symlit("to_s")])],
            ),
            "join",
            vec![slit(",")],
        )),
    ];

    demo_module(main_stmts)
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn symbol_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&full_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_symbol_methods_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_symbol_methods_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
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

    assert_eq!(
        lines,
        vec![
            "hello",  // :hello.to_s
            "2",      // :hi.length
            ":ABC",   // :abc.upcase.inspect
            ":x",     // :x.inspect
            ":Hello", // :hELLo.capitalize.inspect
            "1,2,3",  // [1,2,3].map(&:to_s).join(",")
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
