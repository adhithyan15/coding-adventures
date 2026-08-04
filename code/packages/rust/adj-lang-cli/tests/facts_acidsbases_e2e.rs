//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/acids-bases.adj`) driven through the built CLI:
//! a native `table` of common chemical → acid-or-base classification resolves
//! binding-query recalls (forward and backward) with the source's LibreTexts
//! citation, and abstains on a substance not in the table (vinegar) — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsab_{tag}_{}", std::process::id()));
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
fn chemistry_acid_or_base_recall_binds_class_with_citation() {
    let dir = scratch("acidsbases");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/acids-bases.adj");
    std::fs::copy(&src, dir.join("acids-bases.adj")).expect("copy shipped acids-bases.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"acids-bases.adj\"\n\
         ? acid_or_base(hydrochloric_acid, $Class)\n\
         ? acid_or_base(sodium_hydroxide, $Class)\n\
         ? acid_or_base($S, base)\n\
         ? acid_or_base(vinegar, $Class)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Hydrochloric acid is an acid; sodium hydroxide (lye) is a base — the
    // recalled classification tokens (forward binds).
    assert!(
        out.contains("\"Class\":\"acid\""),
        "hydrochloric_acid -> acid: {out}"
    );
    assert!(
        out.contains("\"Class\":\"base\""),
        "sodium_hydroxide -> base: {out}"
    );
    // The relation runs BACKWARD: bind the class `base`, recall a substance —
    // lithium_hydroxide, the first base in the table.
    assert!(
        out.contains("\"S\":\"lithium_hydroxide\""),
        "base -> lithium_hydroxide (reverse recall into the bases): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at consensus trust
    // (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "vinegar" is not in the table — honest abstention, never a fabricated class.
    assert!(out.contains("\"abstained\":true"), "vinegar abstains: {out}");
}
