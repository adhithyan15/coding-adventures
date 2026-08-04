//! End-to-end tests for the NUM-7b `Real`/`BigDouble` audit companion, driven
//! through the built `adj-lang-cli` binary. A square root's audit gains an
//! additive `"real"` block — the arbitrary-precision value, its precision, and
//! its rounding mode — alongside the ordinary `f64` `"value"`; every other
//! `Pow` node (including a non-square-root power) carries no such key
//! (ADJ-NUMERIC-SUBSTRATE §8).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num7b_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(src: &str, tag: &str) -> (bool, String) {
    let dir = scratch(tag);
    let p = dir.join("case.adj");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&p)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn sqrt_audits_a_real_companion_at_default_precision() {
    let (ok, s) = run("let r = latex \"$\\sqrt{2}$\"\n? r\n", "sqrt2");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The ordinary f64 result is still there.
    assert!(s.contains("\"value\":1.41421"), "f64 result present: {s}");
    // The additive Real companion: default 256-bit precision, half-even mode,
    // and a decimal rendering with far more digits than f64 could carry.
    assert!(
        s.contains("\"real\":{\"precision_bits\":256"),
        "real companion at the default 256-bit precision: {s}"
    );
    assert!(s.contains("\"mode\":\"half_even\""), "records its rounding mode: {s}");
    assert!(
        s.contains("1.41421356237309504880168872420969807856"),
        "renders far more digits than an f64 could hold: {s}"
    );
}

#[test]
fn a_perfect_square_still_gets_a_real_companion() {
    // sqrt(9) = 3 exactly — NUM-7 does not special-case a perfect square away,
    // so it gets a real companion like any other sqrt.
    let (ok, s) = run("let r = latex \"$\\sqrt{9}$\"\n? r\n", "sqrt9");
    assert!(ok, "cli should succeed: {s}");
    assert!(s.contains("\"value\":3"), "f64 result present: {s}");
    assert!(s.contains("\"real\":{\"precision_bits\":256"), "still gets a real companion: {s}");
    assert!(s.contains("\"value\":\"3\""), "renders the exact digit: {s}");
}

#[test]
fn pow_that_is_not_a_sqrt_has_no_real_key() {
    let (ok, s) = run("let r = latex \"$2^3$\"\n? r\n", "pow23");
    assert!(ok, "cli should succeed: {s}");
    assert!(s.contains("\"value\":8"), "computes the power: {s}");
    assert!(!s.contains("\"real\":"), "an ordinary power carries no real companion: {s}");
}
