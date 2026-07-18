//! End-to-end tests for ADJ-TABLES RS-5c — the RANGE / BRACKET lookup tactic —
//! driven through the built CLI binary. A `table` is read as a step function:
//! `? lookup <table> <key_col> = <n> mode range give <value_col>` selects the
//! breakpoint row whose key is the greatest key `<= n` and returns its value
//! column WITH that row's citation. Four things are proven:
//!
//!   (a) BRACKET HIT: a value strictly inside a band binds the band's value AND
//!       the selected breakpoint row's citation, and the audit names the matched
//!       key (which bracket it fell in).
//!   (b) EXACT BOUNDARY: a value equal to a breakpoint selects THAT breakpoint
//!       (greatest key `<=` value is the value itself) — the exact comparison,
//!       no `f64` fuzz.
//!   (c) TOP-OPEN BAND + BELOW-DOMAIN ABSTENTION: a value above the last
//!       breakpoint falls in the top band; a value below the smallest key has no
//!       key `<=` it and honestly ABSTAINS ("below the table's domain"), never a
//!       fabricated classification.
//!   (d) GUARDS: a `mode interpolated` is rejected (reserved for RS-5d), an
//!       unknown value column is a clean compile error, and a non-numeric key
//!       column is rejected (a range key must be a number).

use std::path::Path;
use std::process::Command;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5c_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI, returning (exit-ok, stdout, stderr) so the error-path tests can
/// assert on the diagnostic regardless of which stream it lands on.
fn run_full(program: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn write(dir: &Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

/// A self-contained BMI-category breakpoint table used across the hit/abstain
/// cases. Four bands keyed by the band's minimum BMI; the key column is numeric.
const BMI_TABLE: &str = r#"
table bmi_categories {
    columns min_bmi, category
    row (0, underweight)
    row (18.5, normal)
    row (25, overweight)
    row (30, obese)
    source "A breakpoint table of BMI weight-status bands keyed by band minimum."
    locator "https://example.test/bmi-bands"
    trust consensus
}
"#;

// ---------------------------------------------------------------------------
// (a) Bracket hit — inside a band, carries the selected row's citation + key.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_bracket_hit_binds_band_value_with_citation_and_matched_key() {
    let dir = scratch("hit");
    let src = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27.3 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(out.contains("\"lookups\""), "expected a lookups section: {out}");
    // 27.3 falls in [25, 30) → overweight, selected breakpoint min_bmi = 25.
    assert!(out.contains("\"category\":\"overweight\""), "wrong band: {out}");
    assert!(out.contains("\"min_bmi\":\"25\""), "audit should name the matched breakpoint: {out}");
    // The selected row's citation travels with the answer.
    assert!(out.contains("example.test/bmi-bands"), "answer must carry the row citation: {out}");
    assert!(out.contains("\"trust\":\"consensus\""), "citation must carry the trust tier: {out}");
    assert!(out.contains("\"abstained\":false"), "a hit is not an abstention: {out}");
}

// ---------------------------------------------------------------------------
// (b) Exact boundary — a value equal to a breakpoint selects that breakpoint.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_on_an_exact_breakpoint_selects_that_breakpoint() {
    let dir = scratch("boundary");
    let src = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 18.5 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, _err) = run_full(&dir.join("case.adj"));
    assert!(ok);
    // greatest key <= 18.5 is 18.5 itself → normal (exact comparison, no f64 fuzz).
    assert!(out.contains("\"category\":\"normal\""), "exact boundary must land on the breakpoint: {out}");
    assert!(out.contains("\"min_bmi\":\"18.5\""), "matched key should be the breakpoint itself: {out}");
}

// ---------------------------------------------------------------------------
// (c) Top-open band + below-domain abstention.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_top_band_is_open_and_below_domain_abstains() {
    let dir = scratch("edges");
    // A value above the last breakpoint falls in the top (open) band.
    let top = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 42 mode range give category\n");
    write(&dir, "top.adj", &top);
    let (ok, out, _e) = run_full(&dir.join("top.adj"));
    assert!(ok);
    assert!(out.contains("\"category\":\"obese\""), "top-open band should be selected: {out}");

    // A value below the smallest key has no key <= it → honest abstention.
    let below = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = -1 mode range give category\n");
    write(&dir, "below.adj", &below);
    let (ok2, out2, _e2) = run_full(&dir.join("below.adj"));
    assert!(ok2);
    assert!(out2.contains("\"abstained\":true"), "below the domain must abstain, not fabricate: {out2}");
    assert!(!out2.contains("\"category\":\""), "an abstention has no bound band: {out2}");
}

// ---------------------------------------------------------------------------
// (d) Guards — reserved mode, unknown column, non-numeric key.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_reserved_interpolated_mode_is_rejected() {
    let dir = scratch("mode");
    let src = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27 mode interpolated give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "interpolated is RS-5d — must be rejected, not silently run as range");
    let diag = format!("{out}{err}");
    assert!(diag.contains("LookupModeUnsupported") || diag.to_lowercase().contains("interpolated"), "diag: {diag}");
}

#[test]
fn range_lookup_unknown_value_column_is_a_clean_compile_error() {
    let dir = scratch("col");
    let src = format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27 mode range give bmi_class\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "an unknown value column must be a compile error");
    let diag = format!("{out}{err}");
    assert!(diag.contains("LookupUnknownColumn") || diag.contains("bmi_class"), "diag: {diag}");
}

#[test]
fn range_lookup_non_numeric_key_column_is_rejected() {
    let dir = scratch("nonnum");
    // Here the *category* (an atom column) is (mis)used as the key — a range key
    // must be numeric, so this is a clean compile error, not a silent skip.
    let src = format!("{BMI_TABLE}\n? lookup bmi_categories category = 27 mode range give min_bmi\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "a non-numeric key column must be rejected");
    let diag = format!("{out}{err}");
    assert!(diag.contains("LookupNonNumericKeyColumn") || diag.contains("category"), "diag: {diag}");
}
