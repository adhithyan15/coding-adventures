//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-conversion-example.adj`) driven through
//! the built CLI: a native `table` naming four everyday processes and which
//! form of energy goes in and which comes out, grounding the U.S. EIA's own
//! "law of conservation of energy" statement (energy is never created or
//! destroyed, only changed from one form to another). 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_energyconversionexample_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/energy-conversion-example.adj");
    std::fs::copy(&src, dir.join("energy-conversion-example.adj"))
        .expect("copy shipped energy-conversion-example.adj");
}

#[test]
fn energy_conversion_example_recall_binds_the_conversion_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-conversion-example.adj\"\n\
         ? energy_conversion_example(wood_burning_in_fireplace, $In, $Out)\n\
         ? energy_conversion_example(car_engine_burning_gasoline, $In, $Out)\n\
         ? energy_conversion_example(solar_photovoltaic_cell, $In, $Out)\n\
         ? energy_conversion_example(bicycle_going_downhill, $In, $Out)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"In\":\"chemical\",\"Out\":\"thermal\""),
        "burning wood in a fireplace converts chemical energy to thermal energy: {out}"
    );
    assert!(
        out.contains("\"In\":\"chemical\",\"Out\":\"mechanical\""),
        "a car engine burning gasoline converts chemical energy to mechanical energy: {out}"
    );
    assert!(
        out.contains("\"In\":\"radiant\",\"Out\":\"electrical\""),
        "a solar photovoltaic cell converts radiant energy to electrical energy: {out}"
    );
    assert!(
        out.contains("\"In\":\"gravitational\",\"Out\":\"motion\""),
        "riding a bicycle downhill converts gravitational energy to motion energy: {out}"
    );
    assert!(
        out.contains("eia.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the U.S. EIA citation at authoritative trust: {out}"
    );
}

#[test]
fn energy_conversion_example_reverse_binds_every_process_from_the_same_energy_in() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-conversion-example.adj\"\n\
         ? energy_conversion_example($P, chemical, $Out)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"wood_burning_in_fireplace\"") && out.contains("\"Out\":\"thermal\""),
        "fireplace burning is one of the two chemical-energy processes: {out}"
    );
    assert!(
        out.contains("\"P\":\"car_engine_burning_gasoline\"") && out.contains("\"Out\":\"mechanical\""),
        "the car engine is the OTHER chemical-energy process, with a DIFFERENT output: {out}"
    );
}

#[test]
fn energy_conversion_example_abstains_honestly_on_a_process_outside_the_cited_pages() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-conversion-example.adj\"\n\
         ? energy_conversion_example(toaster, $In, $Out)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a toaster is a real everyday energy-conversion device, but outside the four processes \
         citably stated across the two EIA pages this table grounds -- honest abstention, never \
         invented: {out}"
    );
}
