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

/// The table's `source`, as the NIST page writes it: U+00A0 after "value of"
/// and after "where the", U+2212 in "s−1", and "∆νCs".
const ENVELOPE: &str = "The meter is defined by taking the fixed numerical value of\u{a0}the speed of light in vacuum c to be 299,792,458 when expressed in the unit m s\u{2212}1, where the\u{a0}second is defined in terms of \u{2206}\u{3bd}Cs.";

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
    // The answer carries the NIST citation as its proof -- as ONE contiguous
    // run (sentence, locator, tier, no corroborations), not a host needle and a
    // trust needle that could each match a different citation (#15209).
    assert!(
        out.contains(&format!(
            "\"source\":\"{ENVELOPE}\",\"locator\":\"https://www.nist.gov/si-redefinition/definitions-si-base-units\",\"trust\":\"authoritative\",\"corroborations\":[]"
        )),
        "carries the whole NIST citation at authoritative trust: {out}"
    );
    // The page writes U+00A0 at two places the span used to have an ordinary
    // space; with those spaces the span occurred zero times on the page.
    assert_eq!(ENVELOPE.matches('\u{a0}').count(), 2, "the envelope carries the page's two U+00A0");
    assert!(
        !out.contains(&ENVELOPE.replace('\u{a0}', " ")),
        "the ordinary-space form, which the page never writes, reaches no answer: {out}"
    );
    // The Newtonian gravitational constant is NOT one of the exact defining SI
    // constants and is absent from the table — honest abstention, never a
    // fabricated value.
    assert!(out.contains("\"abstained\":true"), "unknown constant abstains: {out}");
}
