//! End-to-end proof for SIR16 **MutableBindings** + **Loops** in the Go
//! backend (A5).
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way: it hand-builds a SIR module that uses a mutable local,
//! a `while` loop, and a `for`-range accumulator, emits Go, writes it to
//! a temp `.go` file, runs it with `go run`, and checks stdout.  That
//! closes the loop the unit tests cannot — it proves the emitted Go
//! actually *compiles and behaves* under a real Go toolchain, which is
//! the only way to catch Go's strict unused-variable / `:=`-vs-`=`
//! rules (the whole reason this feature is delicate).
//!
//! The test gates on `go` being available (`go version`).  If the Go
//! toolchain is absent it logs a skip rather than failing — a missing
//! tool should never redden a build for reasons unrelated to the change
//! (mirrors `compile_and_run_floats.rs`).

use std::process::Command;

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

fn var_local(name: &str) -> Expr {
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

/// `<name> := <value>` — first-occurrence local binding.
fn let_local(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding {
        name: name.into(),
        value,
        sir_type: None,
        span: s(),
    }
}

/// `<name> = <value>` — reassignment of an already-declared local.
fn assign_local(name: &str, value: Expr) -> Stmt {
    Stmt::Assign {
        name: name.into(),
        scope: Scope::Local,
        value,
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

/// Build a module whose `main`:
///   1. seeds a mutable `sum := 0`,
///   2. accumulates `0+1+2+3+4` with a `for i in range(0,5,1): sum = sum + i`,
///   3. prints `sum`                         → "10"
///   4. seeds `n := 3`, counts it down to 0 with a `while n > 0: n = n - 1`,
///   5. prints `n`                           → "0"
///   6. reassigns `sum = 99` (plain MutableBindings) and prints it → "99"
fn demo_module() -> Module {
    let for_range = Stmt::ForRange {
        var: "i".into(),
        start: ilit(0),
        stop: ilit(5),
        step: ilit(1),
        body: Block {
            stmts: vec![assign_local("sum", call("+", vec![var_local("sum"), var_local("i")]))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        span: s(),
    };

    let while_loop = Stmt::While {
        cond: call(">", vec![var_local("n"), ilit(0)]),
        body: Block {
            stmts: vec![assign_local("n", call("-", vec![var_local("n"), ilit(1)]))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        span: s(),
    };

    let stmts = vec![
        // 1. mutable accumulator seeded at 0.
        let_local("sum", ilit(0)),
        // 2. for-range accumulate.
        for_range,
        // 3. print sum → 10.
        print_stmt(var_local("sum")),
        // 4. while countdown.
        let_local("n", ilit(3)),
        while_loop,
        // 5. print n → 0.
        print_stmt(var_local("n")),
        // 6. plain reassignment, then print → 99.
        assign_local("sum", ilit(99)),
        print_stmt(var_local("sum")),
    ];

    Module {
        name: "loops_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::MutableBindings,
            Feature::Loops,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
fn loops_and_mutable_bindings_compile_and_run() {
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
    let src_path = dir.join(format!("sir_go_loops_{nonce}.go"));
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
        vec!["10", "0", "99"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 5. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
}
