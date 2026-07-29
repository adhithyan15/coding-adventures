//! End-to-end test for the metrology SI-BASE-UNITS facts library
//! (`adj-facts-stdlib/metrology/si-base-units.adj`) driven through the built CLI:
//! a native `table` of base-quantity → unit → symbol resolves a binding-query
//! recall with the NIST citation, runs the relation backwards (unit → quantity),
//! and abstains on anything that is not one of the seven base quantities — 0
//! model calls, never a fabricated unit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssi_{tag}_{}", std::process::id()));
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

fn with_case(dir: &Path, body: &str) -> PathBuf {
    let src = facts_stdlib().join("metrology/si-base-units.adj");
    std::fs::copy(&src, dir.join("si-base-units.adj")).expect("copy shipped si-base-units.adj");
    let p = dir.join("case.adj");
    std::fs::write(&p, format!("import \"si-base-units.adj\"\n{body}")).unwrap();
    p
}

#[test]
fn si_base_unit_forward_recall_binds_unit_and_symbol_with_citation() {
    let dir = scratch("forward");
    let p = with_case(
        &dir,
        "? si_base_unit(mass, $Unit, $Symbol)\n\
         ? si_base_unit(temperature, $Unit, $Symbol)\n",
    );

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Mass -> kilogram (kg); temperature -> kelvin (K).
    assert!(
        out.contains("\"Unit\":\"kilogram\""),
        "mass binds the kilogram: {out}"
    );
    assert!(out.contains("kg"), "mass carries the symbol kg: {out}");
    assert!(
        out.contains("\"Unit\":\"kelvin\""),
        "temperature binds the kelvin: {out}"
    );
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation: {out}"
    );
}

#[test]
fn si_base_unit_runs_backwards_from_unit_to_quantity() {
    let dir = scratch("reverse");
    // Given the unit `second`, recall the base quantity it measures — time.
    let p = with_case(&dir, "? si_base_unit($Quantity, second, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Quantity\":\"time\""),
        "the second measures time (reverse lookup): {out}"
    );
}

#[test]
fn a_non_base_quantity_abstains_rather_than_inventing_a_unit() {
    let dir = scratch("abstain");
    // Luminance is a real photometric quantity but NOT one of the seven SI base
    // quantities — the table must abstain, never fabricate a unit.
    let p = with_case(&dir, "? si_base_unit(luminance, $Unit, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a non-base quantity abstains: {out}"
    );
}
