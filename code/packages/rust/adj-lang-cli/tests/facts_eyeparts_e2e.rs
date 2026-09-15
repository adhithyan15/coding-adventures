//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/eye-parts.adj`) driven through the built CLI:
//! a native `table` of eye part → function resolves binding-query recalls with
//! the source's NEI "How the Eyes Work" citation, runs the relation backward
//! (function → part, recalling the lens for focuses_light), and abstains on a
//! non-listed part (the eardrum, which belongs to the ear) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factseye_{tag}_{}", std::process::id()));
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
fn anatomy_eye_parts_recall_binds_function_with_citation() {
    let dir = scratch("eyeparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/eye-parts.adj");
    std::fs::copy(&src, dir.join("eye-parts.adj")).expect("copy shipped eye-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-parts.adj\"\n\
         ? eye_part_function(retina, $F)\n\
         ? eye_part_function(cornea, $F)\n\
         ? eye_part_function(optic_nerve, $F)\n\
         ? eye_part_function($P, focuses_light)\n\
         ? eye_part_function(eardrum, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The retina turns light into electrical signals; the cornea bends light;
    // the optic nerve carries the signals to the brain — the recalled functions,
    // each a token verbatim from the NEI path-of-sight description.
    assert!(
        out.contains("\"F\":\"turns_light_into_signals\""),
        "retina → turns_light_into_signals: {out}"
    );
    assert!(out.contains("\"F\":\"bends_light\""), "cornea → bends_light: {out}");
    assert!(
        out.contains("\"F\":\"carries_signals_to_brain\""),
        "optic_nerve → carries_signals_to_brain: {out}"
    );
    // The relation runs backward: the function focuses_light recalls the lens.
    assert!(out.contains("\"P\":\"lens\""), "focuses_light → lens (reverse recall): {out}");
    // The answer carries the NEI citation as its proof.
    // THE WHOLE LOCATOR, not just the host. #15139 found five stdlib
    // locators addressing pages that had moved; a host-only pin stays
    // green straight through that rot, because the host is the half a
    // site reorganization leaves alone.
    assert!(
        out.contains("\"locator\":\"https://www.nei.nih.gov/eye-health-information/healthy-vision/how-eyes-work\",\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The eardrum is not an eye part — honest abstention, never a fabricated
    // function.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown part abstains: {out}"
    );
}

const NEI: &str = "https://www.nei.nih.gov/eye-health-information/healthy-vision/how-eyes-work";

/// The retina sentence as the NEI page writes it: U+00A0 on both sides of
/// "retina". In the ordinary-space form it occurs on no page.
const RETINA: &str = "When light hits the\u{a0}retina\u{a0}(a light-sensitive layer of tissue at the back of the eye), special cells called photoreceptors turn the light into electrical signals.";
/// The optic-nerve sentence as the page writes it: U+00A0 after "the", "optic"
/// and "nerve".
const OPTIC_NERVE: &str = "These electrical signals travel from the retina through the\u{a0}optic\u{a0}nerve\u{a0}to the brain.";

/// Assert one row's warrant, binding the PART so exactly one row answers.
///
/// No row carries a `locator`, so every answer's locator is the envelope's —
/// which is asserted here as part of the needle, not assumed.
fn assert_part(tag: &str, part: &str, function: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/eye-parts.adj"),
        dir.join("eye-parts.adj"),
    )
    .expect("copy shipped eye-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"eye-parts.adj\"\n? eye_part_function({part}, $F)\n"),
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
            "\"source\":\"{span}\",\"locator\":\"{NEI}\",\"trust\":\"authoritative\""
        )),
        "{part} is warranted by the sentence that states its job: {out}"
    );
}

/// #14986. The CORNEA sentence was this table's `source` — the field that
/// carries the tier — for all six rows, so a recall of `optic_nerve` came back
/// proved by *"The cornea is shaped like a dome and bends light to help the eye
/// focus."*
///
/// Every sentence is on the SAME page, so the locator was never wrong here and
/// a hostname assertion could never have caught it — only the span was.
#[test]
fn every_part_carries_the_sentence_that_states_its_job() {
    assert_part(
        "epcornea", "cornea", "bends_light",
        "The cornea is shaped like a dome and bends light to help the eye focus.",
    );
    assert_part(
        "eppupil", "pupil", "lets_in_light",
        "Some of this light enters the eye through an opening called the pupil (PYOO-pul).",
    );
    assert_part(
        "epiris", "iris", "controls_light",
        "The iris (the colored part of the eye) controls how much light the pupil lets in.",
    );
    assert_part(
        "eplens", "lens", "focuses_light",
        "The lens works together with the cornea to focus light correctly on the retina.",
    );
    assert_part("epretina", "retina", "turns_light_into_signals", RETINA);
    assert_part("epoptic", "optic_nerve", "carries_signals_to_brain", OPTIC_NERVE);
}

/// The retina and optic-nerve sentences carry the page's U+00A0 no-break
/// spaces. Until this was measured they shipped with ordinary spaces, and in
/// that form both occurred ZERO times on the page they cite, while this test
/// file pinned the ordinary-space forms and stayed green.
#[test]
fn the_retina_and_optic_nerve_spans_carry_the_pages_no_break_spaces() {
    assert_eq!(RETINA.matches('\u{a0}').count(), 2, "two U+00A0 in the retina sentence");
    assert_eq!(OPTIC_NERVE.matches('\u{a0}').count(), 3, "three U+00A0 in the optic-nerve sentence");
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/eye-parts.adj"))
        .expect("read shipped eye-parts.adj");
    let table = &adj[adj.find("table eye_part_function").expect("table")..];
    for span in [RETINA, OPTIC_NERVE] {
        assert!(table.contains(&format!("        source \"{span}\"")), "the table carries the page form: {span:?}");
        assert!(
            !table.contains(&span.replace('\u{a0}', " ")),
            "the ordinary-space form, which the page never writes, is not in the table: {span:?}"
        );
    }
    let dir = scratch("epnbsp");
    std::fs::copy(facts_stdlib().join("anatomy/eye-parts.adj"), dir.join("eye-parts.adj"))
        .expect("copy shipped eye-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-parts.adj\"\n? eye_part_function(retina, $F)\n? eye_part_function(optic_nerve, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for span in [RETINA, OPTIC_NERVE] {
        assert!(!out.contains(&span.replace('\u{a0}', " ")), "the ordinary-space form reaches no answer: {out}");
    }
}

/// #15193. NO ROW HERE SHARES A SPAN, and no row carries a `locator` —
/// asserted against the SHIPPED FILE. The second half is what makes "one page"
/// a checked fact rather than a paragraph: a row-level locator appearing here
/// would mean the table had quietly become multi-page.
#[test]
fn no_two_rows_share_a_span() {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/eye-parts.adj"))
        .expect("read shipped eye-parts.adj");
    let mut spans: Vec<String> = Vec::new();
    let mut locators: Vec<String> = Vec::new();
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            spans.push(rest.trim_end_matches(0x22 as char).to_string());
        } else if let Some(rest) = line.strip_prefix(r#"        locator ""#) {
            locators.push(rest.trim_end_matches(0x22 as char).to_string());
        }
    }
    assert_eq!(spans.len(), 6, "every row carries its own source: {spans:?}");
    let mut uniq = spans.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), 6, "and no two rows share one: {spans:?}");
    // NO ROW OVERRIDES `locator` — all six inherit the envelope's, per the
    // rule `geography/reference-lines.adj` states. The pupil row DOES contain
    // the word `locator`, on its `cites` line, where the grammar makes it
    // mandatory (`Annotation::Cites { source, locator }`); that is a
    // corroboration's own address, not an override of the row's, which is why
    // this scan matches a `locator` LINE and not the word.
    assert!(
        locators.is_empty(),
        "no row overrides the envelope's locator: {locators:?}"
    );
    // And the pupil row's second span reaches its answer: its function takes
    // two sentences, and the iris one says what the pupil lets in is
    // controlled.
    assert!(
        adj.contains("        cites \"The iris (the colored part of the eye) controls how much light the pupil lets in.\""),
        "the pupil row carries the iris sentence as a corroboration"
    );
    assert!(
        adj.contains("    columns part, function"),
        "the shipped column names are unchanged"
    );
}

/// The envelope is the page's own opening sentence. It names NO part —
/// checked against all six row keys, not assumed — so it warrants none of the
/// six rows.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("epenvelope");
    std::fs::copy(
        facts_stdlib().join("anatomy/eye-parts.adj"),
        dir.join("eye-parts.adj"),
    )
    .expect("copy shipped eye-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-parts.adj\"\n? eye_part_function($P, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        6,
        "all six rows answer: {out}"
    );
    assert!(
        !out.contains("work together to help you see"),
        "the framing span warrants no row: {out}"
    );
    // The cornea sentence warrants exactly ONE row where it used to be the
    // `source` on all six — 12 occurrences across citations and steps, now 2.
    assert_eq!(
        out.matches("shaped like a dome").count(),
        2,
        "the cornea sentence warrants the cornea row and nothing else: {out}"
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/eye-parts.adj"))
        .expect("read shipped eye-parts.adj");
    assert!(
        adj.contains(
            "    source \"All the different parts of your eyes work together to help you see.\"\n    locator \"https://www.nei.nih.gov/eye-health-information/healthy-vision/how-eyes-work\"\n    trust authoritative"
        ),
        "the envelope carries the page's opening sentence, verbatim"
    );
}
