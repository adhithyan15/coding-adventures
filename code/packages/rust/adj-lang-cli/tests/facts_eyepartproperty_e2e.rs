//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/eye-part-property.adj`) driven through the
//! built CLI: a native `table` naming a descriptive PROPERTY of three eye
//! parts, decoded from parenthetical spans already sitting unused inside
//! `eye-parts.adj`'s own already-quoted NEI source sentences -- a sibling to
//! that table. Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a real, already-tabled eye part
//! (lens) whose own quote states no descriptive property, only a
//! function -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the iris sentence, the primary source of all three answers; it is now the
//! page's framing sentence, which every row overrides. The retina sentence
//! carries the page's U+00A0 no-break spaces.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_eyepartproperty_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/eye-part-property.adj");
    std::fs::copy(&src, dir.join("eye-part-property.adj"))
        .expect("copy shipped eye-part-property.adj");
}

const LOCATOR: &str = "https://www.nei.nih.gov/eye-health-information/healthy-vision/how-eyes-work";
const ENVELOPE: &str = "All the different parts of your eyes work together to help you see.";
const CORNEA: &str = "The cornea is shaped like a dome and bends light to help the eye focus.";
const IRIS: &str = "The iris (the colored part of the eye) controls how much light the pupil lets in.";
const RETINA: &str = "When light hits the\u{a0}retina\u{a0}(a light-sensitive layer of tissue at the back of the eye), special cells called photoreceptors turn the light into electrical signals.";

/// (part, property, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("cornea", "dome_shaped", CORNEA),
    ("iris", "colored_part_of_eye", IRIS),
    ("retina", "light_sensitive_layer", RETINA),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/eye-part-property.adj"))
        .expect("read shipped eye-part-property.adj");
    adj[adj.find("table eye_part_property").expect("table")..].to_string()
}

#[test]
fn eye_part_property_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property(iris, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"eye_part_property(iris, colored_part_of_eye)\""),
        "the iris is the colored part of the eye: {out}"
    );
    // THE WHOLE CITATION, as one contiguous run: sentence, the whole locator
    // (#15139: a host-only pin stays green through page moves), tier, and no
    // corroborations.
    assert!(out.contains(&citation(IRIS)), "carries the whole NEI citation for the iris: {out}");
}

#[test]
fn eye_part_property_recalls_backward() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property($Part, dome_shaped)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"eye_part_property(cornea, dome_shaped)\""),
        "backward recall should find cornea: {out}"
    );
}

#[test]
fn eye_part_property_abstains_honestly_on_lens() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property(lens, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "lens's own quote states no descriptive property -- honest abstention: {out}"
    );
}

#[test]
fn every_part_answer_carries_its_own_sentence() {
    for (part, property, sentence) in ROWS {
        let dir = scratch(&format!("row_{part}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"eye-part-property.adj\"\n? eye_part_property({part}, $P)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {part}: {out}");
        assert!(out.contains(&format!("\"P\":\"{property}\"")), "{part} → {property}: {out}");
        assert!(out.contains(&citation(sentence)), "{part}: its own sentence, whole: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != part {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {part}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {part}: {out}");
    }
}

#[test]
fn the_retina_row_carries_the_pages_no_break_spaces() {
    assert_eq!(RETINA.matches('\u{a0}').count(), 2, "two U+00A0 in the retina sentence");
    let body = shipped_table();
    assert!(!body.contains(&RETINA.replace('\u{a0}', " ")), "the ordinary-space form is not in the table");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (part, property, sentence) in ROWS {
        let expected = format!("    row ({part}, {property}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({part}, {property}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration anywhere in the table");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{IRIS}\"\n    locator")), "not the iris sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["cornea", "iris", "retina", "dome", "colored", "light-sensitive"] {
        assert!(!folded.contains(word), "the envelope must name no part or property, but contains {word:?}");
    }
}
