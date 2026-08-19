//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/heredity-term.adj`) driven through the built
//! CLI: a native `table` naming the core NGSS MS-LS3 heredity vocabulary --
//! gene, allele, dominant, recessive, genotype, phenotype, trait -- each with
//! its own defining sentence from NHGRI's Talking Glossary of Genomic and
//! Genetic Terms. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_hereditytterm_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/heredity-term.adj");
    std::fs::copy(&src, dir.join("heredity-term.adj")).expect("copy shipped heredity-term.adj");
}

#[test]
fn heredity_term_recall_binds_the_definition_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heredity-term.adj\"\n\
         ? heredity_term(gene, $D)\n\
         ? heredity_term(allele, $D)\n\
         ? heredity_term(dominant, $D)\n\
         ? heredity_term(recessive, $D)\n\
         ? heredity_term(genotype, $D)\n\
         ? heredity_term(phenotype, $D)\n\
         ? heredity_term(trait, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"basic_unit_of_inheritance\""),
        "gene means the basic unit of inheritance: {out}"
    );
    assert!(
        out.contains(
            "\"D\":\"one_of_two_or_more_versions_of_dna_sequence_at_a_given_genomic_location\""
        ),
        "allele means one of two or more versions of DNA sequence: {out}"
    );
    assert!(
        out.contains(
            "\"D\":\"one_allele_is_expressed_and_the_effect_of_the_other_allele_is_masked\""
        ),
        "dominant means one allele is expressed and the other is masked: {out}"
    );
    assert!(
        out.contains("\"D\":\"both_alleles_must_be_present_to_express_the_trait\""),
        "recessive means both alleles must be present to express the trait: {out}"
    );
    assert!(
        out.contains(
            "\"D\":\"a_scoring_of_the_type_of_variant_present_at_a_given_location_in_the_genome\""
        ),
        "genotype means a scoring of the type of variant present at a location: {out}"
    );
    assert!(
        out.contains("\"D\":\"an_individuals_observable_traits\""),
        "phenotype means an individual's observable traits: {out}"
    );
    assert!(
        out.contains("\"D\":\"a_specific_characteristic_of_an_individual\""),
        "trait means a specific characteristic of an individual: {out}"
    );
    assert!(
        out.contains("genome.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NHGRI/genome.gov citation at authoritative trust: {out}"
    );
}

#[test]
fn heredity_term_reverse_binds_the_term_for_that_definition() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heredity-term.adj\"\n\
         ? heredity_term($T, a_specific_characteristic_of_an_individual)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"trait\""),
        "the shipped 'specific characteristic of an individual' definition is the trait term: {out}"
    );
}

#[test]
fn heredity_term_abstains_honestly_on_a_term_outside_the_curated_core() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heredity-term.adj\"\n\
         ? heredity_term(chromosome, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "chromosome is a real NHGRI glossary term but outside this table's curated seven-term \
         core -- honest abstention, never invented: {out}"
    );
}
