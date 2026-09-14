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
    // THIS ASSERTION WAS THE DEFECT IN TEST FORM. `genome.gov` was the
    // envelope locator on all nine rows, so it held for any answer the table
    // produced — it could not tell the rib answer citing StatPearls from the
    // rib answer citing a sentence about chromosomes. Since #14986 these four
    // answers span four different pages.
    // BOUND, not two loose needles over a five-query output. The first draft
    // of this repair asserted `contains("genome.gov")` and
    // `contains("\"trust\":\"authoritative\"")` separately and captioned the pair
    // "the chromosome answer" — but four of the five rows here are
    // `authoritative`, so the tier needle was satisfied by the rib, heart or
    // bone answer. That is the same unbound-needle shape this comment block
    // exists to record.
    assert!(
        out.contains(
            "\"source\":\"Humans have 22 pairs of numbered chromosomes (autosomes) and one pair of sex chromosomes (XX or XY), for a total of 46.\",\"locator\":\"https://www.genome.gov/genetics-glossary/Chromosome\",\"trust\":\"authoritative\""
        ),
        "the chromosome answer cites NHGRI at the authoritative tier: {out}"
    );
    for (span, page) in [
        (
            "It has four hollow chambers surrounded by muscle and other heart tissue.",
            "https://www.nhlbi.nih.gov/health/heart/anatomy",
        ),
        (
            "Generally, there are twelve pairs of ribs.",
            "https://www.ncbi.nlm.nih.gov/books/NBK538328/",
        ),
        (
            "Human infants typically have 270 bones, fusing into around 206 in the human adult.",
            "https://www.ncbi.nlm.nih.gov/books/NBK537199/",
        ),
    ] {
        assert!(
            out.contains(&format!("\"source\":\"{span}\",\"locator\":\"{page}\"")),
            "the answer warranted by {span} cites {page}: {out}"
        );
    }
    // COUNTED, not forbidden under one URL prefix. The first form banned the
    // chromosome sentence only beside an `ncbi` locator, so re-attaching it to
    // the NHLBI or SEER page walked straight through.
    assert_eq!(
        out.matches("for a total of 46.").count(),
        2,
        "the chromosome sentence warrants one row here and no other: {out}"
    );
    // "spleens" is not a structure in the table — honest abstention, never a
    // fabricated count.
    assert!(out.contains("\"abstained\":true"), "unknown structure abstains: {out}");
}

#[test]
fn anatomy_body_counts_hand_bone_group_extension() {
    // EXTENDED this cycle with the three hand-bone GROUP counts, reusing the
    // same already-cited NCBI-Bookshelf sentence `hand-bones.adj` already
    // ships in its own header (no new WebFetch) — a sibling fact decoded
    // from a different clause of that same quote (HOW MANY bones per
    // group, not WHERE the group sits).
    let dir = scratch("handbonecounts");
    let src = facts_stdlib().join("anatomy/body-counts.adj");
    std::fs::copy(&src, dir.join("body-counts.adj")).expect("copy shipped body-counts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"body-counts.adj\"\n\
         ? body_count(carpals, $N)\n\
         ? body_count(metacarpals, $N)\n\
         ? body_count(phalanges, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // 8 carpal bones, 5 metacarpal bones, 14 phalanges.
    assert!(out.contains("\"N\":\"8\""), "carpals → 8: {out}");
    assert!(out.contains("\"N\":\"5\""), "metacarpals → 5: {out}");
    assert!(out.contains("\"N\":\"14\""), "phalanges → 14: {out}");
}

/// Assert one row's warrant AND ITS TIER, binding the structure so exactly one
/// row answers. Every structure in this table names one row.
fn assert_count(tag: &str, structure: &str, n: &str, span: &str, locator: &str, tier: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/body-counts.adj"),
        dir.join("body-counts.adj"),
    )
    .expect("copy shipped body-counts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"body-counts.adj\"\n? body_count({structure}, $N)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {structure}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"N\":\"{n}\"")),
        "{structure} binds {n}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{locator}\",\"trust\":\"{tier}\""
        )),
        "{structure} is warranted by the sentence that states its count, on the page \
         that carries it, at the tier that page earns: {out}"
    );
    // The pairing: the count this row claims has to appear in the span that is
    // supposed to state it. `span` is tied to the shipped row by the needle
    // above and `n` by the binding, so this is a property of the FILE, not of
    // the test's own constants.
    //
    // ITS LIMIT, stated rather than left to be discovered: the hand sentence
    // carries 27, 8, 5 AND 14, so for the three hand-bone rows this check is
    // satisfied by any of those four numbers. What actually separates them is
    // `out.contains("\"N\":\"{n}\"")` under the one-citation gate above.
    //
    // Six of these spans write the number as a numeral; three write it as a
    // word, so both forms are accepted and the words are listed rather than
    // guessed at.
    let word = match n {
        "2" => "pair",     // "the pair of" / "The paired kidneys"
        "4" => "four",     // "It has four hollow chambers"
        "12" => "twelve",  // "there are twelve pairs of ribs"
        _ => n,            // 46, 206, 8, 5, 14 are numerals on their pages
    };
    assert!(
        span.contains(n) || span.contains(word),
        "{structure}'s span states its count: {span}"
    );
}

const NHGRI: &str = "https://www.genome.gov/genetics-glossary/Chromosome";
const BONES: &str = "https://www.ncbi.nlm.nih.gov/books/NBK537199/";
const HEART: &str = "https://www.nhlbi.nih.gov/health/heart/anatomy";
const LUNGS: &str = "https://www.nhlbi.nih.gov/health/lungs";
const RIBS: &str = "https://www.ncbi.nlm.nih.gov/books/NBK538328/";
const KIDNEY: &str = "https://training.seer.cancer.gov/anatomy/urinary/components/kidney.html";
const HAND: &str = "https://www.ncbi.nlm.nih.gov/books/NBK279362/";
const HAND_SPAN: &str = "The human hand is made up of a total of 27 individual bones: 8 carpal bones (in the base of the hand), 5 metacarpal bones (in the middle part of the hand) and 14 phalanges (finger bones) are connected by joints and ligaments.";

/// #14986. The CHROMOSOME sentence was this table's `source` — the field that
/// carries the tier — for all nine rows, so a recall of `pairs_of_ribs` came
/// back proved by a sentence that counts chromosomes.
#[test]
fn every_count_is_stated_by_the_sentence_that_warrants_it() {
    assert_count(
        "bcchrom", "chromosomes", "46",
        "Humans have 22 pairs of numbered chromosomes (autosomes) and one pair of sex chromosomes (XX or XY), for a total of 46.",
        NHGRI, "authoritative",
    );
    assert_count(
        "bcbones", "bones_in_adult_body", "206",
        "Human infants typically have 270 bones, fusing into around 206 in the human adult.",
        BONES, "authoritative",
    );
    assert_count(
        "bcheart", "heart_chambers", "4",
        "It has four hollow chambers surrounded by muscle and other heart tissue.",
        HEART, "authoritative",
    );
    assert_count(
        "bcribs", "pairs_of_ribs", "12",
        "Generally, there are twelve pairs of ribs.",
        RIBS, "authoritative",
    );
    assert_count(
        "bckidney", "kidneys", "2",
        "The paired kidneys are located between the twelfth thoracic and third lumbar vertebrae, one on each side of the vertebral column.",
        KIDNEY, "authoritative",
    );
}

/// THE TIER DEFECT, and the sharpest finding in this table. One envelope
/// imposes one `trust` on every row, and this file's header already said that
/// tier is wrong for three of them:
///
/// > Unlike the other six rows, this source is a patient-education / teaching
/// > summary (IQWiG), not a primary government anatomy authority — so these
/// > three rows are honestly `consensus`-tier, one rung below the table's
/// > `authoritative` envelope trust
///
/// The file said `consensus` in a comment and shipped `authoritative` in the
/// machine value, and the test that queried these three rows asserted only
/// their NUMBERS, so nothing caught it. RS-5e rows override `trust` per row —
/// established by running it, not by reading the lowering code.
#[test]
fn the_three_hand_bone_rows_carry_the_consensus_tier_their_source_earns() {
    for (tag, group, n) in [
        ("bccarp", "carpals", "8"),
        ("bcmeta", "metacarpals", "5"),
        ("bcphal", "phalanges", "14"),
    ] {
        assert_count(tag, group, n, HAND_SPAN, HAND, "consensus");
    }
    // And NOT the envelope's tier. Asserted per row, in output where no other
    // row's citation is present, so an `authoritative` elsewhere cannot mask it.
    for (tag, group) in [("bccarpN", "carpals"), ("bcmetaN", "metacarpals"), ("bcphalN", "phalanges")] {
        let dir = scratch(tag);
        std::fs::copy(
            facts_stdlib().join("anatomy/body-counts.adj"),
            dir.join("body-counts.adj"),
        )
        .expect("copy shipped body-counts.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"body-counts.adj\"\n? body_count({group}, $N)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert!(
            !out.contains("\"trust\":\"authoritative\""),
            "{group} never returns the envelope's tier: {out}"
        );
    }
}

/// The lungs span opened MID-SENTENCE — "the pair of spongy, pinkish-gray
/// organs in your chest" — so the quoted run never said whose, or that it was
/// about lungs at all. The page's full sentence is contiguous and is quoted
/// whole. Same class as #15185.
#[test]
fn the_lungs_span_is_no_longer_a_mid_sentence_fragment() {
    assert_count(
        "bclungs", "lungs", "2",
        "Your lungs are the pair of spongy, pinkish-gray organs in your chest.",
        LUNGS, "authoritative",
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/body-counts.adj"))
        .expect("read shipped body-counts.adj");
    assert!(
        !adj.contains("source \"the pair of spongy"),
        "no row opens on the bare fragment"
    );
}

/// The envelope is the SEER module's opening description of the body as many
/// smaller structures. It states no count of anything this table counts, so it
/// warrants none of the nine rows.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("bcenvelope");
    std::fs::copy(
        facts_stdlib().join("anatomy/body-counts.adj"),
        dir.join("body-counts.adj"),
    )
    .expect("copy shipped body-counts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"body-counts.adj\"\n? body_count($S, $N)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        9,
        "all nine rows answer: {out}"
    );
    assert!(
        !out.contains("The human body is a single structure"),
        "the framing span warrants no row: {out}"
    );
    // The chromosome sentence warrants exactly ONE row where it used to be the
    // `source` on all nine. Twice: citations and steps.
    assert_eq!(
        out.matches("for a total of 46.").count(),
        2,
        "the chromosome sentence warrants the chromosome row and nothing else: {out}"
    );
    // The hand sentence warrants exactly THREE rows — the only shared span here.
    assert_eq!(
        out.matches(HAND_SPAN).count(),
        6,
        "the hand sentence warrants its three rows and no others: {out}"
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/body-counts.adj"))
        .expect("read shipped body-counts.adj");
    assert!(
        adj.contains(
            "    source \"The human body is a single structure but it is made up of billions of smaller structures of four major kinds:\"\n    locator \"https://training.seer.cancer.gov/anatomy/body/\"\n    trust authoritative"
        ),
        "the envelope carries the module's framing sentence, verbatim"
    );
    // The `columns` line. Column names are positional and never reach the
    // output, so renaming them is invisible to every assertion above.
    assert!(
        adj.contains("    columns structure, count"),
        "the shipped column names are unchanged"
    );
}

/// An envelope `source` is REQUIRED, so the framing span above is not
/// removable — someone cannot "fix" the framing-span pattern by deleting it.
///
/// This was asserted in prose in three places (the `.adj` header, the
/// CHANGELOG, and a comment here) and exercised nowhere, which review caught.
/// A claim established by running it once, in a session, is not a claim the
/// repository holds. Now it runs.
#[test]
fn a_table_envelope_without_a_source_is_rejected() {
    let dir = scratch("bcnosource");
    std::fs::write(dir.join("case.adj"), NO_ENVELOPE_SOURCE).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(!ok, "a sourceless envelope must not lower: {out}");
    assert!(
        out.contains("TableMissingProvenance"),
        "and it is rejected for the missing envelope source specifically: {out}"
    );
    // POSITIVE CONTROL: the SAME table with an envelope source lowers and
    // answers. Without it, a program that failed for any other reason — a typo
    // in the table, a bad query — would read as a pass.
    let dir = scratch("bcwithsource");
    std::fs::write(dir.join("case.adj"), WITH_ENVELOPE_SOURCE).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "the same table with an envelope source lowers: {out}");
    assert!(out.contains("\"N\":\"1\""), "and answers: {out}");
}

const NO_ENVELOPE_SOURCE: &str = r#"table probe_count {
    columns structure, count

    row (alpha, 1) {
        source "Alpha sentence."
        locator "https://example.gov/a"
    }

    locator "https://example.gov/frame"
    trust authoritative
}

? probe_count(alpha, $N)
"#;

const WITH_ENVELOPE_SOURCE: &str = r#"table probe_count {
    columns structure, count

    row (alpha, 1) {
        source "Alpha sentence."
        locator "https://example.gov/a"
    }

    source "Framing sentence."
    locator "https://example.gov/frame"
    trust authoritative
}

? probe_count(alpha, $N)
"#;
