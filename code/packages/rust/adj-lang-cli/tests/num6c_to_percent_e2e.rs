//! End-to-end tests for the NUM-6c `to_percent(x [, places])` percentage rendering, driven
//! through the built `adj-lang-cli` binary. They prove the whole path — native application
//! surface → adapter/lower → the engine's exact scale-and-round → the audit JSON — works
//! together: the ratio is scaled and rounded **exactly** (no `f64` hop), the rendered `d.dd%`
//! string and the narrowed exact fraction agree, the audit exposes the rendering
//! (`node:to_percent`, `places`, `mode`, `rendered`, the operand subtree), the `places` arg is
//! optional (a documented default) and `0` is valid, and a bad `places` is a clean compile
//! error rather than a silent mis-format (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num6cp_{tag}_{}", std::process::id()));
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
fn to_percent_renders_a_ratio_and_audits_the_rendering() {
    // 1/3 = 0.333… → 2 places = "33.33%", the narrowed FRACTION held as 3333/10000.
    let (ok, s) = run("let r = to_percent(1 / 3, 2)\n? r\n", "value");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"rendered\":\"33.33%\""),
        "renders 1/3 → 33.33%: {s}"
    );
    // The exact sidecar is the true narrowed fraction (33.33% = 3333/10000).
    assert!(
        s.contains("\"num\":\"3333\"") && s.contains("\"den\":\"10000\""),
        "carries the exact narrowed fraction 3333/10000: {s}"
    );
    // The audit trail exposes the rendering as a first-class, checkable step.
    assert!(
        s.contains("\"node\":\"to_percent\"")
            && s.contains("\"places\":2")
            && s.contains("\"mode\":\"half_even\""),
        "audit records node/places/mode: {s}"
    );
}

#[test]
fn to_percent_pads_trailing_zeros_and_zero_places_drops_the_point() {
    // 1/2 → 2 places pads: "50.00%".
    let (ok, s) = run("let r = to_percent(1 / 2, 2)\n? r\n", "pad");
    assert!(ok, "cli should succeed: {s}");
    assert!(s.contains("\"rendered\":\"50.00%\""), "1/2 → 50.00%: {s}");
    // 1/2 → 0 places drops the decimal point: "50%".
    let (ok2, s2) = run("let r = to_percent(1 / 2, 0)\n? r\n", "zero");
    assert!(ok2, "cli should succeed: {s2}");
    assert!(s2.contains("\"rendered\":\"50%\""), "1/2 at 0 places → 50%: {s2}");
}

#[test]
fn to_percent_places_argument_is_optional() {
    // `to_percent(x)` without a place count uses the default (two decimal places).
    let (ok, s) = run("let r = to_percent(1 / 8)\n? r\n", "default");
    assert!(ok, "cli should succeed: {s}");
    // 1/8 = 0.125 → 2 places (half-even) = "12.50%".
    assert!(
        s.contains("\"rendered\":\"12.50%\"") && s.contains("\"places\":2"),
        "the default is two decimal places: {s}"
    );
}

#[test]
fn a_negative_place_count_is_a_compile_error() {
    // `places` must be a non-negative integer; a negative is rejected at compile time.
    let (ok, s) = run("let r = to_percent(1 / 3, -1)\n? r\n", "negplaces");
    assert!(
        !ok || s.contains("\"error\""),
        "a negative place count must be rejected: {s}"
    );
    assert!(
        !s.contains("\"node\":\"to_percent\""),
        "must not emit a rendering node: {s}"
    );
}

#[test]
fn a_non_integer_place_count_is_a_compile_error() {
    let (ok, s) = run("let r = to_percent(1 / 3, 1.5)\n? r\n", "badplaces");
    assert!(
        !ok || s.contains("\"error\""),
        "a non-integer place count must be rejected: {s}"
    );
}
