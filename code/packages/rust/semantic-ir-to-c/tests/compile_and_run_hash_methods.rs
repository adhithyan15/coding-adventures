//! Execution proof for Collections slice 6 (Hash non-block methods) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.

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
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_hashm_{}_{}", std::process::id(), hasher.finish());
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
fn hash_keys_and_values_in_insertion_order() {
    match run_ruby("h = {1 => \"a\", 2 => \"b\", 3 => \"c\"}\nputs h.keys\nputs h.values\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n[a, b, c]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_fetch_present_and_missing_raises() {
    match run_ruby(
        "h = {1 => 10}\nputs h.fetch(1)\nbegin\n  h.fetch(9)\nrescue KeyError => e\n  puts \"caught\"\nend\n",
    ) {
        Some(out) => assert_eq!(out, "10\ncaught\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_to_a_and_to_h() {
    match run_ruby("h = {1 => 10, 2 => 20}\nputs h.to_a\nputs h.to_h.keys\n") {
        Some(out) => assert_eq!(out, "[[1, 10], [2, 20]]\n[1, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_dig_single_and_nested() {
    match run_ruby(
        "h = {1 => {2 => \"deep\"}}\nputs h.dig(1, 2)\nputs h.dig(1)\nputs h.dig(9, 2)\nputs h.dig(9)\n",
    ) {
        Some(out) => assert_eq!(out, "deep\n{2: deep}\nnil\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_merge_prefers_other_and_does_not_mutate_receiver() {
    match run_ruby(
        "a = {1 => \"a\", 2 => \"b\"}\nb = {2 => \"B\", 3 => \"c\"}\nputs a.merge(b)\nputs a\n",
    ) {
        Some(out) => assert_eq!(out, "{1: a, 2: B, 3: c}\n{1: a, 2: b}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_delete_removes_and_returns_the_value() {
    match run_ruby("h = {1 => \"a\", 2 => \"b\", 3 => \"c\"}\nputs h.delete(2)\nputs h\nputs h.delete(9)\n") {
        Some(out) => assert_eq!(out, "b\n{1: a, 3: c}\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_delete_mutates_a_shared_binding() {
    match run_ruby("a = {1 => \"a\"}\nb = a\na.delete(1)\nputs b.keys\n") {
        Some(out) => assert_eq!(out, "[]\n"),
        None => eprintln!("skip: no cc"),
    }
}

// The Ruby frontend has no source syntax for a bracket-index assignment
// TARGET (`h[k] = v`) yet — confirmed while writing this test ("Unexpected
// token: =") — so the regression below hand-builds a `semantic_ir::Module`
// directly (`Stmt::MapSet`), the same workaround the pre-existing
// cyclic-array/cyclic-map tests already use for an unrelated reason.
mod insert_after_delete {
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use semantic_ir::{
        Block, EffectSet, Expr, Feature, FeatureManifest, Function, MapEntry, Metadata, Module,
        Scope, Span, Stmt, CURRENT_SIR_VERSION,
    };

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn run(module: &Module) -> Option<String> {
        let cc = super::find_cc()?;
        let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
        let dir = std::env::temp_dir();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("sirc_hashm_delcap_{}_{}", std::process::id(), n);
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
    fn slit(v: &str) -> Expr {
        Expr::StrLit { value: v.into(), span: s() }
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
    fn delete_call(key: i64) -> Stmt {
        Stmt::ExprStmt {
            expr: bc("__method__", vec![local("h"), slit("delete"), ilit(key)]),
            span: s(),
        }
    }
    fn map_set(key: i64, value: &str) -> Stmt {
        Stmt::MapSet { map: local("h"), key: ilit(key), value: slit(value), span: s() }
    }

    #[test]
    fn hash_insert_after_delete_does_not_overflow_the_reallocated_buffer() {
        // Security regression: `delete` reallocates a fresh, tightly-sized
        // buffer (see its own runtime.rs comment) but an early draft forgot
        // to sync `m->cap` to the new (smaller) size, leaving
        // `_sir_map_put`'s `len == cap` grow-check desynced -- a later
        // `h[k] = v` would see `len < cap`, skip growing, and write one past
        // the end of the tightly-sized buffer (a heap out-of-bounds write).
        // Reachable by entirely ordinary code: delete a key, then assign a
        // new one. Repeated twice (delete+insert, delete+insert again) to
        // also exercise the grow-triggering `len == cap` path on the SECOND
        // insert.
        let m = Module {
            name: "hashdelcapprog".into(),
            manifest: FeatureManifest::from_features(&[Feature::Maps, Feature::Strings]),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![
                        let_(
                            "h",
                            Expr::MapLit {
                                entries: vec![
                                    MapEntry { key: ilit(1), value: slit("a") },
                                    MapEntry { key: ilit(2), value: slit("b") },
                                    MapEntry { key: ilit(3), value: slit("c") },
                                ],
                                span: s(),
                            },
                        ),
                        delete_call(1),
                        map_set(4, "d"),
                        delete_call(2),
                        map_set(5, "e"),
                        puts(local("h")),
                    ],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
            span: s(),
        };
        match run(&m) {
            Some(out) => assert_eq!(out, "{3: c, 4: d, 5: e}\n"),
            None => eprintln!("skip: no cc"),
        }
    }
}

#[test]
fn hash_clear_empties_and_returns_the_receiver() {
    match run_ruby("h = {1 => \"a\", 2 => \"b\"}\nputs h.clear.keys\nputs h.empty?\n") {
        Some(out) => assert_eq!(out, "[]\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_invert_swaps_keys_and_values() {
    match run_ruby("h = {1 => \"a\", 2 => \"b\"}\nputs h.invert\n") {
        Some(out) => assert_eq!(out, "{a: 1, b: 2}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_invert_duplicate_values_last_one_wins() {
    match run_ruby("h = {1 => \"x\", 2 => \"x\"}\nputs h.invert\n") {
        Some(out) => assert_eq!(out, "{x: 2}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_dig_reuses_the_same_polymorphic_helper() {
    match run_ruby("a = [[1, 2], [3, 4]]\nputs a.dig(1, 0)\n") {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}
