//! Execution proof for SIR16 sequences on the C backend — hand-build modules
//! (producer-agnostic), emit C, compile with a real gcc/clang-style compiler,
//! run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! Covers every `Feature::Sequences`-gated node — `SeqLit`, `SeqIndex`,
//! `SeqLen`, `SeqSet` — plus `ForEach` (reachable once `Loops` is accepted) and
//! structural equality (`[1,2] == [1,2]`). Hand-built to bypass any frontend
//! that masks these nodes (the totality lesson).

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span,
    Stmt, CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn run(module: &Module) -> Option<String> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_seq_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "run failed (exit {:?})", r.status.code());
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
fn let_(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}
/// A `main` module declaring Sequences + Loops.
fn seq_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "seqprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Loops,
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
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

#[test]
fn seq_literal_displays_as_a_bracketed_list() {
    // `puts([1, 2, 3])` → `[1, 2, 3]`, matching the Go/Rust format.
    match run(&seq_module(vec![puts(seq(vec![ilit(1), ilit(2), ilit(3)]))])) {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn seq_structural_equality() {
    // The point of this batch: `Array#==` is STRUCTURAL, so two DISTINCT arrays
    // with equal elements compare equal — driven through an `if` so the check
    // is convention-independent. Positive, negative, and nested.
    let if_eq = |a: Expr, b: Expr, then: &str, els: &str| Stmt::ExprStmt {
        expr: Expr::If {
            cond: Box::new(bc("=", vec![a, b])),
            then_branch: Box::new(Block {
                stmts: vec![puts(Expr::StrLit { value: then.into(), span: s() })],
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            else_branch: Box::new(Block {
                stmts: vec![puts(Expr::StrLit { value: els.into(), span: s() })],
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    };
    let m = seq_module(vec![
        if_eq(seq(vec![ilit(1), ilit(2)]), seq(vec![ilit(1), ilit(2)]), "same", "diff"),
        if_eq(seq(vec![ilit(1), ilit(2)]), seq(vec![ilit(1), ilit(3)]), "same", "diff"),
        if_eq(
            seq(vec![seq(vec![ilit(1)]), ilit(2)]),
            seq(vec![seq(vec![ilit(1)]), ilit(2)]),
            "same",
            "diff",
        ),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "same\ndiff\nsame\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn seq_index_and_len() {
    // `a = [10, 20, 30]; puts a[1]; puts a[-1]; puts a[9]; puts a.length`.
    // `a[9]` is OOB → nil (prints "nil"); a negative index counts from the end.
    let m = seq_module(vec![
        let_("a", seq(vec![ilit(10), ilit(20), ilit(30)])),
        puts(Expr::SeqIndex { seq: Box::new(local("a")), index: Box::new(ilit(1)), span: s() }),
        puts(Expr::SeqIndex { seq: Box::new(local("a")), index: Box::new(ilit(-1)), span: s() }),
        puts(Expr::SeqIndex { seq: Box::new(local("a")), index: Box::new(ilit(9)), span: s() }),
        puts(Expr::SeqLen { seq: Box::new(local("a")), span: s() }),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "20\n30\nnil\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn seq_set_mutates_the_shared_box() {
    // `a = [1, 2, 3]; a[1] = 99; puts a` → `[1, 99, 3]`. The write is visible
    // through the same handle (a boxed, shared sequence).
    let m = seq_module(vec![
        let_("a", seq(vec![ilit(1), ilit(2), ilit(3)])),
        Stmt::SeqSet { seq: local("a"), index: ilit(1), value: ilit(99), span: s() },
        puts(local("a")),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "[1, 99, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn for_each_iterates_a_sequence_block_scoped() {
    // `for x in [1, 2, 3]: puts x` → `1\n2\n3`, and `x` is block-scoped: an
    // enclosing `x = 99` survives the loop.
    let m = seq_module(vec![
        let_("x", ilit(99)),
        Stmt::ForEach {
            var: "x".into(),
            iter: seq(vec![ilit(1), ilit(2), ilit(3)]),
            body: Block { stmts: vec![puts(local("x"))], value: Expr::NilLit { span: s() }, span: s() },
            span: s(),
        },
        puts(local("x")),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1\n2\n3\n99\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn cyclic_sequence_does_not_stack_overflow() {
    // `SeqSet` is the first MUTABLE heap aggregate, so a sequence can be made
    // self-referential (`a[0] = a`) — a newly-reachable cycle. Equality and
    // display must TERMINATE (via the depth caps), not crash the stack. `run`
    // asserts the process exits 0; a stack overflow would be a non-zero exit.
    let nil = || Expr::NilLit { span: s() };
    let m = seq_module(vec![
        // a = [nil]; a[0] = a   → a self-referential sequence
        let_("a", seq(vec![nil()])),
        Stmt::SeqSet { seq: local("a"), index: ilit(0), value: local("a"), span: s() },
        // a == a → #t via the identical-handle fast path (no walk)
        puts(bc("=", vec![local("a"), local("a")])),
        // print(a) → terminates via the fmt depth cap (prints a bounded string)
        puts(local("a")),
        // two DISTINCT mutually-cyclic sequences: a[0]=b, b[0]=a
        let_("b", seq(vec![nil()])),
        Stmt::SeqSet { seq: local("b"), index: ilit(0), value: local("a"), span: s() },
        Stmt::SeqSet { seq: local("a"), index: ilit(0), value: local("b"), span: s() },
        // a == b → terminates via the eq depth cap (result is cap-defined; the
        // point is it does not stack-overflow)
        puts(bc("=", vec![local("a"), local("b")])),
    ]);
    match run(&m) {
        Some(out) => {
            let lines: Vec<&str> = out.lines().collect();
            assert_eq!(lines.first().copied(), Some("#t"), "a == a is true (fast path)");
            assert_eq!(lines.len(), 3, "all three puts ran — no stack overflow");
        }
        None => eprintln!("skip: no cc"),
    }
}
