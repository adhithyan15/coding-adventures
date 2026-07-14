//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/physical-constants.adj`) driven through the built
//! CLI: a native `table` of physical-constant → exact value resolves a
//! binding-query recall with the source's NIST citation, resolves the reverse
//! (value → constant name), and abstains on a constant not in the table —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsc_{tag}_{}", std::process::id()));
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
fn physics_constants_recall_binds_exact_values_with_nist_citation() {
    let dir = scratch("constants");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/physical-constants.adj");
    std::fs::copy(&src, dir.join("physical-constants.adj")).expect("copy shipped constants table");
    std::fs::write(
        dir.join("case.adj"),
        "import \"physical-constants.adj\"\n\
         ? physical_constant(speed_of_light, $V)\n\
         ? physical_constant(avogadro_constant, $V)\n\
         ? physical_constant($C, 299792458)\n\
         ? physical_constant(gravitational_constant, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // Forward recall: the speed of light in vacuum is exactly 299792458 m/s.
    assert!(out.contains("\"V\":\"299792458\""), "speed_of_light -> 299792458: {out}");
    // A scientific-notation literal is kept as its EXACT decimal expansion:
    // Avogadro's number 6.02214076e23 -> 602214076 followed by fifteen zeros.
    assert!(
        out.contains("\"V\":\"602214076000000000000000\""),
        "avogadro_constant -> exact expansion of 6.02214076e23: {out}"
    );
    // Reverse recall: the value 299792458 binds back to the constant's name.
    assert!(
        out.contains("\"C\":\"speed_of_light\""),
        "reverse recall 299792458 -> speed_of_light: {out}"
    );
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov/si-redefinition")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation at authoritative trust: {out}"
    );
    // The Newtonian gravitational constant is NOT one of the exact defining SI
    // constants and is absent from the table — honest abstention, never a
    // fabricated value.
    assert!(out.contains("\"abstained\":true"), "unknown constant abstains: {out}");
}
