//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/abo-genotype-phenotype.adj`) driven through
//! the built CLI: a native `table` naming, for each of three named ABO
//! genotype combinations (the two homozygotes and the one heterozygote a
//! self-cross between them produces), the blood-type PHENOTYPE it produces,
//! grounding OpenStax "Concepts of Biology" section 8.3 ("Extensions of the
//! Laws of Inheritance") and the "heredity" Major Gap
//! (ADJ-STDLIB-COVERAGE.md §5.1/§5.2). Runs the relation BACKWARD as a
//! genuine recall, and abstains honestly on `ia_i` -- a real ABO genotype
//! the cited passage never states a phenotype for in continuous prose.
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_abogenotypephenotype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/abo-genotype-phenotype.adj");
    std::fs::copy(&src, dir.join("abo-genotype-phenotype.adj"))
        .expect("copy shipped abo-genotype-phenotype.adj");
}

#[test]
fn abo_genotype_phenotype_recall_binds_blood_type_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"abo-genotype-phenotype.adj\"\n\
         ? abo_genotype_phenotype(ia_ia, $BloodType)\n\
         ? abo_genotype_phenotype(ib_ib, $BloodType)\n\
         ? abo_genotype_phenotype(ia_ib, $BloodType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("abo_genotype_phenotype(ia_ia, a)"),
        "the IAIA homozygote expresses blood type A: {out}"
    );
    assert!(
        out.contains("abo_genotype_phenotype(ib_ib, b)"),
        "the IBIB homozygote expresses blood type B: {out}"
    );
    assert!(
        out.contains("abo_genotype_phenotype(ia_ib, ab)"),
        "the IAIB heterozygote expresses blood type AB (codominance): {out}"
    );
    assert!(
        out.contains("openstax.org") && out.contains("\"trust\":\"consensus\""),
        "carries the OpenStax citation at consensus trust: {out}"
    );
}

#[test]
fn abo_genotype_phenotype_reverse_binds_genotype_from_blood_type() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"abo-genotype-phenotype.adj\"\n\
         ? abo_genotype_phenotype($G, ab)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The relation runs BACKWARD: binding `ab` recalls the `ia_ib`
    // heterozygote genotype that produces it.
    assert!(
        out.contains("abo_genotype_phenotype(ia_ib, ab)"),
        "blood type ab recalls the ia_ib heterozygote genotype: {out}"
    );
}

#[test]
fn abo_genotype_phenotype_abstains_honestly_on_ia_i() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"abo-genotype-phenotype.adj\"\n\
         ? abo_genotype_phenotype(ia_i, $BloodType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "ia_i is a real ABO genotype (a heterozygote with the null allele), but the \
         cited OpenStax passage never states its phenotype in continuous prose -- \
         honest abstention, never invented: {out}"
    );
}
