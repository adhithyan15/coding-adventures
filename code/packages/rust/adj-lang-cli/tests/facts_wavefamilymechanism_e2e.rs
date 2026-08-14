//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/wave-family-mechanism.adj`) driven through
//! the built CLI: a native `table` recording, for the two wave families
//! already tabled in `wave-types.adj`, the MECHANISM the same already-
//! quoted NASA sentence contrasts them by -- a sibling decoding the
//! mechanism half of an already-verified quote, keyed by FAMILY rather
//! than by individual wave. Resolves forward and backward recall queries
//! with the source's citation, plus honest abstention on an individual
//! wave name (which belongs to the parent's own key set) -- 0 model
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
    let dir = std::env::temp_dir().join(format!("adjcli_wavefamilymechanism_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/wave-family-mechanism.adj");
    std::fs::copy(&src, dir.join("wave-family-mechanism.adj"))
        .expect("copy shipped wave-family-mechanism.adj");
}

#[test]
fn wave_family_mechanism_recalls_electromagnetic_with_citation() {
    let dir = scratch("em");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-family-mechanism.adj\"\n\
         ? wave_family_mechanism(electromagnetic, $Mechanism)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"wave_family_mechanism(electromagnetic, oscillations_of_electric_and_magnetic_fields)\""),
        "electromagnetic should recall its cited mechanism: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn wave_family_mechanism_backward_recalls_mechanical_for_oscillations_of_matter() {
    let dir = scratch("mech");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-family-mechanism.adj\"\n\
         ? wave_family_mechanism($Family, oscillations_of_matter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"wave_family_mechanism(mechanical, oscillations_of_matter)\""),
        "mechanical should be the only recalled oscillations-of-matter family: {out}"
    );
    assert!(
        !out.contains("wave_family_mechanism(electromagnetic, oscillations_of_matter)"),
        "electromagnetic's cited mechanism is fields, not matter: {out}"
    );
}

#[test]
fn wave_family_mechanism_abstains_on_individual_wave_name() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-family-mechanism.adj\"\n\
         ? wave_family_mechanism(radio, $MechanismRadio)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "radio is an individual wave name (parent wave-types.adj's key), not a family -- honest abstention expected: {out}"
    );
}
