//! End-to-end tests for the NUM-6c `to_currency(x, code [, places])` money rendering, driven
//! through the built `adj-lang-cli` binary. They prove the whole path — native application
//! surface (with a bare-identifier currency code, lexed lowercase and normalized to the
//! canonical uppercase ISO-4217 form) → adapter/lower → the engine's exact
//! base-10 rounding → the audit JSON — works together: the amount is rounded **exactly** (no
//! `f64` hop), the rendered `CODE d.dd` string and the narrowed exact amount agree, the audit
//! exposes the rendering (`node:to_currency`, `code`, `places`, `mode`, `rendered`, the
//! operand subtree), the `places` arg is optional (a documented default) and `0` is valid, and
//! a bad `places` is a clean compile error (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num6cc_{tag}_{}", std::process::id()));
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
fn to_currency_renders_amount_with_code_and_audits_the_rendering() {
    // $100 split 3 ways = 33.333… → 2 places (half-even) = "USD 33.33", the narrowed amount
    // held as the EXACT fraction 3333/100 — no lossy f64 hop.
    let (ok, s) = run("let r = to_currency(100 / 3, usd, 2)\n? r\n", "value");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"rendered\":\"USD 33.33\""),
        "renders 100/3 → USD 33.33: {s}"
    );
    // The exact sidecar is the true narrowed amount (33.33 = 3333/100).
    assert!(
        s.contains("\"num\":\"3333\"") && s.contains("\"den\":\"100\""),
        "carries the exact narrowed amount 3333/100: {s}"
    );
    // The audit trail exposes the rendering as a first-class, checkable step: the node type,
    // the currency code, the place count, the stated mode, and the rendered string.
    assert!(
        s.contains("\"node\":\"to_currency\"")
            && s.contains("\"code\":\"USD\"")
            && s.contains("\"places\":2")
            && s.contains("\"mode\":\"half_even\""),
        "audit records node/code/places/mode: {s}"
    );
}

#[test]
fn to_currency_pads_trailing_zeros_and_zero_places_drops_the_point() {
    // An exact amount 2469/2 = 1234.5 → 2 places pads: "USD 1234.50".
    let (ok, s) = run("let r = to_currency(2469 / 2, usd, 2)\n? r\n", "pad");
    assert!(ok, "cli should succeed: {s}");
    assert!(
        s.contains("\"rendered\":\"USD 1234.50\""),
        "1234.5 → USD 1234.50: {s}"
    );
    // JPY has no minor unit: 0 places drops the decimal point. 7/2 = 3.5 → "JPY 4" (half-even).
    let (ok2, s2) = run("let r = to_currency(7 / 2, jpy, 0)\n? r\n", "zero");
    assert!(ok2, "cli should succeed: {s2}");
    assert!(
        s2.contains("\"rendered\":\"JPY 4\"") && s2.contains("\"code\":\"JPY\""),
        "3.5 at 0 places → JPY 4: {s2}"
    );
}

#[test]
fn to_currency_places_argument_is_optional() {
    // `to_currency(x, code)` without a place count uses the default (two decimal places).
    let (ok, s) = run("let r = to_currency(5, eur)\n? r\n", "default");
    assert!(ok, "cli should succeed: {s}");
    assert!(
        s.contains("\"rendered\":\"EUR 5.00\"") && s.contains("\"places\":2"),
        "the default is two decimal places: {s}"
    );
}

#[test]
fn a_negative_place_count_is_a_compile_error() {
    let (ok, s) = run("let r = to_currency(100 / 3, usd, -1)\n? r\n", "negplaces");
    assert!(
        !ok || s.contains("\"error\""),
        "a negative place count must be rejected: {s}"
    );
    assert!(
        !s.contains("\"node\":\"to_currency\""),
        "must not emit a rendering node: {s}"
    );
}

#[test]
fn a_numeric_currency_code_is_a_compile_error() {
    // The code must be a bare identifier (`USD`), not a number — `to_currency(x, 2)` with the
    // code slot filled by a number is a clean compile error, never a mis-render.
    let (ok, s) = run("let r = to_currency(100 / 3, 2)\n? r\n", "numcode");
    assert!(
        !ok || s.contains("\"error\""),
        "a numeric currency code must be rejected: {s}"
    );
}
