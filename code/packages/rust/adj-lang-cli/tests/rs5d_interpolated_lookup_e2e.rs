//! End-to-end tests for ADJ-TABLES RS-5d — the INTERPOLATED lookup tactic —
//! driven through the built CLI binary. A `table` is read as a piecewise-linear
//! function: `? lookup <table> <key_col> = <n> mode interpolated give <value_col>`
//! finds the two breakpoint rows that bracket `n` (greatest key `<= n` and smallest
//! key `>= n`) and returns the EXACT linear blend
//!     v = v0 + (v1 - v0) * (n - k0) / (k1 - k0)
//! carrying BOTH bracketing rows' citations. Five things are proven:
//!
//!   (a) LINEAR BLEND: a value strictly between two breakpoints binds the exact
//!       interpolated value AND both breakpoints' citations, and the audit names
//!       the lower/upper bracket it sits between.
//!   (b) EXACTNESS: a blend that does not terminate in base 10 renders as the exact
//!       reduced fraction (e.g. `10/3`), never a rounded `f64`.
//!   (c) EXACT HIT: a query equal to a breakpoint key returns THAT row's value with
//!       its single citation (the `0/0` blend is short-circuited, not divided).
//!   (d) OUT-OF-DOMAIN: a query below the lowest / above the highest breakpoint
//!       ABSTAINS (`below_table_domain` / `above_table_domain`) — interpolation
//!       never extrapolates past what the source measured.
//!   (e) NUMERIC-VALUE GUARD is covered in the RS-5c suite (a non-numeric `give`
//!       column is a clean compile error under `mode interpolated`).

use std::path::Path;
use std::process::Command;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5d_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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

/// A self-contained calibration table with a numeric key AND numeric value column,
/// so it can be read as a piecewise-linear function. Two linear segments with
/// different slopes (0→10 rises by 10/unit; 10→25 rises by 10/unit as well but the
/// second point makes the bracketing unambiguous).
const CALIBRATION: &str = r#"
table calibration {
    columns input, output
    row (0, 0)
    row (10, 100)
    row (25, 250)
    source "A worked calibration table mapping instrument input to output units."
    locator "https://example.test/calibration"
    trust consensus
}
"#;

// ---------------------------------------------------------------------------
// (a) Linear blend — a value between two breakpoints, both citations ride along.
// ---------------------------------------------------------------------------

#[test]
fn interpolated_lookup_blends_between_breakpoints_with_both_citations() {
    let dir = scratch("blend");
    // input = 5 sits between (0 → 0) and (10 → 100): 0 + 100*(5-0)/(10-0) = 50.
    let src =
        format!("{CALIBRATION}\n? lookup calibration input = 5 mode interpolated give output\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(!out.contains("\"error\""), "no compile error: {out}");
    assert!(
        out.contains("\"mode\":\"interpolated\""),
        "mode echoed: {out}"
    );
    assert!(
        out.contains("\"output\":\"50\""),
        "exact linear blend 50: {out}"
    );
    assert!(
        out.contains("\"abstained\":false"),
        "not an abstention: {out}"
    );
    // The audit shows the bracket it fell between …
    assert!(
        out.contains("\"lower\":") && out.contains("\"upper\":"),
        "audit names both brackets: {out}"
    );
    // … and BOTH breakpoint rows' citations ride along (two of the same table's).
    assert_eq!(
        out.matches("example.test/calibration").count(),
        2,
        "both bracketing rows are cited: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Exactness — a non-terminating blend renders as the reduced fraction.
// ---------------------------------------------------------------------------

#[test]
fn interpolated_lookup_renders_repeating_blend_as_exact_fraction() {
    let dir = scratch("frac");
    // A 0→0, 3→10 segment; at input = 1: 0 + 10*(1-0)/(3-0) = 10/3, which repeats
    // in base 10 and so must render as the exact fraction, never a rounded float.
    let table = r#"
table ramp {
    columns x, y
    row (0, 0)
    row (3, 10)
    source "A two-point ramp for exact-fraction interpolation."
    locator "https://example.test/ramp"
    trust consensus
}
"#;
    let src = format!("{table}\n? lookup ramp x = 1 mode interpolated give y\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"y\":\"10/3\""),
        "repeating blend renders as the exact fraction 10/3, not a float: {out}"
    );
    assert!(
        !out.contains("3.333"),
        "must not fall back to a rounded decimal: {out}"
    );
}

// ---------------------------------------------------------------------------
// (c) Exact hit — a query equal to a breakpoint returns that row (single cite).
// ---------------------------------------------------------------------------

#[test]
fn interpolated_lookup_exact_breakpoint_returns_that_row() {
    let dir = scratch("exact");
    // input = 10 IS a breakpoint (→ 100). The 0/0 blend is short-circuited.
    let src =
        format!("{CALIBRATION}\n? lookup calibration input = 10 mode interpolated give output\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"output\":\"100\""),
        "exact breakpoint value: {out}"
    );
    assert!(
        out.contains("\"exact\":"),
        "audit marks it an exact hit: {out}"
    );
    // An exact hit is a single-row answer — no lower/upper bracket pair.
    assert!(
        !out.contains("\"lower\":") && !out.contains("\"upper\":"),
        "an exact hit has no bracketing pair: {out}"
    );
    assert!(
        out.contains("\"abstained\":false"),
        "not an abstention: {out}"
    );
}

// ---------------------------------------------------------------------------
// (d) Out-of-domain — below the floor and above the ceiling both abstain.
// ---------------------------------------------------------------------------

#[test]
fn interpolated_lookup_below_domain_abstains() {
    let dir = scratch("below");
    let src =
        format!("{CALIBRATION}\n? lookup calibration input = -5 mode interpolated give output\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"abstained\":true"),
        "below domain abstains: {out}"
    );
    assert!(
        out.contains("below_table_domain"),
        "names the below-domain reason: {out}"
    );
    assert!(!out.contains("\"output\":\""), "no fabricated value: {out}");
}

#[test]
fn interpolated_lookup_above_domain_abstains() {
    let dir = scratch("above");
    // input = 30 is above the highest breakpoint (25) — nothing to interpolate up
    // toward, so it abstains rather than extrapolate.
    let src =
        format!("{CALIBRATION}\n? lookup calibration input = 30 mode interpolated give output\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"abstained\":true"),
        "above domain abstains: {out}"
    );
    assert!(
        out.contains("above_table_domain"),
        "names the above-domain reason: {out}"
    );
    assert!(
        !out.contains("\"output\":\""),
        "no extrapolated value: {out}"
    );
}

// ---------------------------------------------------------------------------
// (e) Second segment — interpolation picks the correct bracketing pair.
// ---------------------------------------------------------------------------

#[test]
fn interpolated_lookup_uses_the_correct_segment() {
    let dir = scratch("seg2");
    // input = 15 sits in the SECOND segment, (10 → 100)..(25 → 250):
    // 100 + (250-100)*(15-10)/(25-10) = 100 + 150*5/15 = 100 + 50 = 150.
    let src =
        format!("{CALIBRATION}\n? lookup calibration input = 15 mode interpolated give output\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"output\":\"150\""),
        "second-segment blend 150: {out}"
    );
    // The lower bracket must be the 10-row, not the 0-row.
    assert!(
        out.contains("\"input\":\"10\""),
        "lower bracket is the 10 breakpoint: {out}"
    );
}
