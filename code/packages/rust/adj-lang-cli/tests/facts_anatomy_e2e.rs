//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/body-counts.adj`) driven through the built CLI:
//! a native `table` of structure → count resolves a binding-query recall with
//! the source's citation, runs the relation backward (count → structure), and
//! abstains on a structure not in the table — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsanat_{tag}_{}", std::process::id()));
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
fn anatomy_body_counts_recall_binds_count_with_citation() {
    let dir = scratch("bodycounts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/body-counts.adj");
    std::fs::copy(&src, dir.join("body-counts.adj")).expect("copy shipped body-counts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"body-counts.adj\"\n\
         ? body_count(chromosomes, $N)\n\
         ? body_count(heart_chambers, $N)\n\
         ? body_count(pairs_of_ribs, $N)\n\
         ? body_count($S, 206)\n\
         ? body_count(spleens, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A human cell carries 46 chromosomes; the heart has 4 chambers; there are
    // twelve pairs of ribs — the recalled counts, each a plain number.
    assert!(out.contains("\"N\":\"46\""), "chromosomes → 46: {out}");
    assert!(out.contains("\"N\":\"4\""), "heart_chambers → 4: {out}");
    assert!(out.contains("\"N\":\"12\""), "pairs_of_ribs → 12: {out}");
    // The relation runs backward: the count 206 recalls the adult bone total.
    assert!(
        out.contains("\"S\":\"bones_in_adult_body\""),
        "206 → bones_in_adult_body (reverse recall): {out}"
    );
    // The answer carries the NHGRI genome.gov citation as its proof.
    assert!(
        out.contains("genome.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "spleens" is not a structure in the table — honest abstention, never a
    // fabricated count.
    assert!(out.contains("\"abstained\":true"), "unknown structure abstains: {out}");
}
