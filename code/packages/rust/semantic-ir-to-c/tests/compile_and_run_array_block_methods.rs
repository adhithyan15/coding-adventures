//! Execution proof for Collections slice 5 (Array block methods) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! `[..].method { |x| .. }` lowers to `__method__(recv, "method", MakeClosure)`
//! — the block is an ordinary trailing `SIR_CLOSURE` argument, invoked through
//! the existing `_sir_apply`, so a block-taking built-in behaves exactly like
//! any other closure call.

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
    let stem = format!("sirc_arrblk_{}_{}", std::process::id(), hasher.finish());
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
fn each_runs_the_block_and_returns_the_receiver() {
    match run_ruby("puts [1, 2, 3].each { |x| puts x * 10 }\n") {
        Some(out) => assert_eq!(out, "10\n20\n30\n[1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn map_transforms_every_element() {
    match run_ruby("puts [1, 2, 3].map { |x| x * x }\n") {
        Some(out) => assert_eq!(out, "[1, 4, 9]\n"),
        None => eprintln!("skip: no cc"),
    }
}

// NOTE: these predicates read `r = x > N\nr` (assign-then-return) instead of
// the more natural bare `x > N`. That bare form currently fails to lower at
// ALL for `<`/`>`/`<=`/`>=`/`!=` when it is the TAIL expression of a block --
// a pre-existing `ruby-to-semantic-ir`/shared-validator bug independent of
// this slice (confirmed: the same operators work fine at top level, only
// inside a block's implicit-return position do they mis-lower to "direct call
// to unknown function `x`"; filed as its own follow-up, not fixed here since
// it's outside the Collections/C-backend surface this PR touches). The
// assign-then-return shape sidesteps the bug without changing test intent.

#[test]
fn select_and_reject_are_complementary() {
    match run_ruby(
        "puts [1, 2, 3, 4, 5].select { |x| r = x > 2\nr }\n\
         puts [1, 2, 3, 4, 5].reject { |x| r = x > 2\nr }\n",
    ) {
        Some(out) => assert_eq!(out, "[3, 4, 5]\n[1, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn any_all_none_predicates() {
    match run_ruby(
        "puts [1, 2, 3].any? { |x| r = x > 2\nr }\n\
         puts [1, 2, 3].all? { |x| r = x > 2\nr }\n\
         puts [1, 2, 3].none? { |x| r = x > 5\nr }\n",
    ) {
        Some(out) => assert_eq!(out, "#t\n#f\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn sort_by_orders_by_the_computed_key() {
    // Sort strings by length, not lexicographically.
    match run_ruby("puts [\"ccc\", \"a\", \"bb\"].sort_by { |x| x.length }\n") {
        Some(out) => assert_eq!(out, "[a, bb, ccc]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn each_with_index_passes_both_element_and_index() {
    match run_ruby("[10, 20, 30].each_with_index { |x, i| puts x + i }\n") {
        Some(out) => assert_eq!(out, "10\n21\n32\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reduce_with_initial_value() {
    match run_ruby("puts [1, 2, 3, 4].reduce(0) { |acc, x| acc + x }\n") {
        Some(out) => assert_eq!(out, "10\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reduce_without_initial_value_seeds_from_the_first_element() {
    match run_ruby("puts [1, 2, 3, 4].reduce { |acc, x| acc + x }\n") {
        Some(out) => assert_eq!(out, "10\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn inject_is_an_alias_for_reduce() {
    match run_ruby("puts [2, 3, 4].inject(1) { |acc, x| acc * x }\n") {
        Some(out) => assert_eq!(out, "24\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn count_with_a_block_counts_only_matching_elements() {
    // Regression for the slice-3 `count` gap: the 0-arg form still returns the
    // total length, but the block form must count only matches, not shadow it.
    // (`x.even?` isn't lowered yet -- Numeric methods are a later slice -- and
    // `x % 2 == 0` hits a SEPARATE variant of the block-tail-position bug
    // (fails on `%` itself, not just the comparison) -- so this uses `x > 3`,
    // same as the other predicates above.)
    match run_ruby(
        "puts [1, 2, 3, 4, 5].count\nputs [1, 2, 3, 4, 5].count { |x| r = x > 3\nr }\n",
    ) {
        Some(out) => assert_eq!(out, "5\n2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn block_methods_chain_and_close_over_an_outer_local() {
    // `map` returns an Array, so another built-in dispatches on it; the block
    // captures the enclosing local `factor` (RB2 capture).
    match run_ruby(
        "factor = 10\nputs [1, 2, 3].map { |x| x * factor }.select { |x| r = x > 15\nr }\n",
    ) {
        Some(out) => assert_eq!(out, "[20, 30]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reduce_on_empty_with_no_initial_is_nil() {
    match run_ruby("puts [].reduce { |acc, x| acc + x }\n") {
        Some(out) => assert_eq!(out, "nil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reduce_on_empty_with_initial_returns_it_untouched() {
    match run_ruby("puts [].reduce(42) { |acc, x| acc + x }\n") {
        Some(out) => assert_eq!(out, "42\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn sum_with_block_transforms_before_summing() {
    // Regression: the 0-arg `sum` arm (slice 3) ignored argc/args entirely,
    // so `arr.sum { |x| .. }` silently summed the RAW elements instead of
    // the block's transformed values -- the same latent-shadowing shape the
    // slice-3 `count` gap had before it was fixed in slice 5.
    match run_ruby("puts [1, 2, 3].sum\nputs [1, 2, 3].sum { |x| x * 2 }\n") {
        Some(out) => assert_eq!(out, "6\n12\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn sum_with_block_on_empty_is_zero() {
    match run_ruby("puts [].sum { |x| x * 2 }\n") {
        Some(out) => assert_eq!(out, "0\n"),
        None => eprintln!("skip: no cc"),
    }
}
