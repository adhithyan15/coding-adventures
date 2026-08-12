//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/cloud-type.adj`) driven through the built
//! CLI: a native `table` naming three cloud types and the weather each one
//! indicates, per the National Weather Service's "Cloud Classification"
//! education page. The ELEVENTH science slice in this loop's sweep. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cloudtype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/cloud-type.adj");
    std::fs::copy(&src, dir.join("cloud-type.adj")).expect("copy shipped cloud-type.adj");
}

#[test]
fn cloud_type_recall_binds_the_weather_indication_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-type.adj\"\n\
         ? cloud_type(cirrus, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"W\":\"approaching_warm_front\""),
        "cirrus indicates an approaching warm front: {out}"
    );
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn cloud_type_reverse_binds_the_cloud_for_that_weather_indication() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-type.adj\"\n\
         ? cloud_type($C, heavy_rain_thunderstorm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"cumulonimbus\""),
        "cumulonimbus indicates heavy rain and thunderstorms: {out}"
    );
}

#[test]
fn cloud_type_abstains_honestly_on_an_untabled_cloud() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-type.adj\"\n\
         ? cloud_type(altocumulus, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "altocumulus is a real cloud type but not one this source tables -- honest abstention, never invented: {out}"
    );
}
