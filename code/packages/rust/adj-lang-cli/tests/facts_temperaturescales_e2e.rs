//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/temperature-scales.adj`) driven through the built
//! CLI: a native `table` of temperature reference point → numeric value resolves
//! a binding-query recall with the NIST "SI Units – Temperature" citation, runs
//! the relation backward (value → point), and abstains on a point the source
//! does not fix (the Fahrenheit freezing point of water) — 0 model calls.

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
fn physics_temperature_scales_recall_binds_value_with_citation() {
    let dir = scratch("temperaturescales");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/temperature-scales.adj");
    std::fs::copy(&src, dir.join("temperature-scales.adj"))
        .expect("copy shipped temperature-scales.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"temperature-scales.adj\"\n\
         ? temperature_reference_point(water_freezes_celsius, $V)\n\
         ? temperature_reference_point(water_boils_celsius, $V)\n\
         ? temperature_reference_point(zero_celsius_in_kelvin, $V)\n\
         ? temperature_reference_point($P, 100)\n\
         ? temperature_reference_point(water_freezes_fahrenheit, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each reference point to the number NIST fixes it at,
    // each a plain number (including the decimal 273.15).
    assert!(out.contains("\"V\":\"0\""), "water_freezes_celsius → 0: {out}");
    assert!(out.contains("\"V\":\"100\""), "water_boils_celsius → 100: {out}");
    assert!(
        out.contains("\"V\":\"273.15\""),
        "zero_celsius_in_kelvin → 273.15: {out}"
    );
    // The relation runs BACKWARD: the value 100 recalls water_boils_celsius.
    assert!(
        out.contains("\"P\":\"water_boils_celsius\""),
        "100 → water_boils_celsius (reverse recall): {out}"
    );
    // The answer carries the NIST locator + trust tier as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The Fahrenheit freezing point is not fixed by this NIST page — honest
    // abstention, never a fabricated temperature.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded reference point abstains: {out}"
    );
}
