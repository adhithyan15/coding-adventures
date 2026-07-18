//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/subatomic-particles.adj`) driven through the
//! built CLI: a native `table` of subatomic particle → electric charge resolves
//! binding-query recalls (forward and backward) with the source's DOE citation,
//! and abstains on a particle not in the table (positron) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssp_{tag}_{}", std::process::id()));
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
fn chemistry_particle_charge_recall_binds_charge_with_citation() {
    let dir = scratch("subatomicparticles");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/subatomic-particles.adj");
    std::fs::copy(&src, dir.join("subatomic-particles.adj"))
        .expect("copy shipped subatomic-particles.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"subatomic-particles.adj\"\n\
         ? particle_charge(proton, $C)\n\
         ? particle_charge(neutron, $C)\n\
         ? particle_charge(electron, $C)\n\
         ? particle_charge($P, negative)\n\
         ? particle_charge(positron, $C)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The proton is positive, the neutron is neutral (no charge), the electron is
    // negative — the recalled charge tokens (forward binds).
    assert!(out.contains("\"C\":\"positive\""), "proton -> positive: {out}");
    assert!(out.contains("\"C\":\"neutral\""), "neutron -> neutral: {out}");
    assert!(out.contains("\"C\":\"negative\""), "electron -> negative: {out}");
    // The relation runs BACKWARD: bind the charge `negative`, recall the particle
    // — the electron, the only negatively charged one of the three.
    assert!(
        out.contains("\"P\":\"electron\""),
        "negative -> electron (reverse recall to the negative particle): {out}"
    );
    // The answer carries the DOE citation as its proof, at authoritative trust
    // (a primary U.S. government science source).
    assert!(
        out.contains("energy.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation at authoritative trust: {out}"
    );
    // "positron" is not in the table — honest abstention, never a fabricated
    // charge.
    assert!(out.contains("\"abstained\":true"), "positron abstains: {out}");
}
