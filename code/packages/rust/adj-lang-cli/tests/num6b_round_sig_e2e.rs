//! End-to-end tests for the NUM-6b `round_sig(x, n)` significant-figures
//! narrowing, driven through the built `adj-lang-cli`. They prove the whole path
//! — native application surface → the engine's magnitude-aware exact rounding →
//! the audit JSON — works together across scales: a large integer rounds to the
//! right power of ten, a fraction rounds to `n` significant digits (leading zeros
//! not counted), the value stays **exact**, the audit exposes `sig_figures`, and
//! `n = 0` (meaningless) is a clean compile error (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num6b_{tag}_{}", std::process::id()));
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
fn round_sig_rounds_a_large_integer_to_a_power_of_ten_and_audits_it() {
    // 31459 to 3 significant figures = 31500 — rounding to the hundreds, held
    // exactly as 31500/1, with the audit naming the significant-figures spec.
    let (ok, s) = run("let r = round_sig(31459, 3)\n? r\n", "bigint");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"value\":31500"), "rounds to 31500: {s}");
    assert!(
        s.contains("\"num\":\"31500\"") && s.contains("\"den\":\"1\""),
        "exact 31500/1: {s}"
    );
    assert!(
        s.contains("\"node\":\"round\"")
            && s.contains("\"sig_figures\":3")
            && s.contains("\"mode\":\"half_even\""),
        "audit records the significant-figures narrowing: {s}"
    );
}

#[test]
fn round_sig_rounds_fractional_values_exactly_across_scales() {
    // 3.14159 to 3 sig-figs = 3.14 = 157/50.
    let (ok, s) = run("let r = round_sig(314159 / 100000, 3)\n? r\n", "frac");
    assert!(ok, "cli should succeed: {s}");
    assert!(
        s.contains("\"value\":3.14") && s.contains("\"num\":\"157\"") && s.contains("\"den\":\"50\""),
        "3.14159 → 3.14 = 157/50: {s}"
    );
    // 0.00314159 to 2 sig-figs = 0.0031 = 31/10000 (leading zeros don't count).
    let (ok2, s2) = run("let r = round_sig(314159 / 100000000, 2)\n? r\n", "small");
    assert!(ok2, "cli should succeed: {s2}");
    assert!(
        s2.contains("\"num\":\"31\"") && s2.contains("\"den\":\"10000\""),
        "0.00314159 → 0.0031 = 31/10000: {s2}"
    );
}

#[test]
fn zero_significant_figures_is_a_compile_error() {
    // Zero significant figures is meaningless; it must be rejected, not silently
    // rounded to something.
    let (ok, s) = run("let r = round_sig(5, 0)\n? r\n", "zero");
    assert!(
        !ok || s.contains("\"error\""),
        "round_sig(x, 0) must be rejected: {s}"
    );
}
