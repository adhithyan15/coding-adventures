//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/season-start-month-number.adj`) driven
//! through the built CLI: a `rule` composing the already-shipped
//! `season_start_month` table (`earth-science/seasons.adj`) with the
//! already-shipped `month_number` table (`calendar/months.adj`, a
//! CROSS-DIRECTORY import via `../calendar/months.adj`, the same shape
//! `mathematics/word-problems.adj` already established) to DERIVE
//! `season_start_month_number($Season, $Number)` -- the first cross-DIRECTORY
//! `rule` composition in this loop's science curriculum sweep (prior rule
//! compositions stayed within one subject directory). 0 answer-time model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_seasonmonth_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Copy BOTH shipped libraries, preserving their real relative directory
/// structure: `season-start-month-number.adj` (in `earth-science/`) imports
/// `seasons.adj` (same dir) and `../calendar/months.adj` (cross-directory),
/// so the entry program must sit at a root that contains both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("earth-science/seasons.adj", "earth-science/seasons.adj"),
        (
            "earth-science/season-start-month-number.adj",
            "earth-science/season-start-month-number.adj",
        ),
        ("calendar/months.adj", "calendar/months.adj"),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn summer_derives_month_six_with_dual_citations() {
    let dir = scratch("summer");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-science/season-start-month-number.adj\"\n\
         ? season_start_month_number(summer, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"N\":\"6\""),
        "summer starts in month 6: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: NOAA
    // (season_start_month) AND the ISO calendar convention (month_number).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derivation is a rule composing two fact steps: {out}"
    );
    assert!(
        out.contains("ncei.noaa.gov") && out.contains("cl.cam.ac.uk"),
        "carries citations from BOTH composed libraries (seasons.adj and months.adj): {out}"
    );
}

#[test]
fn month_twelve_reverse_binds_to_winter() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-science/season-start-month-number.adj\"\n\
         ? season_start_month_number($S, 12)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"winter\""),
        "month 12 is winter's start: {out}"
    );
}

#[test]
fn monsoon_abstains_honestly_as_not_a_tabled_meteorological_season() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-science/season-start-month-number.adj\"\n\
         ? season_start_month_number(monsoon, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"monsoon\" has no shipped row -- honest abstention, never invented: {out}"
    );
}
