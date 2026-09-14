//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/respiratory-parts.adj`) driven through the built
//! CLI: a native `table` of each respiratory-system part → the function / role
//! its source states resolves binding-query recalls (forward AND backward) with
//! the source's NCI SEER Training Modules citation, and abstains on a word that
//! is not one of these respiratory parts (the stomach) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn anatomy_respiratory_parts_recall_binds_function_with_citation() {
    let dir = scratch("respiratoryparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/respiratory-parts.adj");
    std::fs::copy(&src, dir.join("respiratory-parts.adj"))
        .expect("copy shipped respiratory-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-parts.adj\"\n\
         ? part_function(trachea, $Function)\n\
         ? part_function(larynx, $Function)\n\
         ? part_function(diaphragm, $Function)\n\
         ? part_function($Part, gas_exchange)\n\
         ? part_function(stomach, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The trachea is the main airway, the larynx does speech, the diaphragm
    // contracts on inspiration — the recalled functions (forward binds).
    assert!(
        out.contains("\"Function\":\"main_airway\""),
        "trachea → main_airway: {out}"
    );
    assert!(
        out.contains("\"Function\":\"human_speech\""),
        "larynx → human_speech: {out}"
    );
    assert!(
        out.contains("\"Function\":\"contracts_inspiration\""),
        "diaphragm → contracts_inspiration: {out}"
    );
    // The relation runs BACKWARD: bind the function `gas_exchange`, recall the
    // part that performs it.
    assert!(
        out.contains("\"Part\":\"alveoli\""),
        "gas_exchange → alveoli (reverse recall): {out}"
    );
    // THIS ASSERTION WAS THE DEFECT IN TEST FORM. `training.seer.cancer.gov`
    // is satisfied by ANY page on the site, so it held equally well when all
    // eight rows carried the NOSE sentence — it could not tell the diaphragm's
    // answer citing the mechanics page from the diaphragm's answer citing a
    // sentence about nose hairs. It is the reason this conversion broke no
    // test until the per-row pins below were added.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries a SEER citation at the authoritative tier: {out}"
    );
    // Since #14986 these four answers span THREE different SEER pages —
    // trachea and larynx are both on larynx.html, which is why the loop
    // below iterates three and not four.
    for page in [
        "respiratory/passages/larynx.html",
        "respiratory/mechanics.html",
        "respiratory/passages/bronchi.html",
    ] {
        assert!(
            out.contains(page),
            "an answer here cites {page}, which states it: {out}"
        );
    }
    assert!(
        !out.contains("Nose hairs at the entrance"),
        "and none of these four answers is warranted by the nose sentence, \
         which was the envelope for all eight rows until #14986: {out}"
    );
    // The stomach is a digestive organ, not a respiratory part — honest
    // abstention, never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "stomach abstains: {out}");
}

const SEER: &str = "https://training.seer.cancer.gov/anatomy/respiratory/";

/// Assert one row's warrant, binding the PART so exactly one row answers.
///
/// Every part and every function in this table is unique to one row, so either
/// direction is single-answer here. Binding the part is still the safer habit:
/// where a key repeats, a whole-stdout `contains` is satisfied by an intact
/// sibling's copy, which is the masking defect found in `joint-types` (#15164).
fn assert_part(tag: &str, part: &str, function: &str, span: &str, page: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/respiratory-parts.adj"),
        dir.join("respiratory-parts.adj"),
    )
    .expect("copy shipped respiratory-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"respiratory-parts.adj\"\n? part_function({part}, $F)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {part}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"F\":\"{function}\"")),
        "{part} binds {function}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{SEER}{page}\",\"trust\":\"authoritative\""
        )),
        "{part} is warranted by the sentence that states its function, on the page \
         that carries it: {out}"
    );
}

/// #14986. The NOSE sentence was this table's `source` — the field that carries
/// the tier — for all eight rows, so a recall of the diaphragm came back proved
/// by "Nose hairs at the entrance to the nose trap large inhaled particles."
/// The header said as much in its own words: the envelope held "the single
/// cleanest span: the SEER statement that fixes the FIRST row".
///
/// All eight rows are pinned here. Their evidence was already in the file, in a
/// comment block no query could reach.
#[test]
fn every_row_carries_the_sentence_that_states_its_function() {
    assert_part(
        "rpnose", "nose", "traps_particles",
        "Nose hairs at the entrance to the nose trap large inhaled particles.",
        "passages/nose.html",
    );
    assert_part(
        "rppharynx", "pharynx", "passes_air",
        "The upper part of the pharynx (throat) lets only air pass through.",
        "passages/pharynx.html",
    );
    assert_part(
        "rplarynx", "larynx", "human_speech",
        "The larynx plays an essential role in human speech.",
        "passages/larynx.html",
    );
    assert_part(
        "rptrachea", "trachea", "main_airway",
        "The trachea, commonly called the windpipe, is the main airway to the lungs.",
        "passages/larynx.html",
    );
    assert_part(
        "rpbronchi", "bronchi", "branch_to_alveoli",
        "The bronchi branch into smaller and smaller passageways until they terminate in tiny air sacs called alveoli.",
        "passages/bronchi.html",
    );
    assert_part(
        "rplungs", "lungs", "external_respiration",
        "After this, there is an exchange of gases between the lungs and the blood. This is called external respiration.",
        "",
    );
    assert_part(
        "rpalveoli", "alveoli", "gas_exchange",
        "Exchange of gases between the air in the lungs and the blood in the capillaries occurs across the walls of the alveolar ducts and alveoli.",
        "passages/bronchi.html",
    );
    assert_part(
        "rpdiaphragm", "diaphragm", "contracts_inspiration",
        "During inspiration, the diaphragm contracts and the thoracic cavity increases in volume.",
        "mechanics.html",
    );
}

/// Two pages each state TWO rows, so two spans have a sibling that must not be
/// able to stand in for them. `assert_part` above queries one row at a time, so
/// each of those four spans is asserted in output where its sibling is absent —
/// the failure `skeleton-bones` (#15171) shipped by testing one row per shared
/// page.
#[test]
fn the_two_shared_pages_do_not_let_one_row_cover_the_other() {
    // (tag, part, function, a phrase from the OTHER row on the same page,
    //  that page). Review found this loop covering only the larynx pair while
    // its name, its doc comment and the changelog all said TWO shared pages.
    for (tag, part, function, other, page) in [
        ("sh1", "larynx", "human_speech", "windpipe", "larynx.html"),
        ("sh2", "trachea", "main_airway", "human speech", "larynx.html"),
        ("sh3", "bronchi", "branch_to_alveoli", "alveolar ducts", "bronchi.html"),
        ("sh4", "alveoli", "gas_exchange", "smaller and smaller passageways", "bronchi.html"),
    ] {
        let dir = scratch(tag);
        std::fs::copy(
            facts_stdlib().join("anatomy/respiratory-parts.adj"),
            dir.join("respiratory-parts.adj"),
        )
        .expect("copy shipped respiratory-parts.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"respiratory-parts.adj\"\n? part_function({part}, $F)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert!(
            out.contains(&format!("\"F\":\"{function}\"")),
            "{part} binds {function}: {out}"
        );
        // The OTHER row on the same page is absent from this answer, so the
        // span asserted for this row cannot have come from its sibling. The
        // page name is carried per case rather than hardcoded: the earlier form
        // said "larynx.html" unconditionally, so adding the bronchi pair would
        // have printed the wrong file on failure.
        assert!(
            !out.contains(other),
            "{part}'s answer carries no trace of the other row on {page}: {out}"
        );
    }
}

/// The envelope is the module's definition of respiration. It names no part of
/// the tract, so it warrants none of the eight rows.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("rpenvelope");
    std::fs::copy(
        facts_stdlib().join("anatomy/respiratory-parts.adj"),
        dir.join("respiratory-parts.adj"),
    )
    .expect("copy shipped respiratory-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-parts.adj\"\n? part_function($P, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        8,
        "all eight rows answer: {out}"
    );
    assert!(
        !out.contains("Respiration is the sequence of events"),
        "the framing span warrants no row: {out}"
    );
    // The nose sentence now warrants exactly ONE row where it used to be the
    // `source` on all eight. Twice: once under `citations`, once under `steps`.
    assert_eq!(
        out.matches("Nose hairs at the entrance").count(),
        2,
        "the nose sentence warrants the nose row and nothing else: {out}"
    );
    // AND PIN THE ENVELOPE ITSELF — source, locator AND tier. A pin stopping at
    // `\n    locator` asserts only that a locator follows (#15183).
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/respiratory-parts.adj"))
        .expect("read shipped respiratory-parts.adj");
    assert!(
        adj.contains(
            "    source \"Respiration is the sequence of events that results in the exchange of oxygen and carbon dioxide between the atmosphere and the body cells.\"\n    locator \"https://training.seer.cancer.gov/anatomy/respiratory/\"\n    trust authoritative"
        ),
        "the envelope carries the module's definition of respiration, verbatim"
    );
}

/// Parse every row-level `source` and `locator` out of a shipped `.adj`.
fn row_fields(rel: &str) -> (Vec<String>, Vec<String>) {
    let adj = std::fs::read_to_string(facts_stdlib().join(rel))
        .unwrap_or_else(|e| panic!("read shipped {rel}: {e}"));
    let mut spans: Vec<String> = Vec::new();
    let mut locators: Vec<String> = Vec::new();
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            spans.push(rest.trim_end_matches(0x22 as char).to_string());
        } else if let Some(rest) = line.strip_prefix(r#"        locator ""#) {
            locators.push(rest.trim_end_matches(0x22 as char).to_string());
        }
    }
    (spans, locators)
}

/// #15193. NO ROW HERE SHARES A SPAN — asserted against the SHIPPED FILE, the
/// same way the sharing tables assert which rows share one.
///
/// Eight rows, eight sentences, SIX pages: `larynx.html` states both the
/// larynx and the trachea, `bronchi.html` both the bronchi and the
/// alveoli.
#[test]
fn no_two_rows_share_a_span() {
    let (spans, locators) = row_fields("anatomy/respiratory-parts.adj");
    assert_eq!(spans.len(), 8, "every row carries its own source: {spans:?}");
    let mut uniq = spans.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        8,
        "and no two rows share one: {spans:?}"
    );
    // The rows DO carry locators, and there are fewer of them than rows —
    // two pages state two rows each.
    assert_eq!(locators.len(), 8, "every row carries a locator: {locators:?}");
    let mut uniq_locs = locators.clone();
    uniq_locs.sort();
    uniq_locs.dedup();
    assert_eq!(uniq_locs.len(), 6, "across 6 distinct pages: {locators:?}");
}
