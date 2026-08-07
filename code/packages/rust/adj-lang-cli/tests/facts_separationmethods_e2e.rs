//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/separation-methods.adj`) driven through the
//! built CLI: a native `table` of separation method → the property/basis its
//! source states it separates by resolves binding-query recalls (forward AND
//! backward) with the source's Chemistry LibreTexts citation, and abstains on a
//! method not in the table (centrifugation) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssep_{tag}_{}", std::process::id()));
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
fn chemistry_separation_basis_recall_binds_basis_with_citation() {
    let dir = scratch("separationmethods");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/separation-methods.adj");
    std::fs::copy(&src, dir.join("separation-methods.adj"))
        .expect("copy shipped separation-methods.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"separation-methods.adj\"\n\
         ? separation_basis(filtration, $Basis)\n\
         ? separation_basis(distillation, $Basis)\n\
         ? separation_basis(chromatography, $Basis)\n\
         ? separation_basis($Method, by_particle_size)\n\
         ? separation_basis(centrifugation, $Basis)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Filtration separates by particle size, distillation by volatility, and
    // chromatography because the components move at different rates — the
    // recalled basis values (forward binds).
    assert!(
        out.contains("\"Basis\":\"by_particle_size\""),
        "filtration → by_particle_size: {out}"
    );
    assert!(
        out.contains("\"Basis\":\"by_volatility\""),
        "distillation → by_volatility: {out}"
    );
    assert!(
        out.contains("\"Basis\":\"by_different_rates\""),
        "chromatography → by_different_rates: {out}"
    );
    // The relation runs BACKWARD: bind the basis `by_particle_size`, recall the
    // method that separates on it.
    assert!(
        out.contains("\"Method\":\"filtration\""),
        "by_particle_size → filtration (reverse recall): {out}"
    );
    // The answer carries the Chemistry LibreTexts citation as its proof, at the
    // `consensus` trust tier for a curated open-education teaching resource.
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // Centrifugation is not one of the methods this source lists — honest
    // abstention, never a fabricated basis.
    assert!(
        out.contains("\"abstained\":true"),
        "centrifugation abstains: {out}"
    );
}
