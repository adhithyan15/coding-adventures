//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/states-of-matter.adj`) driven through the built
//! CLI: a native `table` of state-of-matter -> defining property resolves a
//! binding-query recall with the source's citation, and abstains on a state not
//! in the table — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssom_{tag}_{}", std::process::id()));
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
fn chemistry_states_of_matter_recall_binds_property_with_citation() {
    let dir = scratch("statesofmatter");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/states-of-matter.adj");
    std::fs::copy(&src, dir.join("states-of-matter.adj")).expect("copy shipped states-of-matter.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"states-of-matter.adj\"\n\
         ? matter_state(solid, $Property)\n\
         ? matter_state(gas, $Property)\n\
         ? matter_state(plasma, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A solid holds its own shape; a gas fills its container — the recalled props.
    assert!(out.contains("\"Property\":\"fixed_shape\""), "solid -> fixed_shape: {out}");
    assert!(out.contains("\"Property\":\"fills_container\""), "gas -> fills_container: {out}");
    // The answer carries the NASA citation as its proof.
    assert!(
        out.contains("grc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // plasma is the 4th state and is not shipped here — honest abstention.
    assert!(out.contains("\"abstained\":true"), "plasma abstains: {out}");
}
