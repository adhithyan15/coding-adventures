//! Execution proof for Collections slice 7 (Hash block methods) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! A bare comparison used as a block's TAIL expression (e.g. `{ |k,v| v > 1
//! }`) currently fails to lower for every operator except `==` -- a
//! pre-existing `ruby-to-semantic-ir` frontend bug found and worked around
//! the same way in the Array block-methods tests (assign-then-return:
//! `{ |k,v| r = v > 1\nr }`), not something this slice touches.

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
    let stem = format!("sirc_hashblk_{}_{}", std::process::id(), hasher.finish());
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
fn hash_each_yields_key_and_value_and_returns_the_receiver() {
    // (String interpolation isn't accepted by the C backend yet -- a
    // separate, pre-existing gap -- so key/value print on their own lines.)
    match run_ruby("h = {1 => \"a\", 2 => \"b\"}\nputs h.each { |k, v| puts k\nputs v }.keys\n") {
        Some(out) => assert_eq!(out, "1\na\n2\nb\n[1, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_each_key_and_each_value() {
    match run_ruby(
        "h = {1 => \"a\", 2 => \"b\"}\nh.each_key { |k| puts k }\nh.each_value { |v| puts v }\n",
    ) {
        Some(out) => assert_eq!(out, "1\n2\na\nb\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_map_returns_an_array_not_a_hash() {
    match run_ruby("h = {1 => 10, 2 => 20}\nputs h.map { |k, v| k + v }\n") {
        Some(out) => assert_eq!(out, "[11, 22]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_select_and_reject_return_hashes() {
    match run_ruby(
        "h = {1 => 10, 2 => 20, 3 => 30}\nputs h.select { |k, v| r = v > 15\nr }\nputs h.reject { |k, v| r = v > 15\nr }\n",
    ) {
        Some(out) => assert_eq!(out, "{2: 20, 3: 30}\n{1: 10}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_sort_by_returns_an_array_of_pairs() {
    match run_ruby("h = {3 => \"c\", 1 => \"a\", 2 => \"b\"}\nputs h.sort_by { |k, v| k }\n") {
        Some(out) => assert_eq!(out, "[[1, a], [2, b], [3, c]]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_group_by_groups_pairs_by_the_block_result() {
    // (Bracket-index READ, `g[true]`, only parses as a bare assignment RHS --
    // a separate, pre-existing frontend gap -- so `.fetch` is used instead.)
    match run_ruby(
        "h = {1 => \"a\", 2 => \"b\", 3 => \"c\", 4 => \"d\"}\ng = h.group_by { |k, v| r = k > 2\nr }\nputs g.fetch(true)\nputs g.fetch(false)\n",
    ) {
        Some(out) => assert_eq!(out, "[[3, c], [4, d]]\n[[1, a], [2, b]]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_partition_splits_into_matching_and_non_matching() {
    match run_ruby(
        "h = {1 => \"a\", 2 => \"b\", 3 => \"c\"}\np = h.partition { |k, v| r = k > 1\nr }\nputs p\n",
    ) {
        Some(out) => assert_eq!(out, "[[[2, b], [3, c]], [[1, a]]]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_sum_with_block() {
    match run_ruby("h = {1 => 10, 2 => 20, 3 => 30}\nputs h.sum { |k, v| k + v }\n") {
        Some(out) => assert_eq!(out, "66\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_block_methods_chain() {
    // `map` returns an Array, so an Array built-in dispatches on it.
    match run_ruby("h = {1 => 10, 2 => 20}\nputs h.map { |k, v| v }.sort.reverse\n") {
        Some(out) => assert_eq!(out, "[20, 10]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn each_deleting_from_the_receiver_terminates_and_does_not_overread() {
    // Security regression: `each`'s len/entries snapshot is taken BEFORE the
    // loop, so a block that deletes from the SAME map being iterated (1)
    // does not observe the shrink mid-iteration, and (2) does not read past
    // the entries buffer as `delete` shifts entries down. `each` must still
    // see the ORIGINAL 3 keys/values.
    match run_ruby(
        "h = {1 => \"a\", 2 => \"b\", 3 => \"c\"}\nh.each { |k, v| h.delete(k)\nputs k\nputs v }\nputs h.keys\n",
    ) {
        Some(out) => assert_eq!(out, "1\na\n2\nb\n3\nc\n[]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_clearing_the_receiver_does_not_overrun_its_output_buffer() {
    // The Hash analogue of the Array map/push security regression: `map`
    // pre-allocates its output sized to the receiver's length; if the block
    // clears the SAME receiver mid-loop, `map`'s snapshot must still see the
    // ORIGINAL 3 entries (not 0), and the process must not crash.
    match run_ruby("h = {1 => 10, 2 => 20, 3 => 30}\nb = h.map { |k, v| h.clear\nv }\nputs b\n") {
        Some(out) => assert_eq!(out, "[10, 20, 30]\n"),
        None => eprintln!("skip: no cc"),
    }
}
