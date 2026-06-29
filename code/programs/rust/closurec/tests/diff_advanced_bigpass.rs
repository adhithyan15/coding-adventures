//! Big-pass ADVANCED proof for the `tests/diff/advanced-bigpass/` fixture.
//!
//! Most fixtures isolate ONE transformation. This one is an END-TO-END proof
//! that the whole ADVANCED pipeline cooperates on a realistic little module —
//! and, crucially, that it does so WITHOUT changing the program's observable
//! behaviour.
//!
//! The module (see `input/a.js`) defines four functions and reports three
//! values. At `--compilation_level ADVANCED` it collapses to:
//!
//! ```text
//! function f(x){return x * 10};report(12,25,f(7));sink(f);
//! ```
//!
//! Four distinct passes are visible in that single line:
//!
//! | pass                       | evidence in the output                        |
//! |----------------------------|-----------------------------------------------|
//! | dead-code elimination      | `unusedPerimeter` is gone                      |
//! | single-use inline + fold   | `area(3,4)`→`12`, `hypotSq(3,4)`→`25`          |
//! | global renaming (ADV-only) | `scale` → `f` (SIMPLE keeps the name `scale`)  |
//! | live-reference retention   | `f(7)` and `sink(f)` survive                   |
//!
//! ## Runtime equivalence (the point of the proof)
//!
//! An optimizer that shrinks code but changes results is worse than useless.
//! Here the equivalence is checkable by hand:
//!
//! * original `area(3,4)` = `3*4` = **12**     → output literal `12`
//! * original `hypotSq(3,4)` = `9+16` = **25** → output literal `25`
//! * original `scale(7)` = `7*10` = **70**     → output `f(7)`, `f` ≡ `x*10`, so `70`
//!
//! The two folded literals are asserted directly; the third value is preserved
//! structurally (same body, renamed). Same observable behaviour, ~29% of the
//! size.
//!
//! ## Measuring the *optimization* savings honestly
//!
//! Comparing against the raw source would conflate optimization with
//! comment/whitespace stripping. We instead baseline against
//! `WHITESPACE_ONLY` (which strips comments + whitespace but performs NO
//! optimization). ADVANCED must be dramatically smaller than that baseline —
//! the difference is real optimization, not cosmetics.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const INPUT: &str = "tests/diff/advanced-bigpass/input/a.js";

/// Run closurec at `level` on the fixture input and return stdout as a String.
fn run_at(level: &str) -> String {
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js", INPUT])
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "closurec failed at {level}: exit {:?}, stderr {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/advanced-bigpass/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn advanced_bigpass_fixture_matches_expected_stdout() {
    // The flags file pins `--compilation_level ADVANCED --js input/a.js`.
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/advanced-bigpass/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "byte-exact mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

#[test]
fn advanced_bigpass_eliminates_dead_code_inlines_and_folds() {
    let adv = run_at("ADVANCED");

    // Dead-code elimination: the never-referenced helper is gone, body and all.
    assert!(
        !adv.contains("unusedPerimeter"),
        "unusedPerimeter should be tree-shaken; got:\n{adv}"
    );
    assert!(
        !adv.contains("w + h") && !adv.contains("w+h"),
        "the dead function body should be gone; got:\n{adv}"
    );

    // Single-use inlining + constant folding: the two call sites collapse to
    // their literal results — these are the runtime-equivalence anchors.
    assert!(
        adv.contains("12") && adv.contains("25"),
        "area(3,4)->12 and hypotSq(3,4)->25 should be folded literals; got:\n{adv}"
    );
    // ...and the helpers they came from are fully inlined away.
    assert!(
        !adv.contains("area") && !adv.contains("hypotSq"),
        "area/hypotSq should be inlined out of existence; got:\n{adv}"
    );
}

#[test]
fn advanced_bigpass_renames_surviving_global_unlike_simple() {
    // The ADVANCED-only global-rename pass is the distinguishing behaviour:
    // `scale` survives (it is passed by value to `sink`, so the inliner
    // declines it) but is renamed. SIMPLE runs the same fold/DCE/inline but
    // does NOT rename globals, so it keeps the original name.
    let adv = run_at("ADVANCED");
    let simple = run_at("SIMPLE");

    assert!(
        !adv.contains("scale"),
        "ADVANCED should rename the surviving global `scale`; got:\n{adv}"
    );
    assert!(
        simple.contains("scale"),
        "SIMPLE should keep the name `scale` (no global rename); got:\n{simple}"
    );
    // The live reference is retained in both (sink keeps it reachable).
    assert!(
        adv.contains("sink(") && adv.contains("report("),
        "live calls report(...) and sink(...) must survive; got:\n{adv}"
    );
}

#[test]
fn advanced_bigpass_optimization_beats_whitespace_only_baseline() {
    // Baseline against WHITESPACE_ONLY (comments + whitespace stripped, NO
    // optimization) so the measured shrink is attributable to optimization,
    // not comment removal.
    let ws_len = run_at("WHITESPACE_ONLY").trim_end_matches('\n').len();
    let adv_len = run_at("ADVANCED").trim_end_matches('\n').len();

    assert!(
        ws_len > 0 && adv_len > 0,
        "both outputs should be non-empty (ws={ws_len}, adv={adv_len})"
    );
    // Demand a substantial real reduction: ADVANCED must be under HALF the
    // already-minified-but-unoptimized baseline. (Observed: 56 vs 195 bytes,
    // a ~71% reduction; the 50% bar leaves generous headroom for emitter
    // tweaks while still proving optimization happened.)
    assert!(
        adv_len * 2 < ws_len,
        "ADVANCED ({adv_len}B) should be < half the WHITESPACE_ONLY baseline \
         ({ws_len}B) — proving real optimization, not just comment stripping",
    );
}

#[test]
fn advanced_bigpass_did_not_fall_back_to_whitespace_only() {
    // Regression guard: ADVANCED must run the typed optimizer, not silently
    // fall back to the whitespace re-stitcher (which would leave every call
    // intact and the dead function present).
    let adv = run_at("ADVANCED");
    let ws = run_at("WHITESPACE_ONLY");
    assert_ne!(
        adv.trim_end_matches('\n'),
        ws.trim_end_matches('\n'),
        "ADVANCED output must differ from WHITESPACE_ONLY; got:\n{adv}"
    );
}
