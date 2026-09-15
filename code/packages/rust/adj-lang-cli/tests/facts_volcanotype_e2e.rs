//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/volcano-type.adj`) driven through the built
//! CLI: a native `table` naming three types of volcano and what each
//! actually is, quoted from USGS's "About Volcanoes" page. 0
//! answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986), in the page's
//! characters. The envelope used to be the shield sentence with plain spaces,
//! a form that occurs on no page: the page writes U+00A0 on both sides of
//! "lava" (and after "Cinder" in the cinder-cone sentence). The envelope is
//! now a framing sentence from the page, which every row overrides.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_volcano_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/volcano-type.adj");
    std::fs::copy(&src, dir.join("volcano-type.adj")).expect("copy shipped volcano-type.adj");
}

const LOCATOR: &str = "https://www.usgs.gov/programs/VHP/about-volcanoes";
const ENVELOPE: &str = "There are about 1,350 potentially active volcanoes worldwide, not counting the volcanoes under the oceans.";
const CINDER: &str = "Cinder\u{a0}cones are the simplest type of volcano.";
const SHIELD: &str = "Shield volcanoes are built almost entirely of fluid\u{a0}lava\u{a0}flows.";
const COMPOSITE: &str = "Some of the Earth's grandest mountains are composite volcanoes\u{2014}sometimes called stratovolcanoes.";

/// (type, description, that row's own sentence in the page's characters)
const ROWS: [(&str, &str, &str); 3] = [
    ("cinder_cone", "is_the_simplest_type_of_volcano", CINDER),
    ("shield_volcano", "built_almost_entirely_of_fluid_lava_flows", SHIELD),
    ("composite_volcano", "also_called_a_stratovolcano", COMPOSITE),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_adj() -> String {
    std::fs::read_to_string(facts_stdlib().join("geology/volcano-type.adj")).expect("read shipped volcano-type.adj")
}

fn shipped_table() -> String {
    let adj = shipped_adj();
    adj[adj.find("table volcano_type").expect("table")..].to_string()
}

#[test]
fn volcano_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type(shield_volcano, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATIONS ARRAY, not a fragment (#13916, #13918). This used to
    // pin the shield sentence with PLAIN spaces, the envelope as shipped, a
    // form that occurs on no page (#14986). The row now carries the page's
    // U+00A0 on both sides of "lava".
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(SHIELD)), "the shield sentence, in the page's characters, is the only citation: {out}");
    assert!(
        !out.contains("fluid lava flows."),
        "the plain-space form, which occurs on no page, is not cited: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"built_almost_entirely_of_fluid_lava_flows\""),
        "shield_volcano means built_almost_entirely_of_fluid_lava_flows: {out}"
    );
}

#[test]
fn volcano_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type($T, is_the_simplest_type_of_volcano)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"cinder_cone\""),
        "the shipped is_the_simplest_type_of_volcano example is cinder_cone: {out}"
    );
}

#[test]
fn volcano_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type(lava_dome, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "lava_dome is a real term the source names but explicitly disclaims as technically not a volcano type, not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_type_answer_carries_its_own_sentence() {
    for (kind, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"volcano-type.adj\"\n? volcano_type($T, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"T\":\"{kind}\"")), "{desc} binds {kind}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{kind}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_rows_carry_the_pages_no_break_spaces() {
    // As shipped, the envelope wrote the shield sentence with plain spaces and
    // occurred on no page. The page writes U+00A0 after "Cinder" and on both
    // sides of "lava"; the rows carry those characters, and no plain-space form
    // of either sentence is left in the table.
    assert_eq!(CINDER.matches('\u{a0}').count(), 1, "the cinder sentence has one U+00A0");
    assert_eq!(SHIELD.matches('\u{a0}').count(), 2, "the shield sentence has two U+00A0");
    let body = shipped_table();
    for kind_sentence in [CINDER, SHIELD] {
        assert!(body.contains(&format!("        source \"{kind_sentence}\"\n")), "shipped in the page's characters");
    }
    assert!(!body.contains("Cinder cones are the simplest"), "no plain-space cinder sentence in the table");
    assert!(!body.contains("fluid lava flows."), "no plain-space shield sentence in the table");
    // The header's lava-dome quote now uses the page's double quotes.
    let adj = shipped_adj();
    assert!(!adj.contains("technically not a 'volcano type'"), "no single-quoted lava-dome quote in the header");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, desc, sentence) in ROWS {
        let expected = format!("    row ({kind}, {desc}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    let folded = ENVELOPE.to_lowercase();
    for word in ["cinder", "shield", "composite", "strato", "lava", "cone", "simplest", "grand", "mountain", "dome"] {
        assert!(!folded.contains(word), "the envelope must name no type or description, but contains {word:?}");
    }
}
