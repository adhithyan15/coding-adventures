//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/insulin-glucagon-trigger.adj`) driven through
//! the built CLI: a native `table` recording the blood-glucose condition
//! that triggers insulin vs. glucagon release -- a sibling to the
//! already-shipped `hormone-glands.adj` (which only carries which gland
//! secretes each hormone), decoding the trigger-condition clause already
//! sitting unused inside two of that table's own per-row `source` spans.
//! Resolves forward and backward recall queries with the source's
//! citation -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN CITATION (RS-5e, #14986). Until that change the
//! table envelope was the INSULIN sentence and the glucagon sentence rode as
//! a table-level `cites`, so a `glucagon` recall came back warranted
//! primarily by a sentence about insulin.
//!
//! WHY THE PINS LOOK LIKE THIS. The old citation assertion here was
//! `out.contains("seer.cancer.gov") && out.contains("\"trust\":\"authoritative\"")`
//! -- satisfied by ANY SEER citation, constraining no sentence text, and
//! before this change satisfied by the insulin span riding on the glucagon
//! answer. It could not fail for the bug it was meant to cover. Each answer
//! is now pinned to its own WHOLE serialised citation object, closing on
//! `"corroborations":[]}` so a relocated span cannot satisfy it (#14735).
//!
//! WHY EVERY NEGATIVE ARM NAMES A WHOLE SPAN. The two sentences differ in
//! only three tokens each (`Beta`/`insulin`/`high` against
//! `Alpha`/`glucagons`/`low`) and share nine content words. A short needle
//! like "pancreatic islets" matches both, so a negative arm built from one
//! would pass no matter which sentence was returned.
//!
//! THE PAGE SPELLS IT "glucagons". That plural is the cited page's own, and
//! ships verbatim (the #15320 balance-typo precedent): repairing it would
//! cite a sentence the page does not contain.

use std::path::{Path, PathBuf};
use std::process::Command;

const LOCATOR: &str =
    "https://training.seer.cancer.gov/anatomy/endocrine/glands/pancreas.html";

const INSULIN_SPAN: &str = "Beta cells in the pancreatic islets secrete the hormone insulin in response to a high concentration of glucose in the blood.";

const GLUCAGON_SPAN: &str = "Alpha cells in the pancreatic islets secrete the hormone glucagons in response to a low concentration of glucose in the blood.";

/// The table-level framing sentence. It must reach NO answer: the rows carry
/// their own spans, and the envelope only supplies `locator` and `trust`.
const ENVELOPE: &str = "The pancreas is a long, soft organ that lies transversely along the posterior abdominal wall, posterior to the stomach, and extends from the region of the duodenum to the spleen.";

/// The page's whole hormone/cell taxonomy -- not merely this table's two row
/// keys. Naming neither row key is NOT enough for a framing sentence: the
/// page's very next sentence frames the EXOCRINE pancreas, names no hormone,
/// and would still frame the wrong half of the organ. The same mutant
/// survived in shard 03590 until its forbidden list was widened from the row
/// keys to the page's taxonomy.
const FORBIDDEN_IN_ENVELOPE: &[&str] = &[
    "insulin",
    "glucagon",
    "alpha",
    "beta",
    "islet",
    "exocrine",
    "endocrine",
    "hormone",
];

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn lib_text() -> String {
    std::fs::read_to_string(facts_stdlib().join("biology/insulin-glucagon-trigger.adj"))
        .expect("read shipped insulin-glucagon-trigger.adj")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_insulinglucagontrigger_{tag}_{}",
        std::process::id()
    ));
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
    let src = facts_stdlib().join("biology/insulin-glucagon-trigger.adj");
    std::fs::copy(&src, dir.join("insulin-glucagon-trigger.adj"))
        .expect("copy shipped insulin-glucagon-trigger.adj");
}

fn ask(tag: &str, goal: &str) -> String {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"insulin-glucagon-trigger.adj\"\n? {goal}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

/// The whole serialised citation object for one span, closing on the empty
/// corroborations array. A span that merely APPEARS somewhere in the output
/// does not satisfy this; it has to be THIS answer's own citation.
fn citation(span: &str) -> String {
    format!(
        "{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}"
    )
}

#[test]
fn each_hormone_recalls_its_level_warranted_by_its_own_sentence() {
    // (hormone, level, its own span, the OTHER row's span)
    let cases = [
        ("insulin", "high", INSULIN_SPAN, GLUCAGON_SPAN),
        ("glucagon", "low", GLUCAGON_SPAN, INSULIN_SPAN),
    ];

    for (hormone, level, own, other) in cases {
        let out = ask(hormone, &format!("secretion_trigger({hormone}, $Level)"));

        assert!(
            out.contains(&format!(
                "\"term\":\"secretion_trigger({hormone}, {level})\""
            )),
            "{hormone} should bind {level}: {out}"
        );
        assert!(
            out.contains(&citation(own)),
            "{hormone} must carry its OWN whole citation, corroborations empty: {out}"
        );
        assert!(
            !out.contains(other),
            "{hormone} must not be warranted by the other row's sentence: {out}"
        );
        assert!(
            !out.contains(ENVELOPE),
            "the framing sentence must warrant no answer: {out}"
        );
        // COUNTED, not needled. This arm used to read
        // `!out.contains("\"corroborations\":[\"")` -- looking for a STRING
        // inside the array. The CLI renders corroborations as OBJECTS
        // (`"corroborations":[{`), so that needle could never match and the
        // assertion could not fail for the defect it named. It was proved dead
        // by restoring the pre-conversion `cites` and watching it pass.
        //
        // Comparing the empty count against the TOTAL count cannot go stale
        // the same way: any non-empty array, whatever it contains, breaks the
        // equality. The non-zero check stops it passing vacuously if the key
        // ever stops being rendered at all.
        let total = out.matches("\"corroborations\":").count();
        let empty = out.matches("\"corroborations\":[]").count();
        assert!(
            total > 0,
            "provenance must render corroborations at all, else this arm is \
             vacuous: {out}"
        );
        assert_eq!(
            empty, total,
            "relocating the span leaves EVERY corroborations array empty \
             ({empty} of {total}): {out}"
        );
    }
}

#[test]
fn backward_recall_binds_glucagon_to_low_with_its_own_sentence() {
    let out = ask("low", "secretion_trigger($Hormone, low)");

    assert!(
        out.contains("\"term\":\"secretion_trigger(glucagon, low)\""),
        "glucagon should be the only recalled low-glucose trigger: {out}"
    );
    assert!(
        !out.contains("secretion_trigger(insulin, low)"),
        "insulin is triggered by high glucose, not low: {out}"
    );
    assert!(
        out.contains(&citation(GLUCAGON_SPAN)),
        "the backward bind carries the glucagon sentence itself: {out}"
    );
    assert!(
        !out.contains(INSULIN_SPAN),
        "the backward bind must not surface the insulin sentence: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the framing sentence must warrant no answer: {out}"
    );
}

#[test]
fn abstains_on_a_hormone_with_no_glucose_trigger() {
    let out = ask("abstain", "secretion_trigger(thyroxine, $Level)");
    assert!(
        out.contains("\"abstained\":true"),
        "thyroxine has no blood-glucose trigger in the cited span -- honest abstention expected: {out}"
    );
}

#[test]
fn the_envelope_frames_the_organ_and_names_no_part_of_the_taxonomy() {
    let text = lib_text();

    // Exactly one table-level source (4 spaces), and two row-level ones (8).
    let table_level: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect();
    let row_level = text
        .lines()
        .filter(|l| l.starts_with("        source \""))
        .count();
    assert_eq!(
        table_level.len(),
        1,
        "exactly one table-level source line: {text}"
    );
    assert_eq!(row_level, 2, "each row carries its own source: {text}");

    // Read the envelope OUT OF THE SHIPPED FILE rather than trusting the
    // constant above -- otherwise editing the constant could quietly retire
    // this check.
    let envelope = table_level[0]
        .trim_start_matches("    source \"")
        .trim_end_matches('"');
    assert_eq!(
        envelope, ENVELOPE,
        "the shipped framing sentence is the one this test reasons about"
    );

    let lowered = envelope.to_lowercase();
    for token in FORBIDDEN_IN_ENVELOPE {
        assert!(
            !lowered.contains(token),
            "the framing sentence must name no part of the page's hormone/cell \
             taxonomy, but it contains {token:?}: {envelope}"
        );
    }

    // FORBIDDING IS NOT FRAMING. A purely negative arm passes any text that
    // merely avoids the taxonomy -- including this page's own left-navigation
    // strings ("Ovaries Genital Tract External Genitalia", "Review Module
    // (Cancer As a Disease)"), which name no hormone and frame nothing at all.
    // The envelope has to be ABOUT the organ, not just innocent of the rows.
    assert!(
        lowered.contains("pancreas"),
        "the framing sentence must actually frame this table's subject: \
         {envelope}"
    );
}

#[test]
fn each_row_header_is_followed_by_its_own_span() {
    let text = lib_text();

    // Anchored on the ROW HEADER, so the assertion cannot be satisfied by a
    // source line that belongs to a different row.
    assert!(
        text.contains(&format!(
            "row (insulin, high) {{\n        source \"{INSULIN_SPAN}\""
        )),
        "the insulin row is followed by the insulin sentence: {text}"
    );
    assert!(
        text.contains(&format!(
            "row (glucagon, low) {{\n        source \"{GLUCAGON_SPAN}\""
        )),
        "the glucagon row is followed by the glucagon sentence: {text}"
    );

    // The page's own plural must survive verbatim.
    assert!(
        text.contains("the hormone glucagons"),
        "the cited page spells it \"glucagons\"; it ships verbatim: {text}"
    );
}

/// The span sitting under a row header, read out of the shipped file.
/// Anchored on the ROW HEADER so it cannot return the neighbouring row's
/// sentence -- the two differ in only three tokens.
fn row_span(text: &str, hormone: &str, level: &str) -> String {
    let head = format!("row ({hormone}, {level}) {{\n        source \"");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("no row header for {hormone}: {text}"));
    let rest = &text[start + head.len()..];
    let end = rest.find('"').expect("row source must be a closed string");
    rest[..end].to_string()
}

#[test]
fn each_span_states_the_level_its_row_compresses() {
    // Knowing WHICH span sits under a row is not the same as knowing the span
    // SUPPORTS that row. Without this arm, a row handed real page prose about
    // the right organ but stating no trigger at all -- "The endocrine portion
    // consists of the pancreatic islets, which secrete glucagons and
    // insulin." -- reads as entirely plausible provenance and survives every
    // other assertion here. That is the #15318 shape.
    //
    // (hormone, level, the phrase the atom compresses, the cell that secretes)
    let cases = [
        (
            "insulin",
            "high",
            "high concentration of glucose in the blood",
            "Beta cells",
        ),
        (
            "glucagon",
            "low",
            "low concentration of glucose in the blood",
            "Alpha cells",
        ),
    ];

    let text = lib_text();
    for (hormone, level, phrase, cell) in cases {
        let span = row_span(&text, hormone, level);
        assert!(
            span.contains(phrase),
            "the {hormone} span must state the {level} blood-glucose condition \
             its atom compresses, but reads: {span}"
        );
        assert!(
            span.contains(cell),
            "the {hormone} span must name the cell type that secretes it, but \
             reads: {span}"
        );
        assert!(
            span.contains(hormone),
            "the {hormone} span must name its own hormone, but reads: {span}"
        );
    }
}

#[test]
fn no_cites_remains_and_locator_and_trust_are_stated_once() {
    let text = lib_text();

    // SCOPE: this pins a convention local to THIS table, not a language rule.
    // `lower.rs`'s row path accepts Source, Locator, Trust, Cites and Quote,
    // and other shipped tables do carry a row-level `cites`.
    assert!(
        !text.contains("cites \""),
        "the glucagon sentence moved to its row; no `cites` should remain at \
         any indent: {text}"
    );
    assert_eq!(
        text.matches("    locator \"").count(),
        1,
        "one inherited locator: {text}"
    );
    assert_eq!(
        text.matches("    trust ").count(),
        1,
        "one inherited trust: {text}"
    );
}
