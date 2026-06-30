//! Robustness proof: an emitted program that builds a **cyclic**
//! Seq/Map structure must *terminate gracefully* instead of stack-
//! overflowing (in `format`) or RefCell-panicking (in the map ops).
//!
//! `Value::Seq`/`Value::Map` are shared, *mutable* handles, so a
//! generated program can tie a knot:
//!
//!   ```text
//!   xs = [0]
//!   xs[0] = xs        # SeqSet whose value aliases the seq itself
//!   print(xs)         # would recurse forever without a cycle guard
//!   ```
//!
//! The hardened runtime threads a visited-pointer set through
//! `format`/`format_seq`/`format_map`, printing a `[...]` placeholder on
//! re-entry of a handle already on the active path.  This test hand-
//! builds exactly that module, emits Rust, compiles it with `rustc`,
//! runs it, and asserts the binary **terminates** and prints the
//! placeholder rather than crashing or looping.
//!
//! Mirrors `compile_and_run_seq_maps.rs` for the toolchain plumbing
//! (rustc/linker discovery, host-tool skips).  A missing host tool logs
//! a skip rather than reddening the build.

use std::process::Command;

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

fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
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

fn let_local(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}

fn block(stmts: Vec<Stmt>, value: Expr) -> Block {
    Block { stmts, value, span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

/// Build a module whose `main` constructs a self-referential sequence
/// and prints it:
///
///   1. `xs = [0]`
///   2. `xs[0] = xs`        (SeqSet whose value aliases `xs` ⇒ a cycle)
///   3. `print(xs)`          → must print `[[...]]` (placeholder) and
///      terminate — `xs == [xs]`, so the inner self-reference becomes
///      the `[...]` placeholder.
///   4. `print(xs[0] == xs)` → identity short-circuit ⇒ `#t`,
///      proving `value_eq` on a *self*-cyclic operand terminates.
///   5. `ys = [0]; ys[0] = ys; print(xs == ys)` → two **distinct**
///      cyclic structures (separate handles, so the `Rc::ptr_eq` fast
///      path does *not* fire).  The co-inductive visited-pair guard must
///      still terminate; structurally these are the same shape ⇒ `#t`.
fn cyclic_module() -> Module {
    let eq_expr = |a: Expr, b: Expr| Expr::BuiltinCall {
        name: "=".into(),
        args: vec![a, b],
        effects: EffectSet::PURE,
        span: s(),
    };
    let stmts = vec![
        // 1. xs = [0]
        let_local("xs", seq(vec![ilit(0)])),
        // 2. xs[0] = xs   (the knot)
        Stmt::SeqSet {
            seq: local("xs"),
            index: ilit(0),
            value: local("xs"),
            span: s(),
        },
        // 3. print(xs)  -> "[...]"
        print_stmt(local("xs")),
        // 4. print(xs[0] == xs) -> "#t"  (value_eq, self-cycle, ptr_eq)
        print_stmt(eq_expr(
            Expr::SeqIndex {
                seq: Box::new(local("xs")),
                index: Box::new(ilit(0)),
                span: s(),
            },
            local("xs"),
        )),
        // 5. ys = [0]; ys[0] = ys; print(xs == ys) -> "#t"
        //    Distinct handles ⇒ exercises the co-inductive deep walk.
        let_local("ys", seq(vec![ilit(0)])),
        Stmt::SeqSet {
            seq: local("ys"),
            index: ilit(0),
            value: local("ys"),
            span: s(),
        },
        print_stmt(eq_expr(local("xs"), local("ys"))),
    ];

    Module {
        name: "cyclic_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::MutableBindings,
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
fn cyclic_seq_compiles_runs_and_terminates() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&cyclic_module()).expect("module should compile to Rust source");

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_cyclic_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_cyclic_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile with rustc.
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

    // 4. Run the binary.  The whole point of the guard: this must
    //    *terminate*.  A stack-overflow or RefCell panic would surface as
    //    a non-success exit status (or, for an infinite loop, the test
    //    harness would hang) — so a clean exit is itself the assertion.
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero (cycle guard failed to keep it alive):\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 5. Assert the observable behaviour.
    //
    //    `xs == [xs]`, so printing the outer seq opens `[`, then meets
    //    `xs` again (now on the active path) and emits the `[...]`
    //    placeholder, then closes `]` — i.e. `[[...]]`.  The key property
    //    is that it *terminates* and *contains* the placeholder rather
    //    than recursing forever.
    assert_eq!(
        lines.len(),
        3,
        "expected three printed lines; full stdout:\n{stdout}"
    );
    assert_eq!(lines[0], "[[...]]", "cyclic seq should print with a [...] placeholder");
    assert!(
        lines[0].contains("[...]"),
        "cyclic seq output must contain the [...] placeholder; got {:?}",
        lines[0],
    );
    // Both equality checks (self-cycle via ptr_eq; distinct cycles via
    // the co-inductive visited-pair guard) must terminate as `#t`.
    assert_eq!(lines[1], "#t", "self-cyclic value_eq should be #t and terminate");
    assert_eq!(lines[2], "#t", "distinct-cycle value_eq should be #t and terminate");

    // 6. Best-effort cleanup.
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
