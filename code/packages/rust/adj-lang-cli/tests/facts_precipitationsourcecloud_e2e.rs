//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/precipitation-source-cloud.adj`) driven
//! through the built CLI: a native `table` recording, for hail already
//! tabled in `precipitation-types.adj`/`precipitation-minimum-diameter.adj`/
//! `precipitation-alternate-form.adj`, the originating cloud type an
//! already-cited NWS Glossary sentence names -- a sibling decoding the
//! sentence's cloud-source clause as its own row. Resolves forward and
//! backward recall queries with the source's citation, plus honest
//! abstention on rain (whose cited span names no originating cloud) --
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
    let dir = std::env::temp_dir().join(format!("adjcli_precipsourcecloud_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/precipitation-source-cloud.adj");
    std::fs::copy(&src, dir.join("precipitation-source-cloud.adj"))
        .expect("copy shipped precipitation-source-cloud.adj");
}

#[test]
fn precipitation_source_cloud_recalls_hail_cloud_with_citation() {
    let dir = scratch("hail");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-source-cloud.adj\"\n\
         ? precipitation_source_cloud(hail, $Cloud)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_source_cloud(hail, cumulonimbus)\""),
        "hail should recall cumulonimbus: {out}"
    );
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn precipitation_source_cloud_backward_recalls_hail_for_cumulonimbus() {
    let dir = scratch("cumulonimbus");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-source-cloud.adj\"\n\
         ? precipitation_source_cloud($Precip, cumulonimbus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_source_cloud(hail, cumulonimbus)\""),
        "hail should be the only recalled type for cumulonimbus: {out}"
    );
}

#[test]
fn precipitation_source_cloud_abstains_on_rain() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-source-cloud.adj\"\n\
         ? precipitation_source_cloud(rain, $CloudRain)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "rain's cited span names no originating cloud -- honest abstention expected: {out}"
    );
}
