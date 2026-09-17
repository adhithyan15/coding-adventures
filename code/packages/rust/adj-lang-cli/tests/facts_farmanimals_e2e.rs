//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animals.adj`) driven through the built
//! CLI: a native `table` of farm animal → product resolves a binding-query
//! recall with the provenance of the row it selected, and abstains on a
//! non-farm-animal — 0 model calls.
//!
//! THIS SUITE WAS REWRITTEN FOR THE RS-5e PER-ROW CONVERSION (#14986), AND THE
//! REWRITE IS THE POINT. Before the conversion this file held ONE test whose
//! only citation assertion was
//!
//!     out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\"")
//!
//! — the inverted shape of #14735. It is satisfied by the host and the tier
//! appearing ANYWHERE in the output, in two unrelated places, binding neither
//! to a source. Measured: that test passed against the pre-conversion file and
//! passed again, unchanged, against the converted one, while the output it
//! inspects changed by the envelope going from **8 occurrences to 0** and each
//! of four row spans going from **0 to 2**. A whole conversion was invisible to
//! it. Each answer is now pinned to its **whole serialised citation object**,
//! closing on `"corroborations":[]`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

/// EVERY CALLER MUST PASS A DISTINCT `tag`: the directory is derived from the
/// tag and the process id, and every test in this binary shares one process, so
/// two callers passing the same tag would derive the same path and delete each
/// other's files. An intermittent test is worse than a failing one.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimals_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("agriculture/farm-animals.adj");
    std::fs::copy(&src, dir.join("farm-animals.adj")).expect("copy shipped farm-animals.adj");
}

/// Run one binding query against the shipped table.
fn recall(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"farm-animals.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

/// The shipped file, read as text, for the structural arm.
fn shipped_table() -> String {
    std::fs::read_to_string(facts_stdlib().join("agriculture/farm-animals.adj"))
        .expect("read shipped farm-animals.adj")
}

// The whole serialised citation object for each row: that row's own source, the
// inherited locator and tier, and the closing empty corroborations array.
//
// EXTRACTED FROM CLI OUTPUT, NOT COMPOSED. Every one of these five was copied
// out of a capture of this binary's own stdout. Composing a pin from memory is
// how four separate anchors broke during this session's earlier work.
//
// ONE PAGE FOR FIVE ROWS, so no row restates `locator` or `trust` — ADJ-TABLES
// §4: a row restates `locator` only when its page DIFFERS from the envelope's,
// and the spec records that a row locator equal to the envelope's is a no-op
// producing byte-identical output. That makes the inherited form structurally
// visible instead of something a reader establishes by comparing five copies of
// one URL, and it is what `the_table_shape_matches_the_measured_rows` asserts.
const CHICKEN_CITATION: &str = "\"source\":\"A small flock can keep a family in eggs or meat.\",\"locator\":\"https://www.cfsph.iastate.edu/thelivestockproject/animals-for-beginning-small-farmers/\",\"trust\":\"authoritative\",\"corroborations\":[]";

const DUCK_CITATION: &str = "\"source\":\"Like chickens, Ducks produce meat and eggs.\",\"locator\":\"https://www.cfsph.iastate.edu/thelivestockproject/animals-for-beginning-small-farmers/\",\"trust\":\"authoritative\",\"corroborations\":[]";

const RABBIT_CITATION: &str = "\"source\":\"Some rabbit breeds produce a very soft wool that can be harvested regularly and used for fiber arts.\",\"locator\":\"https://www.cfsph.iastate.edu/thelivestockproject/animals-for-beginning-small-farmers/\",\"trust\":\"authoritative\",\"corroborations\":[]";

const GOAT_CITATION: &str = "\"source\":\"Goat milk may be pasteurized for human consumption.\",\"locator\":\"https://www.cfsph.iastate.edu/thelivestockproject/animals-for-beginning-small-farmers/\",\"trust\":\"authoritative\",\"corroborations\":[]";

/// The sheep sentence ALONE, without its locator or tier.
///
/// This is the arm that actually discriminates the conversion. Before RS-5e the
/// envelope WAS this sentence, so it warranted every answer in the table.
/// Measured through this CLI on the two trees, across the four non-sheep
/// queries: **x8 before, x0 after**.
const SHEEP_SENTENCE: &str =
    "Sheep are low maintenance and versatile – depending on the breed, they can produce wool, meat, and milk.";

/// The page's framing sentence, now the envelope. Names no animal and no
/// product, so it warrants no row and must reach no answer.
const ENVELOPE: &str = "Deciding to bring animals onto your small farm can be exciting. It can also be overwhelming. Here are some things to think about while deciding what animals you will raise.";

#[test]
fn chicken_answer_carries_its_own_span() {
    let out = recall("chicken", "farm_animal_product(chicken, $Product)");
    assert!(out.contains("\"Product\":\"eggs\""), "chicken → eggs: {out}");
    assert!(
        out.contains(CHICKEN_CITATION),
        "the chicken answer carries the chicken sentence in one whole citation \
         object: {out}"
    );
}

#[test]
fn duck_answer_carries_its_own_span() {
    let out = recall("duck", "farm_animal_product(duck, $Product)");
    assert!(
        out.contains(DUCK_CITATION),
        "the duck answer carries the duck sentence: {out}"
    );
}

#[test]
fn rabbit_answer_carries_its_own_span() {
    let out = recall("rabbit", "farm_animal_product(rabbit, $Product)");
    assert!(
        out.contains(RABBIT_CITATION),
        "the rabbit answer carries the rabbit sentence: {out}"
    );
}

#[test]
fn goat_answer_carries_its_own_span() {
    let out = recall("goat", "farm_animal_product(goat, $Product)");
    assert!(
        out.contains(GOAT_CITATION),
        "the goat answer carries the goat sentence: {out}"
    );
}

/// The negative half of the defect, IN ITS OWN TEST — and the separation is the
/// point, not a style preference.
///
/// Shard 03640 records what happens otherwise: a negative arm living inside a
/// positive arm's test never executed, because `assert!` panics on the first
/// failure, and the run was then reported as "failing on both arms" without
/// checking which line panicked. Unexecuted is neither passing nor failing.
///
/// Queries all four non-sheep rows, because a screen that checks one row cannot
/// see a wrong span attached to another — 03640's envelope screen was accepted
/// by a row-warranting envelope for exactly that reason.
///
/// THIS ARM IS A FORWARD GUARD, NOT A DISCRIMINATING REGRESSION ARM, and the
/// distinction was established by watching it rather than by reasoning about
/// it. I predicted it would fail against the pre-conversion file. **It passed**,
/// and the reason is a punctuation accident: pre-conversion the envelope was
/// this sentence WITHOUT its terminal period, while `SHEEP_SENTENCE` carries
/// the period the page actually has, so `!out.contains(...)` was trivially true
/// on the old tree. Measured in the file text: the with-period form occurs
/// **0** times in the pre-conversion file and **2** in the converted one — the
/// header comment that lists each row's span, and the sheep row's own `source`.
///
/// Dropping the period would not rescue it either — the period-less stem is a
/// substring of both forms, so it matches on both trees and discriminates
/// nothing. What actually discriminates this conversion is WHICH ANSWER carries
/// the sentence, which is what the four per-row citation arms above assert, and
/// all four were observed FAILING against the pre-conversion bytes. This arm
/// guards against a future edit letting the sheep sentence reach a non-sheep
/// answer; it is not evidence about this change.
#[test]
fn the_sheep_sentence_warrants_only_the_sheep_row() {
    for (tag, query) in [
        ("neg_chicken", "farm_animal_product(chicken, $Product)"),
        ("neg_duck", "farm_animal_product(duck, $Product)"),
        ("neg_rabbit", "farm_animal_product(rabbit, $Product)"),
        ("neg_goat", "farm_animal_product(goat, $Product)"),
    ] {
        let out = recall(tag, query);
        assert!(
            !out.contains(SHEEP_SENTENCE),
            "the sheep sentence must not warrant `{query}`: {out}"
        );
    }
}

/// The sheep row keeps that sentence — it is the one row it legitimately
/// defends. This arm exists so the negative arm above cannot be satisfied by
/// deleting the sentence from the table altogether.
#[test]
fn the_sheep_row_still_carries_the_sheep_sentence() {
    let out = recall("sheep", "farm_animal_product(sheep, $Product)");
    assert!(out.contains("\"Product\":\"wool\""), "sheep → wool: {out}");
    assert!(
        out.contains(SHEEP_SENTENCE),
        "the sheep row carries the sheep sentence, WITH the terminal period the \
         page has and the pre-conversion envelope dropped: {out}"
    );
}

/// A FORWARD GUARD, NOT A REGRESSION TEST FOR THIS CONVERSION, and it is
/// declared as such rather than counted among the discriminating arms.
///
/// Under RS-5e every row overrides `source`, so the envelope can never actually
/// REACH an answer; this can only fire on a text collision between the envelope
/// and whichever row is queried. It is expected to PASS against the
/// pre-conversion file too, because the new envelope string does not occur
/// there at all — shard 03640 records predicting the opposite and being wrong
/// for precisely this reason, having confused the envelope's ROLE with the
/// ENVELOPE STRING.
#[test]
fn the_envelope_is_primary_for_no_answer() {
    for (tag, query) in [
        ("env_chicken", "farm_animal_product(chicken, $Product)"),
        ("env_sheep", "farm_animal_product(sheep, $Product)"),
        ("env_goat", "farm_animal_product(goat, $Product)"),
    ] {
        let out = recall(tag, query);
        assert!(
            !out.contains(ENVELOPE),
            "the envelope's wording is primary for no answer, and it reached \
             the answer to `{query}`: {out}"
        );
    }
}

/// `milk` names exactly one animal, while `eggs` and `wool` each name two.
#[test]
fn the_reverse_bind_on_milk_names_the_goat() {
    let out = recall("rev_milk", "farm_animal_product($Animal, milk)");
    assert!(
        out.contains("\"term\":\"farm_animal_product(goat, milk)\""),
        "milk reverse-binds to the goat: {out}"
    );
}

#[test]
fn farm_animals_abstains_honestly_on_tiger() {
    let out = recall("abstain", "farm_animal_product(tiger, $Product)");
    assert!(
        out.contains("\"abstained\":true"),
        "a tiger is not a farm animal — honest abstention, never a fabricated \
         product: {out}"
    );
}

/// The structural arm that sibling test files ship and this one did not.
///
/// Counted AFTER this change and therefore including it: **47** files under
/// `adj-lang-cli/tests` define `fn the_table_shape_matches_the_measured_rows`,
/// out of 361 named `facts_*_e2e.rs`. Before this change the same predicate
/// gave 46. Stating which side of the change a count came from matters: a
/// draft of this comment cited 46 as the population this file was joining,
/// which reads as independent support while actually being the number this
/// file moved.
///
/// The predicate is the function name, not what the arm asserts — sibling
/// shape arms differ in content. Under narrower predicates the population is
/// much smaller: 16 files assert a locator-line count, 13 assert no `cites` at
/// any indent, 12 assert a trust-line count.
///
/// It asserts the table's measured shape rather than any answer: five row
/// blocks each carrying its own `source`, no `cites` at any indent, and exactly
/// ONE `locator` and ONE `trust` — the inherited form. That last count is what
/// makes "this table is single-page" a checked fact rather than a paragraph,
/// which is the enforcement #15197 proposes for tables that inherit.
#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        5,
        "five row-level sources, one per row: {body}"
    );
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "this table ships no corroboration at any indent: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("locator "))
            .count(),
        1,
        "exactly one locator, the envelope's — every row inherits it: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("trust "))
            .count(),
        1,
        "exactly one trust, the envelope's: {body}"
    );
    assert!(
        body.contains(&format!("source \"{ENVELOPE}\"")),
        "the envelope is the page's framing sentence: {body}"
    );
}
