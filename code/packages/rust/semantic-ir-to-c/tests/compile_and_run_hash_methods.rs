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
