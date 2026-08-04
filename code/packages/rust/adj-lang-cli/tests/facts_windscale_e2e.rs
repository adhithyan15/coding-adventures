//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/wind-scale.adj`) driven through the built
//! CLI: a native `table` of Beaufort force number → descriptive name resolves a
//! binding-query recall with the NWS "Beaufort Wind Scale" citation, runs the
//! relation backward (name → force), and abstains on a force the scale does not
//! define (13, off the top of the scale) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstemp_{tag}_{}", std::process::id()));
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

#[test]
fn meteorology_wind_scale_recall_binds_name_with_citation() {
    let dir = scratch("windscale");
    // Copy the shipped meteorology table beside the entry program and import it.
    let src = facts_stdlib().join("meteorology/wind-scale.adj");
    std::fs::copy(&src, dir.join("wind-scale.adj")).expect("copy shipped wind-scale.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wind-scale.adj\"\n\
         ? beaufort_force_name(0, $Name)\n\
         ? beaufort_force_name(6, $Name)\n\
         ? beaufort_force_name(12, $Name)\n\
         ? beaufort_force_name($F, hurricane)\n\
         ? beaufort_force_name(13, $Name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each Beaufort force number to the name NWS gives it.
    assert!(out.contains("\"Name\":\"calm\""), "force 0 → calm: {out}");
    assert!(
        out.contains("\"Name\":\"strong_breeze\""),
        "force 6 → strong_breeze: {out}"
    );
    assert!(
        out.contains("\"Name\":\"hurricane\""),
        "force 12 → hurricane: {out}"
    );
    // The relation runs BACKWARD: the name hurricane recalls force 12.
    assert!(
        out.contains("\"F\":\"12\""),
        "hurricane → 12 (reverse recall): {out}"
    );
    // The answer carries the NWS locator + trust tier as its proof.
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Beaufort force 13 is off the top of the scale — honest abstention, never a
    // fabricated name.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded force number abstains: {out}"
    );
}
