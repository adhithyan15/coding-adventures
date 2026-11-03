//! #18798 (re-review of the for → while rewrite): integer comparisons are
//! exact on the C backend.
//!
//! `_sir_lt`/`_sir_gt`/`_sir_le`/`_sir_ge` compared every number as a
//! `double`, which holds integers exactly only up to 2^53.  A `for` over a
//! range that can `break` or `return` is lowered (`ruby-to-semantic-ir`'s
//! `jumps` module) to a `while` whose condition is such a comparison, where a
//! `ForRange` used to count in a native `int64_t`, so ranges past 2^53
//! miscounted on C alone:
//!
//! | range                                      | ruby | C before |
//! |--------------------------------------------|------|----------|
//! | `9007199254740993...9007199254740996`      | 3 trips | 2     |
//! | `9223372036854775805..9223372036854775806` | 2 trips | 0     |
//!
//! Two Integers now compare as `int64_t`; a Float on either side still
//! compares as `double`.  The expected outputs are ruby 3.3.6's.  (These run
//! on C only: JavaScript numbers are doubles, so a range past 2^53 cannot
//! count there at all — a separate, pre-existing limit.)

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
    let scratch = sir_test_scratch::Scratch::new("sir_c_test_");
    let dir = scratch.path();
    // Hash the full source (not just its length — the sibling test files' `len()`
    // scheme collides whenever two same-length sources run as parallel test
    // threads in the same process, each clobbering the other's temp file).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    // The hash alone still collides when two tests compile the SAME
    // source, so the stem also carries a per-call `unique_suffix()` (#18485).
    let stem = format!("sirc_intcmp_{}_{}", unique_suffix(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    let _cleanup = RemoveOnDrop([cpath.clone(), exe.clone()]);
    sir_test_scratch::write_new(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
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
fn a_for_that_can_break_counts_exactly_past_2_pow_53() {
    let src = "n = 0\nfor z in 9007199254740993...9007199254740996\n  break if z == 0\n  n += 1\nend\nputs n\nputs z\nm = 0\nfor w in 9223372036854775805..9223372036854775806\n  break if w == 0\n  m += 1\nend\nputs m\nputs w\n";
    match run_ruby(src) {
        Some(out) => assert_eq!(out, "3\n9007199254740995\n2\n9223372036854775806\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn integer_comparisons_are_exact_and_float_ones_unchanged() {
    // Each pair differs by 1 past 2^53, where a `double` cannot tell them
    // apart; a Float operand still compares as a `double`, as in Ruby.
    let src = "a = 9007199254740993\nb = 9007199254740992\nputs a > b\nputs b < a\nputs a >= b\nputs a <= b\nputs 9223372036854775807 > 9223372036854775806\nputs 1 < 1.5\nputs 2.5 >= 2\nputs -3 < -2\n";
    match run_ruby(src) {
        Some(out) => assert_eq!(out, "true\ntrue\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\n"),
        None => eprintln!("skip: no cc"),
    }
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!("{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Deletes a run's `.c` file and executable when the helper returns or
/// panics.  Each call names its files uniquely (see `unique_suffix` or the
/// counter above), so without this they would pile up in the temp directory,
/// one pair per test run.
struct RemoveOnDrop([std::path::PathBuf; 2]);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}
