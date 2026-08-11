//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/weather-instruments.adj`) driven through
//! the built CLI: a native `table` naming which quantity each of six
//! weather-observing instruments measures, quoted verbatim from NOAA's
//! "Build Your Own Weather Station" education page -- a DIFFERENT
//! "observation and measurement" axis from the already-shipped
//! `chemistry/measuring-tools.adj` (lab tools, not weather instruments),
//! continuing to diversify the science lane. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_weatherinstruments_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/weather-instruments.adj");
    std::fs::copy(&src, dir.join("weather-instruments.adj"))
        .expect("copy shipped weather-instruments.adj");
}

#[test]
fn weather_instrument_recall_binds_the_quantity_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weather-instruments.adj\"\n\
         ? weather_instrument(barometer, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Q\":\"atmospheric_pressure\""),
        "a barometer measures atmospheric pressure: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn weather_instrument_reverse_binds_the_instrument_for_that_quantity() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weather-instruments.adj\"\n\
         ? weather_instrument($I, humidity)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"I\":\"hygrometer\""),
        "a hygrometer measures humidity: {out}"
    );
}

#[test]
fn weather_instrument_abstains_honestly_on_a_non_weather_instrument() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weather-instruments.adj\"\n\
         ? weather_instrument(seismometer, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a seismometer measures earthquakes, not weather, and has no shipped row -- honest abstention, never invented: {out}"
    );
}
