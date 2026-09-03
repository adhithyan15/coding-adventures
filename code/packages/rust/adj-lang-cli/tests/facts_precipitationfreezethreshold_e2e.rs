//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/precipitation-freeze-threshold.adj`)
//! driven through the built CLI: a native `table` recording, for freezing
//! rain already tabled in `precipitation-types.adj`, the numeric freeze
//! threshold in degrees Fahrenheit an already-cited NWS sentence states --
//! a sibling decoding the sentence's numeric clause as its own row instead
//! of folding the whole clause into one compound `form` atom. Resolves
//! forward and backward recall queries with the source's citation, plus
//! honest abstention on rain (whose cited span states no numeric freeze
//! threshold) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_precipfreezethreshold_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("meteorology/precipitation-freeze-threshold.adj");
    std::fs::copy(&src, dir.join("precipitation-freeze-threshold.adj"))
        .expect("copy shipped precipitation-freeze-threshold.adj");
}

#[test]
fn precipitation_freeze_threshold_recalls_freezing_rain_with_citation() {
    let dir = scratch("freezingrain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-freeze-threshold.adj\"\n\
         ? precipitation_freeze_threshold_f(freezing_rain, $TemperatureF)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_freeze_threshold_f(freezing_rain, 32)\""),
        "freezing_rain should recall 32: {out}"
    );
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn precipitation_freeze_threshold_backward_recalls_freezing_rain_for_32() {
    let dir = scratch("thirtytwo");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-freeze-threshold.adj\"\n\
         ? precipitation_freeze_threshold_f($Precip, 32)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_freeze_threshold_f(freezing_rain, 32)\""),
        "freezing_rain should be the only recalled type for 32: {out}"
    );
}

#[test]
fn precipitation_freeze_threshold_abstains_on_rain() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-freeze-threshold.adj\"\n\
         ? precipitation_freeze_threshold_f(rain, $TemperatureRain)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "rain's cited span states no numeric freeze threshold -- honest abstention expected: {out}"
    );
}

const PRECIPITATION_FREEZE_THRESHOLD_PIN: &str = r#""bindings":{"TemperatureF":"32"},"citations":[{"source":"Freezing rain occurs when snowflakes descend into a warmer layer of air and melt completely. When these liquid water drops fall through another thin layer of freezing air just above the surface, they don't have enough time to refreeze before reaching the ground. Because they are “supercooled,” they instantly refreeze upon contact with anything that that is at or below freezing (32 degrees F), creating a glaze of ice on the ground, trees, power lines, or other objects.","locator":"https://www.weather.gov/iwx/sleetvsfreezingrain","trust":"authoritative""#;

#[test]
fn precipitation_freeze_threshold_citation_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-freeze-threshold.adj\"\n? precipitation_freeze_threshold_f(freezing_rain, $TemperatureF)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The value opened with an ellipsis and stopped at "a glaze of ice.",
    // eliding the sentence's subject clause and truncating its tail. Restored
    // whole from the page.
    //
    // "anything that that is at or below freezing" REPRODUCES A TYPO IN THE
    // SOURCE. It is kept, doubled word and all: a citation that silently
    // corrects its page is the same defect class as one that silently tidies
    // it, and this whole effort exists because of the second kind.
    assert!(
        out.contains(PRECIPITATION_FREEZE_THRESHOLD_PIN),
        "the freezing-rain citation is the page's whole sentence: {out}"
    );
}
