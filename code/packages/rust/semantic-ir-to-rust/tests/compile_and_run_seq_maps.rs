//! End-to-end proof for SIR16 **Sequences** + **Maps** in the Rust
//! backend — the final two v1 features.
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way.  It hand-builds a SIR module that exercises every new
//! IR node:
//!
//!   * sequence literal (`SeqLit`), index read (`SeqIndex`), length
//!     (`SeqLen`), and indexed write (`SeqSet`);
//!   * map literal (`MapLit`), key read (`MapGet`), and key write
//!     (`MapSet`), including a missing-key read that must yield `nil`;
//!   * a `for x in <SeqLit>` (`ForEach`) accumulation — the integration
//!     point where the new `Value::Seq` had to be reconciled with A2's
//!     `seq_iter`.
//!
//! It then emits Rust, compiles it with `rustc`, runs the binary, and
//! checks stdout.  That closes the loop the unit tests cannot — it
//! proves the emitted runtime actually *compiles and behaves*.
//!
//! `rustc` ships with every Rust toolchain (it is what `cargo` drives),
//! so this runs in CI.  If `rustc` (or a usable linker) is unavailable
//! the test logs a skip rather than failing — a missing host tool must
//! never redden a build for reasons unrelated to the change.

use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Scope, Span, Stmt,
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

fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}

/// `(name arg0 arg1 ...)` builtin call, pure.
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `print(expr)` as an effectful statement.
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

fn let_local(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}

fn assign_local(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Local, value, span: s() }
}

fn block(stmts: Vec<Stmt>, value: Expr) -> Block {
    Block { stmts, value, span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn seq_index(s_expr: Expr, i: Expr) -> Expr {
    Expr::SeqIndex { seq: Box::new(s_expr), index: Box::new(i), span: s() }
}

fn seq_len(s_expr: Expr) -> Expr {
    Expr::SeqLen { seq: Box::new(s_expr), span: s() }
}

fn map_lit(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries
            .into_iter()
            .map(|(key, value)| MapEntry { key, value })
            .collect(),
        span: s(),
    }
}

fn map_get(m: Expr, k: Expr) -> Expr {
    Expr::MapGet { map: Box::new(m), key: Box::new(k), span: s() }
}

/// Build a module whose `main` exercises sequences, maps, and a
/// `ForEach` over a sequence literal, printing observable results:
///
///  1. `xs = [10, 20, 30]`; print `xs[1]`              → "20"
///  2. print `len(xs)`                                  → "3"
///  3. `xs[0] = 99`; print `xs[0]`  (SeqSet mutation)   → "99"
///  4. `d = {"a": 1, "b": 2}`; print `d["b"]`          → "2"
///  5. print `d["missing"]`  (missing key ⇒ nil)        → "nil"
///  6. `d["c"] = 7`; print `d["c"]`  (MapSet)           → "7"
///  7. `total = 0; for v in [1, 2, 3, 4] { total += v }`; print total
///     (ForEach over a *real* SeqLit, the reconciliation point) → "10"
fn demo_module() -> Module {
    let stmts = vec![
        // 1. xs = [10, 20, 30]; print xs[1]
        let_local("xs", seq(vec![ilit(10), ilit(20), ilit(30)])),
        print_stmt(seq_index(local("xs"), ilit(1))),
        // 2. print len(xs)
        print_stmt(seq_len(local("xs"))),
        // 3. xs[0] = 99; print xs[0]
        Stmt::SeqSet {
            seq: local("xs"),
            index: ilit(0),
            value: ilit(99),
            span: s(),
        },
        print_stmt(seq_index(local("xs"), ilit(0))),
        // 4. d = {"a": 1, "b": 2}; print d["b"]
        let_local(
            "d",
            map_lit(vec![(slit("a"), ilit(1)), (slit("b"), ilit(2))]),
        ),
        print_stmt(map_get(local("d"), slit("b"))),
        // 5. print d["missing"]  -> nil
        print_stmt(map_get(local("d"), slit("missing"))),
        // 6. d["c"] = 7; print d["c"]
        Stmt::MapSet {
            map: local("d"),
            key: slit("c"),
            value: ilit(7),
            span: s(),
        },
        print_stmt(map_get(local("d"), slit("c"))),
        // 7. total = 0; for v in [1, 2, 3, 4] { total = total + v }; print total
        let_local("total", ilit(0)),
        Stmt::ForEach {
            var: "v".into(),
            iter: seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            body: block(
                vec![assign_local("total", call("+", vec![local("total"), local("v")]))],
                Expr::NilLit { span: s() },
            ),
            span: s(),
        },
        print_stmt(local("total")),
    ];

    Module {
        name: "seq_maps_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Maps,
            Feature::Loops,
            Feature::MutableBindings,
            // String literals are used as map keys.
            Feature::Strings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: block(stmts, Expr::NilLit { span: s() }),
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
fn sequences_and_maps_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_seq_maps_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_seq_maps_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile with rustc.  `--edition 2021` is required (raw idents +
    //    2018+ closure capture).  A host whose default linker is absent
    //    can point the test at a working one via `SIR_TEST_RUSTC_LINKER`
    //    (e.g. the toolchain's bundled `rust-lld`).
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
        // A *missing linker* is a host-environment issue, not a defect in
        // the emitted code — skip rather than redden the build.  Any other
        // compile failure (a genuine codegen bug) still fails the test.
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file"))
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

    // 4. Run the binary and capture stdout.
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 5. Assert the program's observable behaviour.
    assert_eq!(
        lines,
        vec!["20", "3", "99", "2", "nil", "7", "10"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 6. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
