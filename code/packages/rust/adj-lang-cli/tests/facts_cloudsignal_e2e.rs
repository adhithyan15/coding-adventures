//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/cloud-signal.adj`) driven through the
//! built CLI: a native `table` recording, for cirrus and stratus clouds
//! already tabled in `cloud-type.adj`, each individual weather signal an
//! already-cited NWS sentence lists for it -- a sibling decoding each
//! listed signal as its own row instead of folding the whole clause into
//! one compound `weather_indication` atom. Resolves forward (multi-answer)
//! and backward recall queries with the source's citation, plus honest
//! abstention on cumulonimbus (whose cited span names only one signal) --
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cloudsignal_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/cloud-signal.adj");
    std::fs::copy(&src, dir.join("cloud-signal.adj"))
        .expect("copy shipped cloud-signal.adj");
}

#[test]
fn cloud_signal_recalls_stratus_signals_with_citation() {
    let dir = scratch("stratus");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-signal.adj\"\n\
         ? cloud_signal(stratus, $Signal)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"cloud_signal(stratus, precipitation_free)\""),
        "stratus should recall precipitation_free: {out}"
    );
    assert!(
        out.contains("\"term\":\"cloud_signal(stratus, light_precipitation)\""),
        "stratus should recall light_precipitation: {out}"
    );
    assert!(
        out.contains("\"term\":\"cloud_signal(stratus, drizzle)\""),
        "stratus should recall drizzle: {out}"
    );
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn cloud_signal_backward_recalls_cirrus_for_upper_level_jet_streak() {
    let dir = scratch("jetstreak");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-signal.adj\"\n\
         ? cloud_signal($Cloud, upper_level_jet_streak)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"cloud_signal(cirrus, upper_level_jet_streak)\""),
        "cirrus should be the only recalled cloud for upper_level_jet_streak: {out}"
    );
}

#[test]
fn cloud_signal_abstains_on_cumulonimbus() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-signal.adj\"\n\
         ? cloud_signal(cumulonimbus, $SignalCumulonimbus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cumulonimbus's cited span names only one signal, no listed alternatives -- honest abstention expected: {out}"
    );
}
