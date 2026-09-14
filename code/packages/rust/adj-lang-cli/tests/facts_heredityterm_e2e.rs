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

const GLOSSARY: &str = "https://www.genome.gov/genetics-glossary/";

/// Assert one row's warrant, binding the TERM so exactly one row answers.
///
/// Each of the seven terms names one row, so the term direction is
/// single-answer and no sibling's intact copy can satisfy a needle meant for
/// this row — the masking defect found in `joint-types` (#15164).
fn assert_term(tag: &str, term: &str, meaning: &str, span: &str, page: &str) {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"heredity-term.adj\"\n? heredity_term({term}, $D)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {term}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"D\":\"{meaning}\"")),
        "{term} binds {meaning}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{GLOSSARY}{page}\",\"trust\":\"authoritative\""
        )),
        "{term} is defined by its own glossary entry, on its own page: {out}"
    );
}

/// #14986. The GENE sentence was this table's `source` — the field that carries
/// the tier — for all seven rows, so a recall of `phenotype` came back proved
/// by *"The gene is considered the basic unit of inheritance."*
///
/// The other six spans were already in the file as untiered `cites`, each with
/// its own glossary URL, in row order. Nothing had to be found here; it had to
/// be attached.
#[test]
fn every_term_is_defined_by_its_own_glossary_entry() {
    assert_term(
        "htgene", "gene", "basic_unit_of_inheritance",
        "The gene is considered the basic unit of inheritance.",
        "Gene",
    );
    assert_term(
        "htallele", "allele",
        "one_of_two_or_more_versions_of_dna_sequence_at_a_given_genomic_location",
        "An allele is one of two or more versions of DNA sequence (a single base or a segment of bases) at a given genomic location.",
        "Allele",
    );
    assert_term(
        "htdom", "dominant",
        "one_allele_is_expressed_and_the_effect_of_the_other_allele_is_masked",
        "If the alleles of a gene are different, one allele will be expressed; it is the dominant gene. The effect of the other allele, called recessive, is masked.",
        "Dominant",
    );
    assert_term(
        "htrec", "recessive", "both_alleles_must_be_present_to_express_the_trait",
        "In the case of a recessive trait, the alleles of the trait-causing gene are the same, and both (recessive) alleles must be present to express the trait.",
        "Recessive-Traits-Alleles",
    );
    assert_term(
        "htgeno", "genotype",
        "a_scoring_of_the_type_of_variant_present_at_a_given_location_in_the_genome",
        "A genotype is a scoring of the type of variant present at a given location (i.e., a locus) in the genome.",
        "genotype",
    );
    assert_term(
        "htpheno", "phenotype", "an_individuals_observable_traits",
        "Phenotype refers to an individual’s observable traits, such as height, eye color and blood type.",
        "Phenotype",
    );
    assert_term(
        "httrait", "trait", "a_specific_characteristic_of_an_individual",
        "A trait, as related to genetics, is a specific characteristic of an individual.",
        "Trait",
    );
}

/// GLYPH FOR GLYPH. The phenotype sentence carries a CURLY apostrophe
/// (U+2019) in `individual’s`, not the ASCII `'`. That was the point of the
/// whole-chain pin this test replaces, and it is kept: a span "fixed" to ASCII
/// would no longer be the page's bytes.
///
/// The chain pin itself could not survive the conversion — it ran from the
/// bindings through five `corroborations` entries, and those spans are now the
/// `source` of the rows they define. Its own comment warned that "a
/// corroboration pin bound to the wrong entry is unique, anchored, and tests
/// nothing"; a chain pin that outlives its corroborations is the same hazard
/// one step later.
#[test]
fn the_phenotype_span_keeps_the_pages_curly_apostrophe() {
    let curly = "individual\u{2019}s observable traits";
    assert_term(
        "htglyph", "phenotype", "an_individuals_observable_traits",
        "Phenotype refers to an individual’s observable traits, such as height, eye color and blood type.",
        "Phenotype",
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/heredity-term.adj"))
        .expect("read shipped heredity-term.adj");
    assert!(adj.contains(curly), "the shipped span keeps U+2019");
    assert!(
        !adj.contains("individual's observable traits"),
        "and never the ASCII apostrophe the page does not use"
    );
}

/// The envelope is the glossary's definition of genetics. It MENTIONS genes —
/// said plainly rather than claimed otherwise — but it does not say what a gene
/// is to inheritance, and says nothing about the other six terms, so it
/// warrants none of the seven rows.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("htenvelope");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heredity-term.adj\"\n? heredity_term($T, $D)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven rows answer: {out}"
    );
    assert!(
        !out.contains("Genetics is the branch of biology"),
        "the framing span warrants no row: {out}"
    );
    // The gene sentence warrants exactly ONE row where it used to be the
    // `source` on all seven. Twice: once under `citations`, once under `steps`.
    assert_eq!(
        out.matches("The gene is considered the basic unit").count(),
        2,
        "the gene sentence warrants the gene row and nothing else: {out}"
    );
    // Seven rows, seven DISTINCT pages — one glossary entry each.
    for page in [
        "Gene", "Allele", "Dominant", "Recessive-Traits-Alleles", "genotype",
        "Phenotype", "Trait",
    ] {
        assert!(
            out.contains(&format!("{GLOSSARY}{page}\"")),
            "some answer cites the {page} entry: {out}"
        );
    }
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/heredity-term.adj"))
        .expect("read shipped heredity-term.adj");
    // Pin the envelope through its locator VALUE and tier, not just its
    // presence (#15183) — no row inherits the envelope's locator here.
    assert!(
        adj.contains(
            "    source \"Genetics is the branch of biology concerned with the study of inheritance, including the interplay of genes, DNA variation and their interactions with environmental factors.\"\n    locator \"https://www.genome.gov/genetics-glossary/Genetics\"\n    trust authoritative"
        ),
        "the envelope carries the glossary's definition of genetics, verbatim"
    );
    // The six `cites` were PROMOTED into the rows they define, not dropped:
    // every one is asserted above as some row's own `source`, at the
    // envelope's tier rather than untiered.
    assert!(
        !adj.contains("cites \""),
        "no corroboration survives at table level: each is now a row's own source"
    );
}
