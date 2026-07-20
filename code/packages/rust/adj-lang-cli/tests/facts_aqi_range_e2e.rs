//! End-to-end test for the environment FACTS library
//! (`adj-facts-stdlib/environment/air-quality-index.adj`) driven through the
//! built CLI. This is the first SHIPPED facts table that is a RANGE table
//! (ADJ-TABLES RS-5c) rather than an exact lookup: its rows are the EPA AQI
//! BREAKPOINTS, keyed by each band's minimum index value, and a query for an
//! arbitrary AQI selects the row whose key is the greatest one `<=` the queried
//! value. The test proves, against the shipped artifact:
//!
//!   (a) a MID-BAND value (75) binds `moderate` with the EPA / AirNow citation
//!       at `authoritative` trust, and the audit names the matched breakpoint
//!       (51) — i.e. which bracket it fell into;
//!   (b) an EXACT breakpoint (101) selects the band that STARTS there,
//!       `unhealthy_for_sensitive_groups` — boundaries are not rounded down
//!       across a health threshold — and the open top band still answers (480 →
//!       `hazardous`, the source's "301 and higher");
//!   (c) a BELOW-DOMAIN value (-5) has no breakpoint `<=` it and honestly
//!       ABSTAINS rather than fabricating a category.
//!
//! 0 model calls throughout.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsaqi_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Copy the SHIPPED table beside an entry program so the `import` resolves —
/// the test therefore exercises the artifact we actually ship, not a copy of
/// the table inlined into the test source (which could silently drift).
fn place_at(dir: &Path) {
    let src = facts_stdlib().join("environment/air-quality-index.adj");
    std::fs::copy(&src, dir.join("air-quality-index.adj"))
        .expect("copy shipped air-quality-index.adj");
}

fn write(dir: &Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

fn run(program: &Path) -> (bool, String, String) {
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

// ---------------------------------------------------------------------------
// (a) Mid-band value — 75 is inside 51..100, brackets down to the 51 row.
// ---------------------------------------------------------------------------

#[test]
fn environment_aqi_mid_band_binds_category_with_epa_citation_and_matched_breakpoint() {
    let dir = scratch("midband");
    place_at(&dir);
    write(
        &dir,
        "case.adj",
        "import \"air-quality-index.adj\"\n\
         ? lookup air_quality_index min_aqi = 75 mode range give category\n",
    );

    let (ok, out, err) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"lookups\""), "expected a lookups section: {out}");

    // No row is literally 75; the greatest key <= 75 is 51 → moderate.
    assert!(
        out.contains("\"category\":\"moderate\""),
        "AQI 75 falls in the 51..100 band → moderate: {out}"
    );
    assert!(
        out.contains("\"min_aqi\":\"51\""),
        "audit must name the matched breakpoint (51), i.e. which bracket it fell in: {out}"
    );

    // The answer travels with the EPA / AirNow citation, at the trust tier a
    // primary U.S. government source earns.
    assert!(
        out.contains("airnow.gov/aqi/aqi-basics"),
        "answer must carry the EPA / AirNow locator: {out}"
    );
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "EPA / AirNow is a primary U.S. government source → authoritative: {out}"
    );
    assert!(
        out.contains("\"abstained\":false"),
        "a bracket hit is not an abstention: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Exact breakpoint, and the open top band.
// ---------------------------------------------------------------------------

#[test]
fn environment_aqi_exact_breakpoint_selects_that_band_and_top_band_is_open() {
    let dir = scratch("boundary");
    place_at(&dir);

    // 101 is exactly a breakpoint: the greatest key <= 101 is 101 itself, so it
    // binds the band that STARTS at 101 — NOT `moderate` below it. This is the
    // point the source's prose calls the turn to unhealthy air.
    write(
        &dir,
        "exact.adj",
        "import \"air-quality-index.adj\"\n\
         ? lookup air_quality_index min_aqi = 101 mode range give category\n",
    );
    let (ok, out, err) = run(&dir.join("exact.adj"));
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(
        out.contains("\"category\":\"unhealthy_for_sensitive_groups\""),
        "an exact breakpoint selects the band it starts, not the one below: {out}"
    );
    assert!(
        out.contains("\"min_aqi\":\"101\""),
        "matched key should be the breakpoint itself: {out}"
    );
    assert!(
        !out.contains("\"category\":\"moderate\""),
        "101 must not round down into the moderate band: {out}"
    );

    // The source writes the last band open-ended ("301 and higher"), so a bad
    // wildfire-smoke day of 480 is still grounded as hazardous.
    write(
        &dir,
        "top.adj",
        "import \"air-quality-index.adj\"\n\
         ? lookup air_quality_index min_aqi = 480 mode range give category\n",
    );
    let (ok2, out2, _e2) = run(&dir.join("top.adj"));
    assert!(ok2);
    assert!(
        out2.contains("\"category\":\"hazardous\""),
        "above the last breakpoint the top band is open → hazardous: {out2}"
    );
    assert!(
        out2.contains("\"min_aqi\":\"301\""),
        "the open top band's breakpoint is 301: {out2}"
    );
}

// ---------------------------------------------------------------------------
// (c) Below the domain — honest abstention.
// ---------------------------------------------------------------------------

#[test]
fn environment_aqi_below_domain_abstains_rather_than_fabricating_a_category() {
    let dir = scratch("below");
    place_at(&dir);
    write(
        &dir,
        "case.adj",
        "import \"air-quality-index.adj\"\n\
         ? lookup air_quality_index min_aqi = -5 mode range give category\n",
    );

    let (ok, out, err) = run(&dir.join("case.adj"));
    assert!(ok, "an abstention is a normal answer, not a crash; stderr={err}");
    assert!(
        out.contains("\"abstained\":true"),
        "the AQI domain starts at 0 — below it there is no breakpoint <= the value: {out}"
    );
    assert!(
        !out.contains("\"category\":\""),
        "an abstention binds no band — a negative AQI is not 'extra good' air: {out}"
    );
}
