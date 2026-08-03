//! Execution proof for Collections slice 4 (Array mutation + 1-arg query
//! methods) on the C backend — lower REAL Ruby source, emit C, compile with a
//! real cc, run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! `push`/`pop`/`shift` are the FIRST Array methods that mutate the receiver
//! (grow/shrink `SirSeq.len`/`.items`) after construction; `fetch`/
//! `values_at`/`rotate`/`zip` are non-mutating 1-arg queries, and
//! `include?`/`index` widen their slice-2 String-only forms to accept an
//! Array receiver too.

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
    // Hash the full source, not just its length -- two tests with equal-length
    // sources run as parallel threads in the SAME process and would collide on
    // a length-keyed stem (see the temp-file-collision fix elsewhere in this
    // test suite).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_arrmut_{}_{}", std::process::id(), hasher.finish());
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
fn push_appends_one_or_more_and_returns_the_receiver() {
    match run_ruby("a = [1, 2]\nputs a.push(3)\nputs a\na.push(4, 5)\nputs a\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n[1, 2, 3]\n[1, 2, 3, 4, 5]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn push_mutates_a_shared_binding() {
    // Every binding sharing the array (not just the one `push` was called
    // through) sees the appended element -- the same shared-box semantics
    // `SeqSet` already has.
    match run_ruby("a = [1]\nb = a\na.push(2)\nputs b\n") {
        Some(out) => assert_eq!(out, "[1, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn pop_removes_and_returns_the_last_element() {
    match run_ruby("a = [1, 2, 3]\nputs a.pop\nputs a\n") {
        Some(out) => assert_eq!(out, "3\n[1, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn pop_on_empty_is_nil() {
    match run_ruby("puts [].pop\n") {
        Some(out) => assert_eq!(out, "nil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn shift_removes_and_returns_the_first_element_and_shifts_the_rest() {
    match run_ruby("a = [1, 2, 3]\nputs a.shift\nputs a\n") {
        Some(out) => assert_eq!(out, "1\n[2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn shift_on_empty_is_nil() {
    match run_ruby("puts [].shift\n") {
        Some(out) => assert_eq!(out, "nil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn push_pop_shift_interleaved() {
    match run_ruby(
        "a = [1, 2]\na.push(3)\nputs a.shift\na.push(4)\nputs a.pop\nputs a\n",
    ) {
        Some(out) => assert_eq!(out, "1\n4\n[2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_include_and_index() {
    match run_ruby(
        "puts [1, 2, 3].include?(2)\nputs [1, 2, 3].include?(9)\nputs [1, 2, 3].index(2)\nputs [1, 2, 3].index(9)\n",
    ) {
        Some(out) => assert_eq!(out, "#t\n#f\n1\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_fetch_in_range_and_out_of_range_raises() {
    match run_ruby(
        "puts [10, 20, 30].fetch(1)\nbegin\n  [10, 20, 30].fetch(9)\nrescue IndexError => e\n  puts \"caught\"\nend\n",
    ) {
        Some(out) => assert_eq!(out, "20\ncaught\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_fetch_negative_index_counts_from_the_end() {
    match run_ruby("puts [10, 20, 30].fetch(-1)\n") {
        Some(out) => assert_eq!(out, "30\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_values_at_multiple_indices() {
    match run_ruby("puts [10, 20, 30, 40].values_at(0, 2, 9, -1)\n") {
        Some(out) => assert_eq!(out, "[10, 30, nil, 40]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_rotate_default_and_explicit_and_negative() {
    match run_ruby(
        "puts [1, 2, 3, 4].rotate\nputs [1, 2, 3, 4].rotate(2)\nputs [1, 2, 3, 4].rotate(-1)\n",
    ) {
        Some(out) => assert_eq!(out, "[2, 3, 4, 1]\n[3, 4, 1, 2]\n[4, 1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_rotate_does_not_mutate_the_receiver() {
    match run_ruby("a = [1, 2, 3]\na.rotate\nputs a\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_zip_pads_shorter_others_with_nil() {
    match run_ruby("puts [1, 2, 3].zip([4, 5])\n") {
        Some(out) => assert_eq!(out, "[[1, 4], [2, 5], [3, nil]]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_zip_multiple_others() {
    match run_ruby("puts [1, 2].zip([10, 20], [100, 200])\n") {
        Some(out) => assert_eq!(out, "[[1, 10, 100], [2, 20, 200]]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn each_pushing_to_the_receiver_terminates_and_does_not_see_pushed_elements() {
    // Security regression: `each`'s loop bound is snapshotted BEFORE any
    // block call, so a block that grows the very array being iterated (1)
    // does not run unbounded, and (2) does not read/write past the
    // originally-allocated output where applicable. `each` has no output
    // buffer, so this mainly proves termination; `array_map_pushing_to_the_
    // receiver_does_not_overrun_its_output_buffer` below proves the
    // allocation-safety half.
    match run_ruby("a = [1, 2, 3]\na.each { |x| a.push(x) }\nputs a.length\n") {
        Some(out) => assert_eq!(out, "6\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_map_pushing_to_the_receiver_does_not_overrun_its_output_buffer() {
    // The exact scenario the security review flagged: `map` pre-allocates
    // its output sized to the receiver's length; if the block grows the
    // SAME receiver mid-loop and `map`'s loop bound re-read that (now
    // larger) length, it would write past the allocation. `map`'s output
    // must reflect only the ORIGINAL 3 elements, and the process must not
    // crash (a heap overflow would corrupt or crash under most allocators).
    match run_ruby("a = [1, 2, 3]\nb = a.map { |x| a.push(x)\nx * 10 }\nputs b\n") {
        Some(out) => assert_eq!(out, "[10, 20, 30]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn count_with_block_pushing_to_the_receiver_terminates() {
    // Security regression: an earlier draft of the slice-4 safety retrofit
    // touched every slice-3/5 helper but MISSED `count`'s block form (added
    // in slice 5) -- caught by security review. `count`'s block-form loop
    // must snapshot len/items BEFORE the loop too, or a block that pushes to
    // its own receiver never terminates.
    match run_ruby("a = [1, 2, 3]\nputs a.count { |x| a.push(x)\ntrue }\n") {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_each_shrinking_then_growing_the_receiver_does_not_overread() {
    // Security regression: snapshotting `len` ALONE is not enough -- `push`
    // reallocates its new buffer sized to the CURRENT (live) `s->len`, so a
    // block that first shrinks the receiver (`pop`, len-only, no
    // reallocation) and THEN pushes produces a buffer smaller than a `len`
    // already snapshotted by an outer iterating helper. Continuing to read
    // via a live `s->items` pointer up to that stale, larger snapshot would
    // then run past the fresh (smaller) allocation -- caught by security
    // review. The fix snapshots the ITEMS POINTER too, not just the length,
    // so this must not crash, and `x` must reflect the ORIGINAL 5-element
    // buffer (`each`'s own snapshot at entry), never the shrunk/regrown one.
    match run_ruby("a = [1, 2, 3, 4, 5]\na.each { |x| a.pop\na.pop\na.push(99)\nputs x }\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn each_shifting_the_receiver_does_not_corrupt_an_in_flight_snapshot() {
    // Security regression (found while implementing Hash#delete, which has
    // the identical shape): `shift`'s ORIGINAL implementation compacted
    // `s->items` IN PLACE (no reallocation) -- but a block-taking helper
    // snapshots the `items` POINTER, not a copy of the data, before its
    // loop. Snapshotting a pointer is only safe against a mutator that
    // REALLOCATES (like `push`): the snapshot and the mutator's new buffer
    // are then different memory. An in-place compact instead mutates the
    // SAME memory the snapshot points into, silently corrupting an
    // in-flight outer iteration -- elements shift under it, some get read
    // twice, some get skipped. `shift` now reallocates too, so `each` must
    // still see the ORIGINAL 3 elements here, each exactly once.
    match run_ruby("a = [1, 2, 3]\na.each { |x| a.shift\nputs x }\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}
