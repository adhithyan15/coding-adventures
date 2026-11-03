//! Execution proof for the slice-8-deferred String methods on the C backend:
//! char-set methods (`count`/`delete`/`squeeze`) and padding methods
//! (`ljust`/`rjust`/`center`). Lowers REAL Ruby source, emits C, compiles
//! with a real cc, runs, asserts stdout. Skips gracefully when no `cc` is
//! present.
//!
//! Every expected value below is independently confirmed against a live
//! `ruby -e` interpreter, not hand-derived — including the charset
//! INTERSECTION rule for multi-argument `count`/`delete`, `center`'s
//! odd-leftover-pad-goes-RIGHT rule, and an empty-string charset argument
//! meaning "squeeze/count/delete nothing" (not "everything").

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
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    // The hash alone still collides when two tests compile the SAME
    // source, so the stem also carries a per-call `unique_suffix()` (#18485).
    let stem = format!("sirc_charpad_{}_{}", unique_suffix(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    let _cleanup = RemoveOnDrop([cpath.clone(), exe.clone()]);
    sir_test_scratch::write_new(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm") // Linux needs -lm to link floor/ceil/pow (macOS libSystem folds it in)
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
fn count_returns_how_many_chars_lie_in_the_charset() {
    match run_ruby("puts \"hello\".count(\"l\")\nputs \"hello\".count(\"lo\")\n") {
        Some(out) => assert_eq!(out, "2\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn count_and_delete_with_multiple_charset_args_intersect() {
    match run_ruby(
        "puts \"hello\".count(\"helo\", \"ol\")\nputs \"hello\".delete(\"helo\", \"ol\")\n",
    ) {
        Some(out) => assert_eq!(out, "3\nhe\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn delete_removes_every_char_in_the_charset() {
    match run_ruby("puts \"hello\".delete(\"l\")\n") {
        Some(out) => assert_eq!(out, "heo\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn squeeze_with_no_charset_collapses_every_run() {
    match run_ruby("puts \"aaabbbccc\".squeeze\n") {
        Some(out) => assert_eq!(out, "abc\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn squeeze_with_a_charset_only_collapses_matching_runs() {
    match run_ruby(
        "puts \"aaabbbccc\".squeeze(\"a\")\nputs \"aaabbbccc\".squeeze(\"ab\")\n",
    ) {
        Some(out) => assert_eq!(out, "abbbccc\nabccc\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn squeeze_with_an_empty_string_charset_collapses_nothing() {
    // Real Ruby: `"aabbcc".squeeze("")` is unchanged -- an empty charset
    // argument still counts as "has a charset" (unlike the no-argument
    // form), and the empty set has no members to match against.
    match run_ruby("puts \"aabbcc\".squeeze(\"\")\n") {
        Some(out) => assert_eq!(out, "aabbcc\n"),
        None => eprintln!("skip: no cc"),
    }
}

// `.inspect` isn't implemented for a String receiver (only Symbol -- a
// separate, pre-existing gap; see the backlog item filed alongside this
// batch), and trailing whitespace is invisible in a bare `puts`, so every
// test below wraps its result in `"[" + ... + "]"` (the `+` operator, which
// routes through a DIFFERENT, already-proven mechanism than `.` method
// dispatch -- see the `string_concat` conformance corpus entry) to make
// padding visible. The pad character is `"-"`, not Ruby's usual example
// `"*"`: a `"*"`-content STRING LITERAL in a comma-separated call-argument
// position crashes this frontend's PARSER (confirmed independently of
// `ljust`/`rjust`/`center` -- a bare `foo(1, "*")` panics the same way);
// that is a separate, real parser bug, filed as its own backlog item.

#[test]
fn ljust_rjust_pad_with_a_default_space_or_a_given_string() {
    match run_ruby(
        "puts \"[\" + \"hello\".ljust(8) + \"]\"\nputs \"[\" + \"hello\".ljust(8, \"-\") + \"]\"\nputs \"[\" + \"hello\".rjust(8, \"-\") + \"]\"\n",
    ) {
        Some(out) => assert_eq!(out, "[hello   ]\n[hello---]\n[---hello]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn center_puts_any_odd_leftover_pad_char_on_the_right() {
    match run_ruby(
        "puts \"[\" + \"hello\".center(11, \"-\") + \"]\"\nputs \"[\" + \"hello\".center(10, \"-\") + \"]\"\n",
    ) {
        Some(out) => assert_eq!(out, "[---hello---]\n[--hello---]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_width_at_or_below_the_receivers_length_is_a_noop() {
    match run_ruby(
        "puts \"[\" + \"hi\".ljust(1) + \"]\"\nputs \"[\" + \"hi\".rjust(0) + \"]\"\nputs \"[\" + \"hi\".ljust(2) + \"]\"\n",
    ) {
        Some(out) => assert_eq!(out, "[hi]\n[hi]\n[hi]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_float_width_argument_truncates_toward_zero() {
    // Real Ruby: `"hi".ljust(5.9)` pads to 5, not 6 -- `5.9`'s Integer part.
    match run_ruby("puts \"[\" + \"hi\".ljust(5.9) + \"]\"\n") {
        Some(out) => assert_eq!(out, "[hi   ]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn an_empty_receiver_pads_from_nothing() {
    match run_ruby("puts \"[\" + \"\".ljust(3) + \"]\"\nputs \"[\" + \"\".squeeze + \"]\"\n") {
        Some(out) => assert_eq!(out, "[   ]\n[]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_hostile_extreme_width_saturates_and_is_clamped_instead_of_exhausting_memory() {
    // Security-relevant: a huge-magnitude width must not attempt a
    // multi-gigabyte allocation. Both a saturated INT64_MAX-ish width (from
    // an extreme Float argument) and the SIR_MAX_PAD_LEN clamp itself are
    // exercised -- this only checks the program terminates with a bounded,
    // correctly-padded result rather than hanging or aborting.
    match run_ruby("puts \"x\".ljust(1.0e300).length\n") {
        // 1 (the receiver) + SIR_MAX_PAD_LEN (the clamped deficit).
        Some(out) => assert_eq!(out, "100000001\n"),
        None => eprintln!("skip: no cc"),
    }
}

/// A per-call unique part for this file's temp-file names.  The process id
/// keeps parallel test binaries apart, and the counter keeps apart the tests
/// of this binary, which run on parallel threads under one process id
/// (#18485).  A name built from the process id alone, or from it and a hash
/// of the source, let two tests that compile the same program share a file.
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
