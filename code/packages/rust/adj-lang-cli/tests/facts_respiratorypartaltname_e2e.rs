//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/respiratory-part-alt-name.adj`) driven through
//! the built CLI: a native `table` naming the everyday alternate name for
//! two named respiratory parts, decoded from clauses already sitting unused
//! inside `respiratory-parts.adj`'s own already-quoted NCI SEER source
//! sentences -- a sibling to that table. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a real,
//! already-tabled part (larynx) whose own quote states only its function,
//! never an everyday alternate name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_respiratorypartaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/respiratory-part-alt-name.adj");
    std::fs::copy(&src, dir.join("respiratory-part-alt-name.adj"))
        .expect("copy shipped respiratory-part-alt-name.adj");
}

#[test]
fn respiratory_part_alt_name_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name(trachea, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"respiratory_part_alt_name(trachea, windpipe)\""),
        "the trachea is commonly called the windpipe: {out}"
    );
    // STRENGTHENED. This arm used to read
    //     out.contains("training.seer.cancer.gov")
    //         && out.contains("\"trust\":\"authoritative\"")
    // which is satisfied by the host and the tier appearing ANYWHERE in the
    // output, in two unrelated places. It could not bind them to each other or
    // to a source -- and before the RS-5e conversion it was satisfied just as
    // well when the ALVEOLI answer carried the trachea sentence under
    // larynx.html. The parent table's own PR records this same pin letting a
    // whole conversion pass with zero test changes.
    assert!(
        out.contains(TRACHEA_CITATION),
        "the trachea answer carries the trachea sentence, bound to larynx.html \
         and the authoritative tier in one citation object: {out}"
    );
}

/// The whole serialised citation object for one answer: that row's own source,
/// its own locator, the inherited tier, and the closing `corroborations` array.
///
/// TWO PAGES FOR TWO ROWS. Each row restates its `locator` because neither page
/// is the envelope's (ADJ-TABLES §4). Measured from the page bytes: the trachea
/// sentence occurs x1 on larynx.html and x0 on bronchi.html; the alveoli
/// sentence occurs x1 on bronchi.html and x0 on larynx.html; both score x0
/// against a real 404 control on the same host.
const TRACHEA_CITATION: &str = r#""source":"The trachea, commonly called the windpipe, is the main airway to the lungs.","locator":"https://training.seer.cancer.gov/anatomy/respiratory/passages/larynx.html","trust":"authoritative","corroborations":[]"#;

const ALVEOLI_CITATION: &str = r#""source":"The bronchi branch into smaller and smaller passageways until they terminate in tiny air sacs called alveoli.","locator":"https://training.seer.cancer.gov/anatomy/respiratory/passages/bronchi.html","trust":"authoritative","corroborations":[]"#;

/// The trachea sentence ALONE, without its locator or tier.
///
/// Used as a NEGATIVE arm on the alveoli answer, and it is the arm that actually
/// discriminates this conversion: before RS-5e the envelope WAS this sentence,
/// so it warranted the alveoli answer too. Measured on the two captures --
/// the alveoli answer bound to this sentence occurred x1 before and x0 after.
const TRACHEA_SENTENCE: &str =
    "The trachea, commonly called the windpipe, is the main airway to the lungs.";

/// The page's framing sentence. Names neither row key nor either alt-name, so
/// it warrants no row and must reach no answer.
const ENVELOPE: &str = "Respiration is the sequence of events that results in the exchange of oxygen and carbon dioxide between the atmosphere and the body cells.";

/// Run the three-query companion shape against the shipped table.
///
/// EVERY CALLER MUST PASS A DISTINCT `tag`: `scratch` derives its directory from
/// the tag and the process id, and every test in this binary shares one process,
/// so two callers passing the same tag would derive the same path and delete
/// each other's files.
fn recall(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"respiratory-part-alt-name.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

/// THE DEFECT THIS CONVERSION REMOVED, as one needle.
///
/// Before RS-5e the envelope WAS the trachea sentence and the single shipped
/// locator was larynx.html, so this answer was warranted by a sentence about a
/// different part AND addressed to a page its own sentence is not on. Measured
/// on the pre-conversion capture: `bronchi.html` occurred x0 across the whole
/// output while `larynx.html` occurred x4.
#[test]
fn alveoli_answer_carries_the_bronchi_sentence_and_its_own_page() {
    let out = recall("alv_row", "respiratory_part_alt_name($Part, air_sacs)");
    assert!(
        out.contains(ALVEOLI_CITATION),
        "the alveoli answer carries the BRONCHI sentence under bronchi.html: {out}"
    );
}

/// The negative half of the same defect, IN ITS OWN TEST -- and the separation
/// is the point, not a style preference.
///
/// It lived beside the positive arm above for one revision. Run against the
/// pre-conversion file, the positive arm panicked FIRST and this one never
/// executed: `assert!` stops at the first failure. I then reported that the
/// test "fails on both arms", which was a prediction I had not verified. An arm
/// whose failure is masked by an earlier arm has never been observed to fire,
/// and unexecuted is neither passing nor failing -- it is unproven.
///
/// Measured on the two captures: the alveoli answer bound to the TRACHEA
/// sentence occurred x1 before the conversion and x0 after.
#[test]
fn the_trachea_sentence_does_not_warrant_the_alveoli_answer() {
    let out = recall("alv_neg", "respiratory_part_alt_name($Part, air_sacs)");
    assert!(
        !out.contains(TRACHEA_SENTENCE),
        "the trachea sentence must not warrant the alveoli answer: {out}"
    );
}

/// The trachea answer must not drift onto the other row's page either.
#[test]
fn trachea_answer_carries_its_own_page() {
    let out = recall("tra_row", "respiratory_part_alt_name(trachea, $AltName)");
    assert!(
        out.contains(TRACHEA_CITATION),
        "the trachea answer carries the trachea sentence under larynx.html: {out}"
    );
}

/// Every row overrides both `source` and `locator`, so the envelope's wording is
/// primary for NO answer. Without this arm the measured "envelope reaches zero
/// answers" is unpinned, and the envelope could drift back to a row-warranting
/// sentence with every other assertion still green.
///
/// THIS IS A FORWARD GUARD, NOT A REGRESSION TEST FOR THIS CONVERSION, and the
/// distinction was established by watching it rather than by reasoning about it.
/// Run against the pre-conversion file it PASSES -- I predicted it would fail.
/// The prediction confused the envelope's ROLE with the `ENVELOPE` string: the
/// old envelope was the trachea sentence, and the respiration framing sentence
/// did not occur anywhere in that file, so `!out.contains(ENVELOPE)` was
/// trivially true. The arm can only fire if a LATER edit lets this envelope
/// reach an answer. That is worth having; it is not evidence about this change.
///
/// IT ALSO QUERIES BOTH ROWS, AND THE FIRST VERSION DID NOT -- a mutation
/// harness caught that, not a reading of the code.
///
/// Under RS-5e every row overrides `source` and `locator`, so the envelope can
/// never actually REACH an answer. That means this assertion can only fire on a
/// TEXT COLLISION between the envelope and whichever row is queried. The first
/// version queried only the trachea row, so:
///
///   envelope := the trachea sentence  -> rejected, but only because the text
///                                        collided with the TRACHEA ROW'S OWN
///                                        source, not because a screen noticed
///                                        a row-warranting envelope;
///   envelope := the alveoli sentence  -> ACCEPTED. A row-warranting envelope
///                                        drifted in with every assertion green,
///                                        which is precisely what the paragraph
///                                        above claimed could not happen.
///
/// Querying both rows closes it: any envelope equal to a row's own span now
/// collides with that row's answer, and that is #14986's defect class by
/// definition. The negative control -- real page prose naming no row -- is
/// accepted under both versions, so the screen is not merely rejecting
/// everything.
#[test]
fn the_envelope_is_primary_for_no_answer() {
    for (tag, query) in [
        ("env_tra", "respiratory_part_alt_name(trachea, $AltName)"),
        ("env_alv", "respiratory_part_alt_name($Part, air_sacs)"),
    ] {
        let out = recall(tag, query);
        assert!(
            !out.contains(ENVELOPE),
            "the envelope's wording is primary for no answer, and it reached \
             the answer to `{query}`: {out}"
        );
    }
}

#[test]
fn respiratory_part_alt_name_recalls_backward_to_alveoli() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name($Part, air_sacs)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"respiratory_part_alt_name(alveoli, air_sacs)\""),
        "air_sacs recalls the alveoli: {out}"
    );
}

#[test]
fn respiratory_part_alt_name_abstains_honestly_on_larynx() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name(larynx, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "larynx is a real, already-tabled respiratory part but its own quote states only its function, never an everyday alternate name -- honest abstention: {out}"
    );
}
