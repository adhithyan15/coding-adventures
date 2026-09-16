//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/plate-boundaries.adj`) driven through the
//! built CLI: a native `table` of the three tectonic plate-boundary types → how
//! the plates move at each resolves binding-query recalls (forward AND backward)
//! with the source's National Park Service citation, and abstains on a word that
//! is not one of the three boundary types (the equator) — 0 model calls.

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
fn earth_science_plate_boundaries_recall_binds_motion_with_citation() {
    let dir = scratch("plateboundaries");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/plate-boundaries.adj");
    std::fs::copy(&src, dir.join("plate-boundaries.adj"))
        .expect("copy shipped plate-boundaries.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plate-boundaries.adj\"\n\
         ? boundary_motion(divergent, $Motion)\n\
         ? boundary_motion(convergent, $Motion)\n\
         ? boundary_motion(transform, $Motion)\n\
         ? boundary_motion($Boundary, slide_past)\n\
         ? boundary_motion(equator, $Motion)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Plates rip apart at a divergent plate boundary, causing volcanic activity and shallow earthquakes;\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Plates rip apart at a divergent boundary, one plate subducts at a
    // convergent boundary, and plates slide past at a transform boundary — the
    // recalled motions (forward binds).
    assert!(
        out.contains("\"Motion\":\"rip_apart\""),
        "divergent → rip_apart: {out}"
    );
    assert!(
        out.contains("\"Motion\":\"subducts\""),
        "convergent → subducts: {out}"
    );
    assert!(
        out.contains("\"Motion\":\"slide_past\""),
        "transform → slide_past: {out}"
    );
    // The relation runs BACKWARD: bind the motion `slide_past`, recall its
    // boundary type.
    assert!(
        out.contains("\"Boundary\":\"transform\""),
        "slide_past → transform (reverse recall): {out}"
    );
    // EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The three arms that can
    // SEE that change each live in their OWN `#[test]` below, one arm apiece.
    //
    // They were written here together first, and that was a mistake worth
    // recording: `assert!` panics on the FIRST failure, so running all three
    // against the pre-conversion file demonstrated ONE arm failing and left the
    // other two UNEXECUTED. Unexecuted is not passing and it is not failing --
    // it is unproven, and an arm whose failure is masked by an earlier arm has
    // never been observed to fire.
    // The answer carries the National Park Service citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    //
    // STRENGTHENED. This arm used to read
    //     out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\"")
    // which is satisfied by the host and the tier appearing ANYWHERE in the
    // output, in any two unrelated places -- it cannot bind them to each other
    // or to a source. The whole object below binds source, locator and trust
    // together and closes on the corroborations array.
    assert!(
        out.contains(
            "\"source\":\"Plates rip apart at a divergent plate boundary, causing volcanic activity and shallow earthquakes;\",\"locator\":\"https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "source, locator and trust are bound together in one citation: {out}"
    );
    // The equator is a line of latitude, not one of the three plate-boundary
    // types — honest abstention, never a fabricated motion.
    assert!(out.contains("\"abstained\":true"), "equator abstains: {out}");
    // EVERY ROW OVERRIDES THE ENVELOPE, so the envelope's wording is primary for
    // NO answer. Without this arm the measured "envelope wording in zero
    // answers" is unpinned, and the envelope could drift back to a
    // row-warranting sentence with every other assertion still green.
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
}

/// Run the five-query companion against the shipped table and return stdout.
///
/// EVERY CALLER MUST PASS A DISTINCT `tag`. `scratch` derives its directory from
/// the tag and the PROCESS id, and every test in this binary shares one process:
/// two callers passing the same tag would derive the SAME path and delete each
/// other's files. That race was introduced once already in this batch, and an
/// intermittent test is worse than a failing one.
fn recall_output(tag: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("earth-science/plate-boundaries.adj"),
        dir.join("plate-boundaries.adj"),
    )
    .expect("copy shipped plate-boundaries.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plate-boundaries.adj\"\n\
         ? boundary_motion(divergent, $Motion)\n\
         ? boundary_motion(convergent, $Motion)\n\
         ? boundary_motion(transform, $Motion)\n\
         ? boundary_motion($Boundary, slide_past)\n\
         ? boundary_motion(equator, $Motion)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

// Each constant is the WHOLE serialised citation object for one answer:
// bindings, that row's own source, locator, trust, and the closing
// `corroborations` array. A needle that pinned only the motion atom, or only
// the host, could not tell an answer warranted by its OWN sentence from one
// warranted by a sentence about a different boundary type -- which is exactly
// what this table shipped before the conversion.
//
// The convergent span carries the page's CURLY quotes and its terminating
// SEMICOLON. The ASCII-quote, full-stop form occurs ZERO times on the page:
// that form is a fragment punctuated into a sentence, and pinning it here would
// re-enshrine the defect `plate_boundary_citation_keeps_the_pages_semicolon`
// was written to catch.
const CONVERGENT_CITATION: &str = r#""bindings":{"Motion":"subducts"},"citations":[{"source":"At a convergent plate boundary, one plate dives (“subducts”) beneath the other, resulting in a variety of earthquakes and a line of volcanoes on the overriding plate;","locator":"https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm","trust":"authoritative","corroborations":[]"#;

const TRANSFORM_CITATION: &str = r#""bindings":{"Motion":"slide_past"},"citations":[{"source":"Transform plate boundaries are where plates slide laterally past one another, producing shallow earthquakes but little or no volcanic activity.","locator":"https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm","trust":"authoritative","corroborations":[]"#;

const REVERSE_TRANSFORM_CITATION: &str = r#""bindings":{"Boundary":"transform"},"citations":[{"source":"Transform plate boundaries are where plates slide laterally past one another, producing shallow earthquakes but little or no volcanic activity.","locator":"https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm","trust":"authoritative","corroborations":[]"#;

#[test]
fn convergent_answer_carries_the_convergent_sentence() {
    let out = recall_output("conv_row");
    assert!(
        out.contains(CONVERGENT_CITATION),
        "the convergent answer carries the CONVERGENT sentence, curly quotes and \
         semicolon as the page writes it: {out}"
    );
}

#[test]
fn transform_answer_carries_the_transform_sentence() {
    let out = recall_output("trans_row");
    assert!(
        out.contains(TRANSFORM_CITATION),
        "the transform answer carries the TRANSFORM sentence: {out}"
    );
}

/// The reverse bind resolves to the same row as the forward one, so it must
/// carry that row's own sentence too. This is why the transform span occurs
/// FOUR times in a run while the other two occur twice: transform is the only
/// row bound twice, and each answer renders its citation on two surfaces.
#[test]
fn reverse_bind_carries_the_transform_sentence() {
    let out = recall_output("rev_row");
    assert!(
        out.contains(REVERSE_TRANSFORM_CITATION),
        "the REVERSE bind carries the TRANSFORM sentence: {out}"
    );
}

const LOCATOR: &str =
    "https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm";

/// The page's framing sentence. Frames plate MOTION while naming no boundary
/// type and no motion verb, so it warrants no row by itself.
const ENVELOPE: &str = "The landscapes of our national parks, as well as geologic hazards such as earthquakes and volcanic eruptions, are due to the movement of the large plates of Earth\u{2019}s outer shell.";

const DIVERGENT: &str = "Plates rip apart at a divergent plate boundary, causing volcanic activity and shallow earthquakes;";
const CONVERGENT: &str = "At a convergent plate boundary, one plate dives (\u{201c}subducts\u{201d}) beneath the other, resulting in a variety of earthquakes and a line of volcanoes on the overriding plate;";
const TRANSFORM: &str = "Transform plate boundaries are where plates slide laterally past one another, producing shallow earthquakes but little or no volcanic activity.";

/// (boundary type, its motion atom, the NPS sentence stating that motion)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("divergent", "rip_apart", DIVERGENT),
        ("convergent", "subducts", CONVERGENT),
        ("transform", "slide_past", TRANSFORM),
    ]
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/plate-boundaries.adj"))
        .expect("read shipped plate-boundaries.adj");
    adj[adj.find("table boundary_motion").expect("table")..].to_string()
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    // THIS ARM IS THE ONE THAT WOULD HAVE FAILED ON THE PRE-CONVERSION FILE,
    // which carried no row-level `source` at all -- the rows were bare
    // `row (k, v)` lines and the divergent span sat in the envelope. Measured:
    // against those bytes this assertion is the panic site.
    assert_eq!(
        body.matches("\n        source \"").count(),
        3,
        "three row sources, one per boundary type: {body}"
    );
    // SCOPE: a row-level `cites` is legal ADJ, so this pins a convention local
    // to THIS table, not a language rule. It is keyword-anchored, so a `cites`
    // at any indent fails it.
    //
    // MEASURED 2026-09-16, NOT INHERITED: 19 shipped fact files carry a
    // row-level `cites` (86 such lines), over 362 files matching
    // adj-facts-stdlib/**/*.adj with CHANGELOG.d and *.query.adj excluded. Two
    // predicates agree on the same file set -- a `cites` at 8-space indent, and
    // a `cites` inside a real `row (...) {` block -- because in this corpus
    // every row-level one is written at 8 spaces, while a TABLE-level `cites`
    // sits at 4 spaces before the table's closing brace (122 such lines, all
    // legal; ADJ-A9 requires each to carry its own `locator`, and forbids
    // nothing about where it sits).
    //
    // The sibling test files say "18 shipped tables use one". That figure is
    // stale and is propagated by copying rather than re-derivation -- it is in
    // facts_bloodcells_e2e.rs, facts_musclenucleicount_e2e.rs and shards 03590
    // and 03600. The predicate is stated here with the number so the next copy
    // can be re-measured instead of inherited a fourth time.
    //
    // It would NOT have failed on the pre-conversion file: that file shipped no
    // `cites` either, so this arm passed there and passes here. It guards
    // against a future one being added, not against the defect this PR fixes.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "this table ships no corroboration at any indent: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("locator "))
            .count(),
        1,
        "exactly one locator line, the envelope's: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("trust "))
            .count(),
        1,
        "exactly one trust line, the envelope's: {body}"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's framing sentence, at the authoritative tier: {body}"
    );
    assert!(
        !body.contains(&format!("\n    source \"{DIVERGENT}\"\n    locator")),
        "not the divergent span as the envelope again: {body}"
    );
    // THE ENVELOPE MUST NAME NO BOUNDARY TYPE **AND NO MOTION VERB**, and the
    // verbs are the half that does the work here.
    //
    // The sharpest mutants this page offers name no row key and still restate a
    // row fact outright: "Volcanic eruptions and shallow earthquakes are common
    // where plates rip apart." and "Shallow earthquakes and little volcanism
    // occur where one plate slides laterally past another." Either would warrant
    // a ROW if shipped as the envelope, and a key-only rule admits both.
    //
    // A LOCAL RULE, NOT A GENERAL ONE. #15344 (`blood-cell-types`) settled the
    // general principle the other way for an ENUMERATING frame: there the
    // envelope MAY name the keys, and its shape test REQUIRES them. This table's
    // envelope is DEFINITIONAL -- it frames what produces landscapes and hazards,
    // naming no type -- so forbidding the taxonomy is the right pin HERE and
    // would be wrong for an enumerating frame such as "There are three types of
    // tectonic plate boundaries:".
    //
    // SCOPE OF THE CLAIM: this arm screens the two RESTATING mutants and only
    // those. Of the five prepared mutants it rejects `restates_divergent_fact`
    // (on "rip apart") and `restates_transform_fact` (on "laterally"). It does
    // NOT catch `hotspot_wrong_subject`, `fingernails_off_subject` or
    // `enumerating_frame`, which are wrong on subject or on a trailing colon
    // that frames the list rather than the subject. A purely negative arm is
    // never enough, and this one does not pretend otherwise.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["divergent", "convergent", "transform", "subducts", "dives", "laterally"] {
        assert!(
            !tokens.contains(&word),
            "the envelope must name no boundary type and no motion verb, but \
             contains {word:?} as a whole word: {shipped_envelope:?}"
        );
    }
    // "rip apart" is two words, so it is checked on the phrase rather than on
    // the token list -- the token scan above cannot see it.
    assert!(
        !shipped_envelope.contains("rip apart"),
        "the envelope must not restate the divergent motion: {shipped_envelope:?}"
    );
}

#[test]
fn every_row_motion_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check (#15318).
    let body = shipped_table();
    for (boundary, motion, span) in scale() {
        // ANCHORED ON THE ROW HEADER, not the bare `source` line. The weaker
        // needle proves a span sits among the eight-space source lines
        // SOMEWHERE, not that it belongs to THIS row; on `mixture-types` a
        // mutant swapping two rows' spans satisfied the weaker needle fully.
        assert!(
            body.contains(&format!(
                "row ({boundary}, {motion}) {{\n        source \"{span}\"\n"
            )),
            "{boundary} carries its own span IN ITS OWN ROW: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        assert!(
            tokens.contains(&boundary),
            "{boundary}: its own span must name the boundary type as a whole \
             word, but it is not a token of {normalized:?}"
        );
        // And it must state the motion the atom compresses. EXHAUSTIVE, with no
        // catch-all: a `_` arm would silently apply one row's needle to any row
        // added later. Each atom is a COMPRESSION of the page's phrase -- the
        // page writes "rip apart", "subducts", "slide laterally past" -- so the
        // needle is the page's wording, never the atom's own spelling.
        let content: &[&str] = match boundary {
            "divergent" => &["rip apart"],
            "convergent" => &["subducts"],
            "transform" => &["slide laterally past"],
            other => panic!("no content needle registered for row {other}"),
        };
        for needle in content {
            assert!(
                normalized.contains(*needle),
                "{boundary}: its span must state {needle:?}, the motion the atom \
                 {motion:?} compresses, but it is not in {normalized:?}"
            );
        }
    }
}

const PLATE_BOUNDARIES_PIN: &str = r#""bindings":{"Motion":"rip_apart"},"citations":[{"source":"Plates rip apart at a divergent plate boundary, causing volcanic activity and shallow earthquakes;","locator":"https://www.nps.gov/subjects/geology/plate-tectonics-types-of-plate-boundaries.htm","trust":"authoritative""#;

#[test]
fn plate_boundary_citation_keeps_the_pages_semicolon() {
    let dir = scratch("reground");
    std::fs::copy(
        facts_stdlib().join("earth-science/plate-boundaries.adj"),
        dir.join("plate-boundaries.adj"),
    )
    .expect("copy shipped plate-boundaries.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plate-boundaries.adj\"
? boundary_motion(divergent, $Motion)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped value was a FRAGMENT PUNCTUATED INTO A SENTENCE: the page's
    // wording, with one character changed so it would read as standalone. It
    // therefore appeared on no page. Every quote-keyed screen passed it,
    // because the quotes were all correct.
    assert!(
        out.contains(PLATE_BOUNDARIES_PIN),
        "the divergent citation ends as the page does: {out}"
    );
}
