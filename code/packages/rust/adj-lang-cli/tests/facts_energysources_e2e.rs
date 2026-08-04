//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-sources.adj`) driven through the built
//! CLI: a native `table` of common energy sources → whether each is renewable
//! or nonrenewable resolves binding-query recalls (forward AND backward) with
//! the source's U.S. EIA "Energy Explained" citation, and abstains on a word
//! that is not one of the enumerated sources (magnetic) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn physics_energy_sources_recall_binds_class_with_citation() {
    let dir = scratch("energysources");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/energy-sources.adj");
    std::fs::copy(&src, dir.join("energy-sources.adj")).expect("copy shipped energy-sources.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-sources.adj\"\n\
         ? energy_source_class(solar, $Class)\n\
         ? energy_source_class(coal, $Class)\n\
         ? energy_source_class(nuclear, $Class)\n\
         ? energy_source_class($Source, renewable)\n\
         ? energy_source_class(magnetic, $Class)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Solar is one of the five major renewable sources; coal and nuclear are in
    // the nonrenewable list — the recalled classes (forward binds).
    assert!(
        out.contains("\"Class\":\"renewable\""),
        "solar → renewable: {out}"
    );
    assert!(
        out.contains("\"Class\":\"nonrenewable\""),
        "coal / nuclear → nonrenewable: {out}"
    );
    // The relation runs BACKWARD: bind the class `renewable`, recall a source in
    // that family (e.g. solar).
    assert!(
        out.contains("\"Source\":\"solar\""),
        "renewable → solar (reverse recall): {out}"
    );
    // The answer carries the U.S. EIA citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("eia.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Magnetic energy is not one of the sources this EIA page enumerates —
    // honest abstention, never a fabricated class.
    assert!(out.contains("\"abstained\":true"), "magnetic abstains: {out}");
}
