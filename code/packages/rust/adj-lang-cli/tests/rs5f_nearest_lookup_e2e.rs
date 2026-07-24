//! End-to-end tests for ADJ-TABLES RS-5f — the NEAREST / nearest-neighbour lookup
//! tactic — driven through the built CLI binary. A `table` is read as a discrete grid:
//! `? lookup <table> <key_col> = <n> mode nearest give <value_col>` selects the single
//! row whose key is CLOSEST to `n` (exact `|k - n|`, ties → smaller key) and returns its
//! value column VERBATIM, with that row's citation. Where `range` floors and
//! `interpolated` blends, `nearest` snaps. Proven here:
//!
//!   (a) SNAP: a query between two keys binds the value of the closer key and carries
//!       that row's citation + the matched key.
//!   (b) EXACT TIE → SMALLER KEY: a query exactly halfway between two keys snaps to the
//!       LOWER key, deterministically (order-independent, exact arithmetic — no `f64`).
//!   (c) NON-NUMERIC VALUE COLUMN IS FINE: unlike `interpolated`, `nearest` returns the
//!       value cell as-is, so a category-label value column is allowed and returned.
//!   (d) OUT-OF-DOMAIN STILL SNAPS: a query beyond the last key snaps to the nearest
//!       endpoint (nearest-neighbour never abstains for a non-empty table), and an
//!       empty table honestly abstains (no nearest key to invent).

use std::path::Path;
use std::process::Command;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5f_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI, returning (exit-ok, stdout, stderr).
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

/// A self-contained lens-power / dioptre snap grid: standard trial-lens powers a
/// measured refraction is snapped to. Numeric key (dioptres), numeric value (also the
/// dioptre, so the "nearest stocked lens" is returned as a number).
const LENS_GRID: &str = r#"
table trial_lenses {
    columns power, stocked
    row (0.25, 0.25)
    row (0.5, 0.5)
    row (0.75, 0.75)
    row (1.0, 1.0)
    source "A grid of standard stocked trial-lens powers, in dioptres."
    locator "https://example.test/trial-lenses"
    trust consensus
}
"#;

// ---------------------------------------------------------------------------
// (a) Snap — a query between two keys binds the CLOSER key's value + citation.
// ---------------------------------------------------------------------------

#[test]
fn nearest_snaps_to_the_closest_key_with_citation_and_matched_key() {
    let dir = scratch("snap");
    // 0.6 is closer to 0.5 (|0.6-0.5| = 0.1) than to 0.75 (|0.75-0.6| = 0.15).
    let src = format!("{LENS_GRID}\n? lookup trial_lenses power = 0.6 mode nearest give stocked\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(out.contains("\"lookups\""), "expected a lookups section: {out}");
    assert!(!out.contains("\"error\""), "no compile error: {out}");
    // Snaps to 0.5 (the nearer key), returns its stocked value 0.5, not blended.
    assert!(
        out.contains("\"stocked\":\"0.5\"") && out.contains("\"power\":\"0.5\""),
        "0.6 snaps to the 0.5 lens: {out}"
    );
    assert!(out.contains("\"mode\":\"nearest\""), "mode echoed: {out}");
    assert!(out.contains("\"abstained\":false"), "a hit, not an abstention: {out}");
    // The selected row's citation rides along.
    assert!(
        out.contains("trial-lenses"),
        "carries the snapped row's citation: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Exact tie → smaller key, deterministically (exact halfway point).
// ---------------------------------------------------------------------------

#[test]
fn nearest_breaks_an_exact_tie_toward_the_smaller_key() {
    let dir = scratch("tie");
    // 0.625 is EXACTLY halfway between 0.5 and 0.75 (|·| = 0.125 both ways).
    // The documented tie-break picks the SMALLER key: 0.5.
    let src =
        format!("{LENS_GRID}\n? lookup trial_lenses power = 0.625 mode nearest give stocked\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(
        out.contains("\"power\":\"0.5\"") && out.contains("\"stocked\":\"0.5\""),
        "an exact tie snaps to the smaller key 0.5, not 0.75: {out}"
    );
    assert!(out.contains("\"abstained\":false"), "a deterministic hit: {out}");
}

// ---------------------------------------------------------------------------
// (c) Non-numeric value column is fine — the cell is returned verbatim.
// ---------------------------------------------------------------------------

#[test]
fn nearest_returns_a_non_numeric_value_cell_verbatim() {
    let dir = scratch("label");
    // A snap grid whose VALUE column is a category label (illegal for `interpolated`,
    // fine for `nearest`, which returns the cell as-is).
    let table = r#"
table shoe_sizes {
    columns foot_cm, label
    row (22, small)
    row (25, medium)
    row (28, large)
    source "A snap grid of shoe-size labels keyed by foot length in centimetres."
    locator "https://example.test/shoe-sizes"
    trust consensus
}
"#;
    // 24.2 is closest to 25 (|0.8|) vs 22 (|2.2|) → "medium".
    let src = format!("{table}\n? lookup shoe_sizes foot_cm = 24.2 mode nearest give label\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(!out.contains("\"error\""), "a label value column is allowed: {out}");
    assert!(
        out.contains("\"label\":\"medium\"") && out.contains("\"foot_cm\":\"25\""),
        "24.2 snaps to the 25 cm row → medium: {out}"
    );
}

// ---------------------------------------------------------------------------
// (d) Out-of-domain still snaps to the nearest endpoint (never abstains for a
//     non-empty table) — the defining difference from range/interpolated.
// ---------------------------------------------------------------------------

#[test]
fn nearest_snaps_out_of_domain_query_to_the_endpoint() {
    let dir = scratch("beyond");
    // 5.0 is far above the last key 1.0, but the NEAREST key still exists: 1.0.
    // (`range` would floor to 1.0 too; `interpolated` would ABSTAIN above-domain —
    // `nearest` snaps.)
    let src = format!("{LENS_GRID}\n? lookup trial_lenses power = 5.0 mode nearest give stocked\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(
        out.contains("\"power\":\"1\"") && out.contains("\"abstained\":false"),
        "5.0 snaps to the nearest endpoint 1.0, never abstains on a non-empty table: {out}"
    );
}
