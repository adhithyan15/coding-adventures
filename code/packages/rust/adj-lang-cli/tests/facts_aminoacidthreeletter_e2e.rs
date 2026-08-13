//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/amino-acid-three-letter-code.adj`) driven
//! through the built CLI: a native `table` of amino acid -> THREE-letter
//! code, a sibling to the already-shipped `amino-acids.adj` (which only
//! carries the ONE-letter code), decoding the OTHER column of the SAME
//! already-cited DDBJ page. Resolves binding-query recall in BOTH
//! directions with the source's citation, and abstains on a non-standard
//! amino acid — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_aa3letter_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("biology/amino-acid-three-letter-code.adj");
    std::fs::copy(&src, dir.join("amino-acid-three-letter-code.adj"))
        .expect("copy shipped amino-acid-three-letter-code.adj");
}

#[test]
fn amino_acid_three_letter_code_recall_binds_both_directions_with_citation() {
    let dir = scratch("both");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"amino-acid-three-letter-code.adj\"\n\
         ? amino_acid_three_letter_code(glycine, $C)\n\
         ? amino_acid_three_letter_code($A, ala)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"C\":\"gly\""), "glycine -> gly: {out}");
    assert!(out.contains("\"A\":\"alanine\""), "ala -> alanine: {out}");
    assert!(
        out.contains("ddbj.nig.ac.jp") && out.contains("\"trust\":\"authoritative\""),
        "carries the DDBJ citation: {out}"
    );
}

#[test]
fn amino_acid_three_letter_code_recalls_the_two_acidic_residues() {
    let dir = scratch("acidic");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"amino-acid-three-letter-code.adj\"\n\
         ? amino_acid_three_letter_code(aspartic_acid, $C)\n\
         ? amino_acid_three_letter_code(glutamic_acid, $C)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"amino_acid_three_letter_code(aspartic_acid, asp)\""),
        "aspartic_acid -> asp: {out}"
    );
    assert!(
        out.contains("\"term\":\"amino_acid_three_letter_code(glutamic_acid, glu)\""),
        "glutamic_acid -> glu: {out}"
    );
}

#[test]
fn amino_acid_three_letter_code_abstains_honestly_on_a_nonstandard_amino_acid() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"amino-acid-three-letter-code.adj\"\n\
         ? amino_acid_three_letter_code(selenocysteine, $C)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "selenocysteine is not one of the standard twenty -- honest abstention: {out}"
    );
}
