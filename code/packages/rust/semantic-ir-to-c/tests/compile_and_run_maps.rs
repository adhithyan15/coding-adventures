//! Execution proof for SIR16 maps on the C backend — hand-build modules
//! (producer-agnostic), emit C, compile with a real gcc/clang-style compiler,
//! run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! Covers every `Feature::Maps`-gated node — `MapLit`, `MapGet`, `MapSet` —
//! plus structural composite keys, insertion-order display, and the
//! newly-reachable cyclic-map case (`m[k] = m`, constructible via the mutable
//! `MapSet`). Hand-built to bypass any frontend that masks these nodes (the
//! totality lesson: accepting a feature obligates handling every node it can
//! surface for a producer-agnostic module, not just the ones a frontend emits).

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, MapEntry, Metadata, Module, Scope,
    Span, Stmt, CURRENT_SIR_VERSION,
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
    let stem = format!("sirc_map_{}_{}", std::process::id(), n);
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
fn strlit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}
fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}
fn maplit(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries
            .into_iter()
            .map(|(key, value)| MapEntry { key, value })
            .collect(),
        span: s(),
    }
}
fn mapget(map: Expr, key: Expr) -> Expr {
    Expr::MapGet { map: Box::new(map), key: Box::new(key), span: s() }
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
/// A `main` module declaring Maps + Loops + Sequences + Strings. Sequences lets
/// a composite `[1, 2]` map key be built; Strings lets a `"found"` value be
/// used — both exercised below.
fn map_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "mapprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Maps,
            Feature::Loops,
            Feature::Sequences,
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
fn map_get_reads_present_and_missing_keys() {
    // `h = {10 => 100, 20 => 200}; puts h[10]; puts h[30]` → `100`, then `nil`
    // (a missing key yields nil, not an error — matching the Go/Rust reference).
    let m = map_module(vec![
        let_("h", maplit(vec![(ilit(10), ilit(100)), (ilit(20), ilit(200))])),
        puts(mapget(local("h"), ilit(10))),
        puts(mapget(local("h"), ilit(30))),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "100\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_set_inserts_updates_and_shares_the_box() {
    // `h = {1 => 1}; h[1] = 99 (update in place); h[2] = 2 (append new key)`.
    // Aliasing: `g = h` shares the box, so `g[3] = 3` is visible through `h`.
    let m = map_module(vec![
        let_("h", maplit(vec![(ilit(1), ilit(1))])),
        Stmt::MapSet { map: local("h"), key: ilit(1), value: ilit(99), span: s() },
        Stmt::MapSet { map: local("h"), key: ilit(2), value: ilit(2), span: s() },
        let_("g", local("h")),
        Stmt::MapSet { map: local("g"), key: ilit(3), value: ilit(3), span: s() },
        puts(mapget(local("h"), ilit(1))), // updated → 99
        puts(mapget(local("h"), ilit(2))), // inserted → 2
        puts(mapget(local("h"), ilit(3))), // written through the alias → 3
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "99\n2\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_composite_key_is_structural() {
    // `{[1, 2] => "found"}[[1, 2]]` → `found`: keys are compared by STRUCTURAL
    // equality (`_sir_value_eq`), so a DISTINCT `[1, 2]` literal still matches —
    // the whole point of an assoc-array over a pointer-identity table.
    let m = map_module(vec![
        let_("h", maplit(vec![(seq(vec![ilit(1), ilit(2)]), strlit("found"))])),
        puts(mapget(local("h"), seq(vec![ilit(1), ilit(2)]))),
        // A key that differs by value misses → nil.
        puts(mapget(local("h"), seq(vec![ilit(1), ilit(3)]))),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "found\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_literal_displays_as_a_brace_list() {
    // `puts({1 => 2, 3 => 4})` → `{1: 2, 3: 4}` — brace-wrapped, colon-space,
    // insertion order, matching the Go/Rust backends (a uniform, documented
    // divergence from real Ruby's ` => `, kept for cross-backend agreement).
    let m = map_module(vec![puts(maplit(vec![
        (ilit(1), ilit(2)),
        (ilit(3), ilit(4)),
    ]))]);
    match run(&m) {
        Some(out) => assert_eq!(out, "{1: 2, 3: 4}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_literal_duplicate_key_overwrites() {
    // `{1 => 1, 1 => 2}` collapses to a single entry `1 => 2` (a later duplicate
    // key overwrites), matching Ruby's Hash literal and the Go/Rust `_sir_map_lit`.
    let m = map_module(vec![puts(maplit(vec![
        (ilit(1), ilit(1)),
        (ilit(1), ilit(2)),
    ]))]);
    match run(&m) {
        Some(out) => assert_eq!(out, "{1: 2}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_equality_is_structural_and_positional() {
    // Two DISTINCT maps with equal entries in the SAME insertion order compare
    // equal; a different value (or a different order) is unequal — positional
    // structural equality, matching Go's `[]MapEntry` zip and Rust's `zip`.
    let if_eq = |a: Expr, b: Expr, then: &str, els: &str| Stmt::ExprStmt {
        expr: Expr::If {
            cond: Box::new(bc("=", vec![a, b])),
            then_branch: Box::new(Block {
                stmts: vec![puts(strlit(then))],
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            else_branch: Box::new(Block {
                stmts: vec![puts(strlit(els))],
                value: Expr::NilLit { span: s() },
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    };
    let m = map_module(vec![
        // equal entries, same order → same
        if_eq(
            maplit(vec![(ilit(1), ilit(2)), (ilit(3), ilit(4))]),
            maplit(vec![(ilit(1), ilit(2)), (ilit(3), ilit(4))]),
            "same",
            "diff",
        ),
        // a differing value → diff
        if_eq(
            maplit(vec![(ilit(1), ilit(2))]),
            maplit(vec![(ilit(1), ilit(9))]),
            "same",
            "diff",
        ),
        // different length → diff
        if_eq(
            maplit(vec![(ilit(1), ilit(2))]),
            maplit(vec![(ilit(1), ilit(2)), (ilit(3), ilit(4))]),
            "same",
            "diff",
        ),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "same\ndiff\ndiff\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn for_each_over_a_map_does_not_panic() {
    // `ForEach` becomes reachable once `Loops` is accepted, and its iterable may
    // now be a map. Iterating a map is REFERENCE-UNDEFINED (Go's `_sir_seq_iter`
    // panics on it); C's lenient `_sir_seq_iter` else-branch treats a non-seq /
    // non-cons as an empty iteration — so the loop body runs zero times and the
    // emitter stays total (no `unreachable!`). The guard printed before and
    // after must both appear, with nothing from the body between them.
    let m = map_module(vec![
        puts(strlit("before")),
        Stmt::ForEach {
            var: "kv".into(),
            iter: maplit(vec![(ilit(1), ilit(2))]),
            body: Block {
                stmts: vec![puts(strlit("body"))],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        },
        puts(strlit("after")),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "before\nafter\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn cyclic_map_does_not_stack_overflow() {
    // `MapSet` mutates in place, so a map can be made self-referential
    // (`h[0] = h`) — a newly-reachable cycle. Equality and display must
    // TERMINATE (via the `SeqSet`-era depth caps), not crash the stack. `run`
    // asserts the process exits 0; a stack overflow would be a non-zero exit.
    let nil = || Expr::NilLit { span: s() };
    let m = map_module(vec![
        // h = {0 => nil}; h[0] = h  → a self-referential map
        let_("h", maplit(vec![(ilit(0), nil())])),
        Stmt::MapSet { map: local("h"), key: ilit(0), value: local("h"), span: s() },
        // h == h → #t via the identical-handle fast path (no walk)
        puts(bc("=", vec![local("h"), local("h")])),
        // print(h) → terminates via the fmt depth cap (bounded string)
        puts(local("h")),
        // two DISTINCT mutually-cyclic maps: h[0]=g, g[0]=h
        let_("g", maplit(vec![(ilit(0), nil())])),
        Stmt::MapSet { map: local("g"), key: ilit(0), value: local("h"), span: s() },
        Stmt::MapSet { map: local("h"), key: ilit(0), value: local("g"), span: s() },
        // h == g → terminates via the eq depth cap (result is cap-defined; the
        // point is it does not stack-overflow)
        puts(bc("=", vec![local("h"), local("g")])),
    ]);
    match run(&m) {
        Some(out) => {
            let lines: Vec<&str> = out.lines().collect();
            assert_eq!(lines.first().copied(), Some("#t"), "h == h is true (fast path)");
            assert_eq!(lines.len(), 3, "all three puts ran — no stack overflow");
        }
        None => eprintln!("skip: no cc"),
    }
}
