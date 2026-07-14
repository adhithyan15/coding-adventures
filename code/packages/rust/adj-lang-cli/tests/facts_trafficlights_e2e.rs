//! End-to-end test for the transportation FACTS library
//! (`adj-facts-stdlib/transportation/traffic-lights.adj`) driven through the built CLI:
//! a native `table` of steady-signal color → meaning resolves a binding-query recall
//! with the source's citation, and abstains on a non-signal color — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn transportation_traffic_lights_recall_binds_meaning_with_citation() {
    let dir = scratch("trafficlights");
    // Copy the shipped transportation table beside the entry program and import it.
    let src = facts_stdlib().join("transportation/traffic-lights.adj");
    std::fs::copy(&src, dir.join("traffic-lights.adj")).expect("copy shipped traffic-lights.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"traffic-lights.adj\"\n\
         ? traffic_light_meaning(red, $Meaning)\n\
         ? traffic_light_meaning(green, $Meaning)\n\
         ? traffic_light_meaning(blue, $Meaning)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Red means stop; green means proceed — the recalled meanings.
    assert!(out.contains("\"Meaning\":\"stop\""), "red → stop: {out}");
    assert!(out.contains("\"Meaning\":\"proceed\""), "green → proceed: {out}");
    // The answer carries the FHWA MUTCD citation (locator + trust) as its proof.
    assert!(
        out.contains("mutcd.fhwa.dot.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Blue is not a steady signal color — honest abstention, never a fabricated meaning.
    assert!(out.contains("\"abstained\":true"), "blue abstains: {out}");
}
