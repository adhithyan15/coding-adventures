//! End-to-end proof for SIR16 **Sequences** + **Maps** in the Go backend
//! (A6) — the final two v1 features, which complete the Go backend's
//! SIR-v1 parity (the fifth and last backend to reach v1).
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way: it hand-builds a SIR module that builds a sequence
//! (lit/index/len/set), a map (lit/get/set), and a `for x in [10,20,30]`
//! ForEach accumulator, emits Go, writes it to a temp `.go` file, runs it
//! with `go run`, and checks stdout.  That closes the loop the unit tests
//! cannot — it proves the emitted Go actually *compiles and behaves*
//! under a real Go toolchain (Go's strict unused-variable / `:=`-vs-`=`
//! rules make codegen delicate).
//!
//! The test gates on `go` being available (`go version`).  If the Go
//! toolchain is absent it logs a skip rather than failing — a missing
//! tool should never redden a build for reasons unrelated to the change
//! (mirrors `compile_and_run_loops.rs`).

use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
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

fn var_local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

fn let_local(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), value, sir_type: None, span: s() }
}

fn assign_local(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Local, value, span: s() }
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

/// Build a module whose `main`:
///   1. `xs := [1, 2, 3]`            (SeqLit)
///   2. `xs[0] = 99`                 (SeqSet — mutate shared seq)
///   3. print `xs[0]`                (SeqIndex)               → "99"
///   4. print `len(xs)`              (SeqLen)                 → "3"
///   5. alias `ys := xs`; print `ys[0]` — aliasing observes the
///      earlier SeqSet through the shared handle               → "99"
///   6. `d := {a: 1, b: 2}`          (MapLit)
///   7. `d[c] = 3`                   (MapSet — insert)
///   8. print `d[b]`, `d[c]`, `d[zzz]` (MapGet; missing ⇒ nil) → "2", "3", "nil"
///   9. `for x in [10, 20, 30]: total = total + x`; print total → "60"
fn demo_module() -> Module {
    let map_lit = Expr::MapLit {
        entries: vec![
            MapEntry { key: slit("a"), value: ilit(1) },
            MapEntry { key: slit("b"), value: ilit(2) },
        ],
        span: s(),
    };

    let for_each = Stmt::ForEach {
        var: "x".into(),
        iter: Expr::SeqLit { items: vec![ilit(10), ilit(20), ilit(30)], span: s() },
        body: Block {
            stmts: vec![assign_local("total", call("+", vec![var_local("total"), var_local("x")]))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        span: s(),
    };

    let stmts = vec![
        // Sequences.
        let_local("xs", Expr::SeqLit { items: vec![ilit(1), ilit(2), ilit(3)], span: s() }),
        Stmt::SeqSet { seq: var_local("xs"), index: ilit(0), value: ilit(99), span: s() },
        print_stmt(Expr::SeqIndex {
            seq: Box::new(var_local("xs")),
            index: Box::new(ilit(0)),
            span: s(),
        }),
        print_stmt(Expr::SeqLen { seq: Box::new(var_local("xs")), span: s() }),
        // Aliasing: ys shares the same backing slice as xs.
        let_local("ys", var_local("xs")),
        print_stmt(Expr::SeqIndex {
            seq: Box::new(var_local("ys")),
            index: Box::new(ilit(0)),
            span: s(),
        }),
        // Maps.
        let_local("d", map_lit),
        Stmt::MapSet { map: var_local("d"), key: slit("c"), value: ilit(3), span: s() },
        print_stmt(Expr::MapGet {
            map: Box::new(var_local("d")),
            key: Box::new(slit("b")),
            span: s(),
        }),
        print_stmt(Expr::MapGet {
            map: Box::new(var_local("d")),
            key: Box::new(slit("c")),
            span: s(),
        }),
        // Missing key ⇒ nil (printed as "nil").
        print_stmt(Expr::MapGet {
            map: Box::new(var_local("d")),
            key: Box::new(slit("zzz")),
            span: s(),
        }),
        // ForEach over a real SeqLit.
        let_local("total", ilit(0)),
        for_each,
        print_stmt(var_local("total")),
    ];

    Module {
        name: "seq_maps_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::MutableBindings,
            Feature::Loops,
            Feature::Sequences,
            Feature::Maps,
            Feature::Strings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
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

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn sequences_and_maps_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    // 2. Write the source to a unique temp file.  `go run` requires a
    //    `.go` extension.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_seq_maps_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile + run with `go run` (arg vector — no shell).
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");

    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    // 4. Assert the program's observable behaviour.
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["99", "3", "99", "2", "3", "nil", "60"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 5. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
}
