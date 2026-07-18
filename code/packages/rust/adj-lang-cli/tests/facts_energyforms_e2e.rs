//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-forms.adj`) driven through the built CLI:
//! a native `table` of energy form → defining token resolves a binding query
//! recall with the EIA citation, runs backward (token → form), and abstains on
//! something that is not one of the enumerated forms — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsef_{tag}_{}", std::process::id()));
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
fn physics_energy_forms_recall_binds_token_with_citation() {
    let dir = scratch("energyforms");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/energy-forms.adj");
    std::fs::copy(&src, dir.join("energy-forms.adj")).expect("copy shipped energy-forms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-forms.adj\"\n\
         ? energy_form_token(chemical, $T)\n\
         ? energy_form_token(nuclear, $T)\n\
         ? energy_form_token(electrical, $T)\n\
         ? energy_form_token($F, heat)\n\
         ? energy_form_token(magnetic, $T)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each form to the token EIA uses to define it.
    assert!(out.contains("\"T\":\"bonds\""), "chemical binds to bonds: {out}");
    assert!(
        out.contains("energy_form_token(chemical, bonds)"),
        "chemical is governing-bound to bonds: {out}"
    );
    assert!(
        out.contains("energy_form_token(nuclear, nucleus)"),
        "nuclear is governing-bound to nucleus: {out}"
    );
    assert!(
        out.contains("energy_form_token(electrical, electrons)"),
        "electrical is governing-bound to electrons: {out}"
    );
    // The relation runs BACKWARD: bind the token, recall the form.
    assert!(
        out.contains("energy_form_token(thermal, heat)"),
        "reverse recall binds F=thermal from heat: {out}"
    );
    // The answer carries the EIA locator + trust tier as its proof.
    assert!(
        out.contains("eia.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Magnetic energy is NOT one of the enumerated forms — honest abstention.
    assert!(out.contains("\"abstained\":true"), "magnetic abstains: {out}");
}
