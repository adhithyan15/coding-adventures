//! Execution proof for Collections slice 3 (0-arg Array query/transform
//! methods) on the C backend — lower REAL Ruby source, emit C, compile with a
//! real cc, run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! `[..].method` lowers to `__method__(recv, "method")`; when `method` is a
//! built-in name (not a user-defined method) it routes to the runtime
//! `_sir_builtin_method` dispatcher, which type-checks the receiver and
//! applies the Array implementation — a FRESH array/scalar, never mutating
//! the receiver.

use std::process::Command;

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    ["cc", "clang", "gcc"]
        .iter()
        .find(|c| {
            Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
}

fn run_ruby(src: &str) -> Option<String> {
    let cc = find_cc()?;
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    let art = semantic_ir_to_c::compile(&module).expect("C compile (no panic)");
    let dir = std::env::temp_dir();
    // Hash the full source (not just its length — the sibling test files' `len()`
    // scheme collides whenever two same-length sources run as parallel test
    // threads in the same process, each clobbering the other's temp file).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_arrm_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        art.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "program exited non-zero");
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

#[test]
fn array_count_first_last() {
    match run_ruby("puts [10, 20, 30].count\nputs [10, 20, 30].first\nputs [10, 20, 30].last\n") {
        Some(out) => assert_eq!(out, "3\n10\n30\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_first_last_on_empty_is_nil() {
    // Ruby's `[].first`/`[].last` return nil, not raise; this backend's `puts`
    // prints nil as the literal text `nil` (see `compile_and_run_sequences.rs`'s
    // own OOB-index case for the same convention).
    match run_ruby("puts [].first\nputs [].last\n") {
        Some(out) => assert_eq!(out, "nil\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_reverse_and_sort() {
    match run_ruby("puts [1, 2, 3].reverse\nputs [3, 1, 2].sort\n") {
        Some(out) => assert_eq!(out, "[3, 2, 1]\n[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_reverse_does_not_mutate_the_receiver() {
    // `reverse` (unlike a hypothetical `reverse!`) returns a FRESH array; the
    // original binding must still print in its original order afterward.
    match run_ruby("a = [1, 2, 3]\na.reverse\nputs a\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_sort_strings() {
    // `sort` reuses `_sir_lt`, which has a String branch (`strcmp`), so a
    // String array sorts lexicographically, not just numeric arrays.
    match run_ruby("puts [\"banana\", \"apple\", \"cherry\"].sort\n") {
        Some(out) => assert_eq!(out, "[apple, banana, cherry]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_min_max_sum() {
    match run_ruby("puts [5, 1, 9, 3].min\nputs [5, 1, 9, 3].max\nputs [1, 2, 3, 4].sum\n") {
        Some(out) => assert_eq!(out, "1\n9\n10\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_min_max_on_empty_is_nil_sum_is_zero() {
    match run_ruby("puts [].min\nputs [].max\nputs [].sum\n") {
        Some(out) => assert_eq!(out, "nil\nnil\n0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_sum_promotes_to_float() {
    // Mirrors `+`'s own int/float promotion (`_sir_plus_v`, reused by `sum`).
    match run_ruby("puts [1, 2.5].sum\n") {
        Some(out) => assert_eq!(out, "3.5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_uniq_preserves_first_occurrence_order() {
    match run_ruby("puts [1, 2, 1, 3, 2].uniq\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_compact_drops_nils() {
    match run_ruby("puts [1, nil, 2, nil, 3].compact\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_flatten_is_fully_recursive() {
    match run_ruby("puts [1, [2, [3, 4], 5], 6].flatten\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3, 4, 5, 6]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_to_a_is_the_array_itself() {
    match run_ruby("puts [1, 2, 3].to_a\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_methods_chain() {
    // `sort` returns an Array, so another built-in method dispatches on it.
    match run_ruby("puts [3, 1, 2].sort.reverse\n") {
        Some(out) => assert_eq!(out, "[3, 2, 1]\n"),
        None => eprintln!("skip: no cc"),
    }
}

// The Ruby frontend has no source syntax for an index-assignment TARGET like
// `a[0] = a` yet (only `SeqSet` built by hand reaches it — see the sibling
// `compile_and_run_sequences.rs`'s own `cyclic_sequence_does_not_stack_overflow`,
// which hand-builds for the same reason), so the cyclic-flatten proof below
// hand-builds a `semantic_ir::Module` directly rather than going through
// `run_ruby`.
mod cyclic {
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use semantic_ir::{
        Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span,
        Stmt, CURRENT_SIR_VERSION,
    };

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn run(module: &Module) -> Option<String> {
        let cc = super::find_cc()?;
        let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
        let dir = std::env::temp_dir();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("sirc_arrm_cyc_{}_{}", std::process::id(), n);
        let cpath: PathBuf = dir.join(format!("{stem}.c"));
        let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&cpath, &artifact.source).expect("write .c");
        let out = Command::new(&cc)
            .args(["-std=c99", "-Wall", "-o"])
            .arg(&exe)
            .arg(&cpath)
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

    /// A `main` module: `a = [1, 2]`, then `sets` (each an index to point back
    /// at `a` itself), then `a.flatten`, then `puts "ok"`. Shared by both the
    /// linear (one self-pointing element) and branching (every element
    /// self-pointing) cyclic-flatten proofs below.
    fn cyclic_flatten_module(sets: &[i64]) -> Module {
        let mut stmts = vec![let_("a", Expr::SeqLit { items: vec![ilit(1), ilit(2)], span: s() })];
        for &i in sets {
            stmts.push(Stmt::SeqSet { seq: local("a"), index: ilit(i), value: local("a"), span: s() });
        }
        stmts.push(let_(
            "b",
            bc("__method__", vec![local("a"), Expr::StrLit { value: "flatten".into(), span: s() }]),
        ));
        stmts.push(puts(Expr::StrLit { value: "ok".into(), span: s() }));
        Module {
            name: "arrcycprog".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::Sequences,
                Feature::Strings,
                Feature::MutableBindings,
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
    fn array_flatten_on_a_linearly_cyclic_array_terminates() {
        // `a = [1, 2]; a[0] = a` — ONE element points back to `a`, the other
        // (`2`) is a plain leaf. `flatten`'s recursion has a fixed fan-out of
        // 1 down the cyclic branch, so this is bounded by the DEPTH cap alone.
        // The exact shape past the cap is unspecified, so `flatten`'s result
        // is computed but deliberately not printed — only that the process
        // reaches and prints past it (exit 0) is asserted.
        match run(&cyclic_flatten_module(&[0])) {
            Some(out) => assert_eq!(out, "ok\n"),
            None => eprintln!("skip: no cc"),
        }
    }

    #[test]
    fn array_flatten_on_a_branching_cyclic_array_terminates() {
        // `a = [1, 2]; a[0] = a; a[1] = a` — BOTH elements point back to `a`,
        // so naive recursion fans out ~2^depth calls; with the depth cap at
        // 500 that is astronomically more work than a depth-only guard could
        // ever finish (and would overflow the `int64_t` element count well
        // before the cap, under-allocating the output buffer). This is
        // exactly the gap a depth-only cap leaves open — `flatten` must ALSO
        // cap total nodes visited, independent of depth, for this to
        // terminate promptly. Asserts only that the process completes
        // (exit 0) in reasonable time; the flattened shape past the budget
        // is unspecified.
        match run(&cyclic_flatten_module(&[0, 1])) {
            Some(out) => assert_eq!(out, "ok\n"),
            None => eprintln!("skip: no cc"),
        }
    }
}
