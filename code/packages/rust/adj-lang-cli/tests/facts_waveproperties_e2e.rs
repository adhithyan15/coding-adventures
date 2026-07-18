//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/wave-properties.adj`) driven through the built
//! CLI: a native `table` of the four measurable wave properties → what each one
//! is / measures resolves binding-query recalls (forward AND backward) with the
//! source's OpenStax Physics citation, and abstains on a word that is not one of
//! the four measurable wave properties (color) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsw_{tag}_{}", std::process::id()));
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
fn physics_wave_properties_recall_binds_definition_with_citation() {
    let dir = scratch("waveproperties");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/wave-properties.adj");
    std::fs::copy(&src, dir.join("wave-properties.adj")).expect("copy shipped wave-properties.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-properties.adj\"\n\
         ? wave_property(wavelength, $Definition)\n\
         ? wave_property(frequency, $Definition)\n\
         ? wave_property(amplitude, $Definition)\n\
         ? wave_property($Property, time_for_one_cycle)\n\
         ? wave_property(color, $Definition)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Wavelength is the distance between identical parts, frequency is waves
    // per second, amplitude is the max displacement from rest — the recalled
    // definitions (forward binds).
    assert!(
        out.contains("\"Definition\":\"distance_between_identical_parts\""),
        "wavelength → distance_between_identical_parts: {out}"
    );
    assert!(
        out.contains("\"Definition\":\"waves_per_second\""),
        "frequency → waves_per_second: {out}"
    );
    assert!(
        out.contains("\"Definition\":\"max_displacement_from_rest\""),
        "amplitude → max_displacement_from_rest: {out}"
    );
    // The relation runs BACKWARD: bind the definition `time_for_one_cycle`,
    // recall its property.
    assert!(
        out.contains("\"Property\":\"period\""),
        "time_for_one_cycle → period (reverse recall): {out}"
    );
    // The answer carries the OpenStax Physics citation as its proof, at the
    // `consensus` trust tier for a peer-reviewed open textbook.
    assert!(
        out.contains("openstax.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // Color is a perception that depends on wavelength, not one of the four
    // measurable wave properties — honest abstention, never a fabricated
    // definition.
    assert!(out.contains("\"abstained\":true"), "color abstains: {out}");
}
