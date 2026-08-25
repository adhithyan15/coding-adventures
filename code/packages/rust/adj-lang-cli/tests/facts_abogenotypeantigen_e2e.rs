//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/abo-genotype-antigen.adj`) driven through the
//! built CLI: a `rule` composing the already-shipped `abo_genotype_phenotype`
//! table (`biology/abo-genotype-phenotype.adj`) with the already-shipped
//! `blood_type_antigen` table (`biology/blood-groups.adj`, a SAME-DIRECTORY
//! import, the same shape `heat-causes-phase-change.adj`/
//! `force-causes-acceleration.adj`/`animal-habitat-definition.adj` already
//! established) to DERIVE `abo_genotype_antigen($Genotype, $Antigen)` -- the
//! SIXTH `rule`-based CAUSAL-COMPOSITION fact in this loop's science
//! curriculum sweep, and the FIRST to close the "heredity" gap with a
//! derivation rather than a direct recall. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_abogenotypeantigen_{tag}_{}", std::process::id()));
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

/// Copy all THREE shipped files, preserving their real relative directory
/// structure: `abo-genotype-antigen.adj` (in `biology/`) imports
/// `abo-genotype-phenotype.adj` and `blood-groups.adj` (both same dir).
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for rel in [
        "biology/abo-genotype-phenotype.adj",
        "biology/blood-groups.adj",
        "biology/abo-genotype-antigen.adj",
    ] {
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel), &dst).unwrap_or_else(|e| panic!("copy shipped {rel}: {e}"));
    }
}

#[test]
fn ia_ia_derives_a_antigen_with_dual_citations() {
    let dir = scratch("iaia");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/abo-genotype-antigen.adj\"\n\
         ? abo_genotype_antigen(ia_ia, $Antigen)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Antigen\":\"a_antigen\""),
        "an ia_ia genotype produces the A phenotype, whose red cells carry the A antigen: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: OpenStax
    // (abo_genotype_phenotype) AND NCBI Bookshelf (blood_type_antigen).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derived fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("openstax.org") && out.contains("ncbi.nlm.nih.gov"),
        "carries citations from BOTH composed libraries (abo-genotype-phenotype.adj and blood-groups.adj): {out}"
    );
}

#[test]
fn ia_ib_derives_both_antigens_and_reverse_binds_to_ia_ib() {
    let dir = scratch("iaib");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/abo-genotype-antigen.adj\"\n\
         ? abo_genotype_antigen(ia_ib, $Antigen)\n\
         ? abo_genotype_antigen($G, a_and_b_antigens)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Antigen\":\"a_and_b_antigens\""),
        "an ia_ib genotype produces the AB phenotype, whose red cells carry both antigens: {out}"
    );
    assert!(
        out.contains("\"G\":\"ia_ib\""),
        "the only tabled genotype whose red cells carry both antigens is ia_ib: {out}"
    );
}

#[test]
fn ia_i_abstains_honestly_as_abo_genotype_phenotype_itself_abstains() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/abo-genotype-antigen.adj\"\n\
         ? abo_genotype_antigen(ia_i, $Antigen)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "ia_i has no phenotype in abo_genotype_phenotype's own three-row table -- the abstention propagates through the join rather than inventing an antigen: {out}"
    );
}
