//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/element-groups.adj`) driven through the built
//! CLI: a native `table` of common element → periodic-table group family
//! resolves binding-query recalls (forward and backward) with the source's
//! Wikipedia citation, and abstains on an element not in the table (gold) —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factseg_{tag}_{}", std::process::id()));
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
fn chemistry_element_group_family_recall_binds_family_with_citation() {
    let dir = scratch("elementgroups");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/element-groups.adj");
    std::fs::copy(&src, dir.join("element-groups.adj")).expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n\
         ? element_group_family(sodium, $Family)\n\
         ? element_group_family(chlorine, $Family)\n\
         ? element_group_family(iron, $Family)\n\
         ? element_group_family($E, noble_gas)\n\
         ? element_group_family(gold, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Sodium is an alkali metal, chlorine a halogen, iron a transition metal —
    // the recalled families (forward binds).
    assert!(
        out.contains("\"Family\":\"alkali_metal\""),
        "sodium → alkali_metal: {out}"
    );
    assert!(
        out.contains("\"Family\":\"halogen\""),
        "chlorine → halogen: {out}"
    );
    assert!(
        out.contains("\"Family\":\"transition_metal\""),
        "iron → transition_metal: {out}"
    );
    // The relation runs BACKWARD: bind the family noble_gas, recall an element in
    // it — helium, the first noble gas in the table.
    assert!(
        out.contains("\"E\":\"helium\""),
        "noble_gas → helium (reverse recall into the noble gases): {out}"
    );
    // The answer carries the Wikipedia citation as its proof, at consensus trust
    // (a secondary encyclopedia reference, honestly tiered).
    assert!(
        out.contains("en.wikipedia.org/wiki/Alkali_metal")
            && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "gold" is not in the table — honest abstention, never a fabricated family.
    assert!(out.contains("\"abstained\":true"), "gold abstains: {out}");
}
