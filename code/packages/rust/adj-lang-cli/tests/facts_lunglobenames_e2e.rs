//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/lung-lobe-names.adj`) driven through the built
//! CLI: a native `table` of individual named lung lobe -> owning lung
//! resolves a binding-query recall with the source's NIH/NCBI Bookshelf
//! citation, runs the relation backward with a genuine one-to-many reverse
//! recall (lung -> every lobe it has), and abstains on a non-lobe (the
//! trachea) -- 0 model calls. A sibling to the already-shipped
//! `lung-lobes.adj` (which recalls only lobe COUNT, not individual names) --
//! discovered via a header-revisit of that table's own already-cited
//! StatPearls corroboration sentence.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_lunglobenames_{tag}_{}", std::process::id()));
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
fn anatomy_lung_lobe_names_recall_binds_lung_with_citation() {
    let dir = scratch("lunglobenames");
    let src = facts_stdlib().join("anatomy/lung-lobe-names.adj");
    std::fs::copy(&src, dir.join("lung-lobe-names.adj")).expect("copy shipped lung-lobe-names.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-lobe-names.adj\"\n\
         ? lung_lobe_name(right_middle_lobe, $Lung)\n\
         ? lung_lobe_name(left_upper_lobe, $Lung)\n\
         ? lung_lobe_name($Lobe, right_lung)\n\
         ? lung_lobe_name(trachea, $Lung)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The right middle lobe belongs to the right lung; the left upper lobe
    // belongs to the left lung.
    assert!(
        out.contains("\"Lung\":\"right_lung\""),
        "right_middle_lobe -> right_lung: {out}"
    );
    assert!(
        out.contains("\"Lung\":\"left_lung\""),
        "left_upper_lobe -> left_lung: {out}"
    );
    // The relation runs BACKWARD as a genuine one-to-many recall: binding
    // right_lung recalls all THREE of its lobes.
    for lobe in ["right_upper_lobe", "right_middle_lobe", "right_lower_lobe"] {
        assert!(
            out.contains(&format!("lung_lobe_name({lobe}, right_lung)")),
            "right_lung recalls {lobe}: {out}"
        );
    }
    // The answer carries the NIH/NCBI Bookshelf citation as its proof, at the
    // authoritative trust tier for a primary U.S. government source.
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The trachea is not one of the five lobes -- honest abstention, never a
    // fabricated lung assignment.
    assert!(out.contains("\"abstained\":true"), "trachea abstains: {out}");
}
