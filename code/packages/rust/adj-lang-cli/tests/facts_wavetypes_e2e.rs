//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/wave-types.adj`) driven through the built CLI:
//! a native `table` of named wave → wave family (mechanical / electromagnetic)
//! resolves a binding query recall with the NASA citation, runs backward
//! (family → wave), and abstains on a wave the source never classifies
//! (a seismic wave) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factswt_{tag}_{}", std::process::id()));
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
fn physics_wave_types_recall_binds_family_with_citation() {
    let dir = scratch("wavetypes");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/wave-types.adj");
    std::fs::copy(&src, dir.join("wave-types.adj")).expect("copy shipped wave-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-types.adj\"\n\
         ? wave_family(sound, $F)\n\
         ? wave_family(radio, $F)\n\
         ? wave_family(gamma, $F)\n\
         ? wave_family($W, electromagnetic)\n\
         ? wave_family(seismic, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each named wave to the family NASA sorts it into.
    assert!(out.contains("\"F\":\"mechanical\""), "sound binds to mechanical: {out}");
    assert!(
        out.contains("\"F\":\"electromagnetic\""),
        "radio binds to electromagnetic: {out}"
    );
    assert!(
        out.contains("wave_family(sound, mechanical)"),
        "sound is governing-bound to mechanical: {out}"
    );
    assert!(
        out.contains("wave_family(radio, electromagnetic)"),
        "radio is governing-bound to electromagnetic: {out}"
    );
    assert!(
        out.contains("wave_family(gamma, electromagnetic)"),
        "gamma is governing-bound to electromagnetic: {out}"
    );
    // The relation runs BACKWARD: bind the family, recall a wave that belongs to it.
    assert!(
        out.contains("\"W\":\"gamma\""),
        "reverse recall binds W=gamma from electromagnetic: {out}"
    );
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A seismic wave is never sorted into a family by the source — honest abstention.
    assert!(out.contains("\"abstained\":true"), "seismic abstains: {out}");
}
