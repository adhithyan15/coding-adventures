//! End-to-end tests for the NUM-6c `to_scientific(x [, figures])` scientific-notation
//! rendering, driven through the built `adj-lang-cli` binary. They prove the whole path —
//! native application surface → adapter/lower → the engine's exact significant-figure
//! narrowing → the audit JSON — works together: the mantissa is narrowed **exactly** (no
//! `f64` log or tie-break), the rendered `d.ddde±E` string and the narrowed exact value
//! agree, the audit exposes the rendering (`node:to_scientific`, `figures`, `mode`,
//! `rendered`, the operand subtree), the `figures` arg is optional (a documented default),
//! and a bad `figures` is a clean compile error rather than a silent mis-format
//! (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num6c_{tag}_{}", std::process::id()));
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
fn to_scientific_renders_exactly_and_audits_the_rendering() {
    // 31459 to 3 significant figures = 3.15e4 (the trailing 59 rounds the 4 up), with
    // the narrowed value held as the EXACT fraction 31500/1 — no lossy f64 hop.
    let (ok, s) = run("let r = to_scientific(31459, 3)\n? r\n", "value");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The rendered boundary string is the scientific form.
    assert!(
        s.contains("\"rendered\":\"3.15e4\""),
        "renders 31459 → 3.15e4: {s}"
    );
    // The exact sidecar is the true narrowed fraction, so an auditor re-derives it.
    assert!(
        s.contains("\"num\":\"31500\"") && s.contains("\"den\":\"1\""),
        "carries the exact narrowed value 31500/1: {s}"
    );
    // The audit trail exposes the rendering as a first-class, checkable step: the node
    // type, the significant-figure count, the stated mode, the rendered string, and the
    // operand subtree it narrowed — everything `adj-verify` needs to re-render.
    assert!(
        s.contains("\"node\":\"to_scientific\"")
            && s.contains("\"figures\":3")
            && s.contains("\"mode\":\"half_even\""),
        "audit records node/figures/mode: {s}"
    );
}

#[test]
fn to_scientific_narrows_a_repeating_rational_on_the_exact_path() {
    // 1/3 = 0.333… to 4 sig-figs = 3.333e-1, held exactly as 3333/10000 — the whole
    // point: the rendered figures come from the exact fraction, not an f64 log.
    let (ok, s) = run("let r = to_scientific(1 / 3, 4)\n? r\n", "repeat");
    assert!(ok, "cli should succeed: {s}");
    assert!(
        s.contains("\"rendered\":\"3.333e-1\""),
        "renders 1/3 → 3.333e-1: {s}"
    );
    assert!(
        s.contains("\"num\":\"3333\"") && s.contains("\"den\":\"10000\""),
        "carries the exact narrowed value 3333/10000: {s}"
    );
}

#[test]
fn to_scientific_figures_argument_is_optional() {
    // `to_scientific(x)` without a figure count uses the default mantissa precision
    // (six significant figures) — a documented default, recorded in the audit.
    let (ok, s) = run("let r = to_scientific(31459)\n? r\n", "default");
    assert!(ok, "cli should succeed: {s}");
    // 31459 at 6 sig-figs is exactly 31459 = 3.14590e4 (a padding zero at the 6th figure).
    assert!(
        s.contains("\"rendered\":\"3.14590e4\"") && s.contains("\"figures\":6"),
        "the default is six significant figures: {s}"
    );
}

#[test]
fn a_zero_figure_count_is_a_compile_error() {
    // A scientific mantissa has at least one significant figure, so `figures` must be
    // ≥ 1; zero is rejected at compile time rather than producing a meaningless render.
    let (ok, s) = run("let r = to_scientific(31459, 0)\n? r\n", "zerofig");
    assert!(
        !ok || s.contains("\"error\""),
        "a zero figure count must be rejected: {s}"
    );
    assert!(
        !s.contains("\"node\":\"to_scientific\""),
        "must not emit a rendering node: {s}"
    );
}

#[test]
fn a_non_integer_figure_count_is_a_compile_error() {
    let (ok, s) = run("let r = to_scientific(31459, 2.5)\n? r\n", "badfig");
    assert!(
        !ok || s.contains("\"error\""),
        "a non-integer figure count must be rejected: {s}"
    );
}
