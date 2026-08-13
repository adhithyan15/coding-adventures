//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/chemical-bond-family.adj`) driven through the
//! built CLI: a native `table` of bond type -> primary/secondary family
//! resolves a binding-query recall with the source's LibreTexts citation,
//! runs the relation backward as a genuine one-to-many recall (primary ->
//! ionic ; covalent ; metallic), and abstains on a bond type this source
//! does not classify into either family (hydrogen) -- 0 model calls. A
//! sibling to the already-shipped `chemical-bonds.adj` (which recalls only
//! each bond's single defining TOKEN, not its family) -- discovered via a
//! header-revisit of that table's own already-cited LibreTexts sentence.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_bondfamily_{tag}_{}", std::process::id()));
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
fn chemistry_bond_family_recall_binds_family_with_citation() {
    let dir = scratch("bondfamily");
    let src = facts_stdlib().join("chemistry/chemical-bond-family.adj");
    std::fs::copy(&src, dir.join("chemical-bond-family.adj"))
        .expect("copy shipped chemical-bond-family.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemical-bond-family.adj\"\n\
         ? bond_family(ionic, $Family)\n\
         ? bond_family(van_der_waals, $Family)\n\
         ? bond_family($Bond, primary)\n\
         ? bond_family(hydrogen, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // ionic is primary bonding; van der Waals is secondary.
    assert!(
        out.contains("\"Family\":\"primary\""),
        "ionic -> primary: {out}"
    );
    assert!(
        out.contains("bond_family(van_der_waals, secondary)"),
        "van_der_waals -> secondary: {out}"
    );
    // The relation runs BACKWARD as a genuine one-to-many recall: binding
    // `primary` recalls all THREE bonds the source classifies that way.
    for bond in ["ionic", "covalent", "metallic"] {
        assert!(
            out.contains(&format!("bond_family({bond}, primary)")),
            "primary recalls {bond}: {out}"
        );
    }
    // The answer carries the LibreTexts citation as its proof, at consensus
    // trust (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // Hydrogen bonding is NOT classified into either family by this source --
    // honest abstention, never a fabricated family.
    assert!(out.contains("\"abstained\":true"), "hydrogen abstains: {out}");
}
