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
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
                expr,
            ],
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
        manifest: FeatureManifest::from_features(&[Feature::ConsoleIO, Feature::Strings, 
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

// ── SIR16 addendum: loop control (`break`/`continue`), task #63 ────────

/// `if (cond) { then_stmts } else {}` as a bare statement — the exact
/// shape whose value-position codegen used to route through Go's
/// `func() Value {...}()` IIFE lift before `emit_stmt`'s own
/// `Stmt::ExprStmt`/`Expr::If` special case fixed it.
fn if_stmt(cond: Expr, then_stmts: Vec<Stmt>) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(Block {
                stmts: then_stmts,
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            else_branch: Box::new(Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }
}

fn loop_control_module(name: &str, main_stmts: Vec<Stmt>) -> Module {
    Module {
        name: name.into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::MutableBindings,
            Feature::Loops,
            Feature::LoopControl,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: main_stmts,
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

/// Run a hand-built module through `go run`, returning trimmed stdout.
/// `None` when `go` is unavailable. Mirrors
/// `loops_and_mutable_bindings_compile_and_run`'s own harness above,
/// factored out so both new tests below can share it.
fn run_via_go(module: &Module, tag: &str) -> Option<String> {
    if !go_available() {
        eprintln!("skipping {tag}: go not on PATH");
        return None;
    }
    let artifact = compile(module).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_lc_{tag}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let run_out = Command::new("go").arg("run").arg(&src_path).output().expect("invoke go run");
    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run for `{tag}`:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }
    let _ = std::fs::remove_file(&src_path);
    Some(String::from_utf8_lossy(&run_out.stdout).trim_end().to_string())
}

#[test]
fn while_loop_continue_skips_five_and_break_stops_past_seven() {
    // n := 0; sum := 0
    // for _sir_truthy(n < 10) {
    //   n = n + 1
    //   if (n = 5) { continue }
    //   if (n > 7) { break }
    //   sum = sum + n
    // }
    // print(sum)  → 1+2+3+4 (skip 5) +6+7 = 23, then break at n=8
    let body = Block {
        stmts: vec![
            assign_local("n", call("+", vec![var_local("n"), ilit(1)])),
            if_stmt(
                call("=", vec![var_local("n"), ilit(5)]),
                vec![Stmt::Continue { span: s() }],
            ),
            if_stmt(call(">", vec![var_local("n"), ilit(7)]), vec![Stmt::Break { span: s() }]),
            assign_local("sum", call("+", vec![var_local("sum"), var_local("n")])),
        ],
        value: Expr::NilLit { span: s() },
        span: s(),
    };
    let main_stmts = vec![
        let_local("n", ilit(0)),
        let_local("sum", ilit(0)),
        Stmt::While { cond: call("<", vec![var_local("n"), ilit(10)]), body, span: s() },
        print_stmt(var_local("sum")),
    ];
    let module = loop_control_module("loop_control_while", main_stmts);
    if let Some(stdout) = run_via_go(&module, "loop_control_while") {
        assert_eq!(stdout, "23");
    }
}

#[test]
fn for_each_loop_break_stops_iteration_before_the_matching_element() {
    // sum := 0
    // for x := range _sir_seq_iter([1,2,3,4,5]) {
    //   if (x = 3) { break }
    //   sum = sum + x
    // }
    // print(sum)  → 1 + 2 = 3 (the loop never adds 3, 4, or 5)
    let body = Block {
        stmts: vec![
            if_stmt(call("=", vec![var_local("x"), ilit(3)]), vec![Stmt::Break { span: s() }]),
            assign_local("sum", call("+", vec![var_local("sum"), var_local("x")])),
        ],
        value: Expr::NilLit { span: s() },
        span: s(),
    };
    let main_stmts = vec![
        let_local("sum", ilit(0)),
        Stmt::ForEach {
            var: "x".into(),
            iter: Expr::SeqLit {
                items: vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)],
                span: s(),
            },
            body,
            span: s(),
        },
        print_stmt(var_local("sum")),
    ];
    let mut module = loop_control_module("loop_control_foreach", main_stmts);
    module.manifest = FeatureManifest::from_features(&[
        Feature::ConsoleIO,
        Feature::Strings,
        Feature::MutableBindings,
        Feature::Loops,
        Feature::LoopControl,
        Feature::Sequences,
    ]);
    if let Some(stdout) = run_via_go(&module, "loop_control_foreach") {
        assert_eq!(stdout, "3");
    }
}
