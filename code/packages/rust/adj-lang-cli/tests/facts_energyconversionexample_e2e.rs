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
    // First, the welded value must not come back. This names the DEFECT
    // rather than a citation, so no duplicate elsewhere in the output can
    // satisfy it on the real one's behalf. It is asserted BEFORE the citation
    // object deliberately: with the order the other way round a RESTORE mutant
    // trips the citation assertion first and this one is never observed to
    // fire at all, which makes it decoration rather than a test.
    assert!(
        !out.contains("in a car's engine. When a person rides a bicycle"),
        "the two EIA spans are not welded back into one: {out}"
    );
    // The pin here used to be a HOSTNAME and a TRUST TIER:
    // `contains("eia.gov") && contains(trust)`. Both were satisfied by the
    // welded 257-character `cites` that #13934 installment 4n split -- a
    // string that occurs ZERO times on the page it named -- exactly as happily
    // as by the two real spans that replaced it. The oceans and reference-lines
    // pins had the same shape and the same hole.
    //
    // The needle is the whole citation object as the serialiser actually emits
    // it, taken from a real run rather than from memory of the format, and it
    // CLOSES on the corroborations `]`. That bounds the source text, both
    // locators, the trust tier, and the corroboration SET -- so a fabricated
    // `cites` cannot be appended without reddening (#14735). What it does NOT
    // bound is the NUMBER of citation objects in the output: nothing here
    // asserts that `citations` holds exactly one, or that no other citation
    // appears elsewhere. Both happen to hold today, and the lowerer rejects
    // the inputs that would falsify them, which is why no mutant could ever
    // be observed to trip such an assertion.
    //
    // Its string appears EIGHT times in this test's output, at eight distinct
    // JSON paths: `recall/[n]/answers/[0]/citations/[0]` and `.../steps/[0]`
    // for each of the four queries above. That was counted BEFORE trusting the
    // needle, and six mutations of the `.adj` each drove all eight to zero
    // together -- echoes of one field, not the independently-driftable copies
    // of #14745.
    assert!(
        out.contains(
            "\"source\":\"A car engine burns gasoline, converting the chemical energy in gasoline into mechanical energy. Solar photovoltaic cells change radiant energy from the sun into electrical energy.\",\"locator\":\"https://www.eia.gov/energyexplained/what-is-energy/laws-of-energy.php\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"For example, chemical energy is converted to thermal energy when people burn wood in a fireplace or burn gasoline in a car's engine.\",\"locator\":\"https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php\"},{\"source\":\"When a person rides a bicycle down a steep hill and picks up speed, the gravitational energy is converting to motion energy.\",\"locator\":\"https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php\"}]"
        ),
        "carries the U.S. EIA citation with exactly these two corroborations, in this \
         order, and nothing appended after the corroborations `]`: {out}"
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
