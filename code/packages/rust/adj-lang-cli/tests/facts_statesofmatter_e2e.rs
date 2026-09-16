//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/states-of-matter.adj`) driven through the built
//! CLI: a native `table` of state-of-matter -> defining property resolves a
//! binding-query recall with the source's citation, and abstains on a state not
//! in the table — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN CITATION (RS-5e, #14986). Until that change the
//! table envelope was the SOLID sentence, so a `liquid` or `gas` recall came
//! back warranted primarily by a sentence about solids, and those two rows'
//! own sentences reached no answer at all.
//!
//! WHY THE PINS LOOK LIKE THIS. The previous assertion here was
//! `out.contains("grc.nasa.gov") && out.contains("\"trust\":\"authoritative\"")`
//! -- satisfied by ANY NASA citation, constraining no sentence text, and
//! satisfied before this change by the solid span riding on the gas answer. It
//! could not fail for the bug it was meant to cover. Each answer is now pinned
//! to its own WHOLE serialised citation object (#14735).
//!
//! THE LIQUID ROW IS THE INTERESTING ONE. Its header used to quote a span
//! STITCHED with an ellipsis -- "A liquid will take the shape of its container
//! ... a liquid has a fixed volume" -- but those are two NON-ADJACENT page
//! sentences, separated by "In microgravity, a liquid forms a ball inside a
//! free surface." An elided quotation is not verbatim. The row now ships the
//! CONTIGUOUS sentence as its `source`, and the fixed-volume sentence as its
//! own `cites`, so this is the one row whose corroborations array is NOT empty.
//!
//! `liquid` was never queried by this test before, which is exactly why the
//! stitched quotation went unnoticed. It is queried now.

use std::path::{Path, PathBuf};
use std::process::Command;

const LOCATOR: &str = "https://www.grc.nasa.gov/www/k-12/airplane/state.html";

const SOLID_SPAN: &str =
    "A solid holds its shape and the volume of a solid is fixed by the shape of the solid.";
const LIQUID_SPAN: &str =
    "A liquid will take the shape of its container with a free surface in a gravitational field.";
const GAS_SPAN: &str =
    "A gas fills its container, taking both the shape and the volume of the container.";

/// The liquid row's corroboration: real page text that supports the atom
/// without stating it, carried as `cites` rather than welded into the source.
const LIQUID_CITES: &str = "Regardless of gravity, a liquid has a fixed volume.";

/// The table-level framing sentence. It must reach NO answer: the rows carry
/// their own spans, and the envelope only supplies `locator` and `trust`.
const ENVELOPE: &str = "We call this property of matter the phase of the matter.";

/// The page's whole state taxonomy -- not merely this table's three row keys.
/// `plasma` is the fourth state the header deliberately excludes, so a frame
/// naming it would frame a member of the taxonomy while naming no row key.
const FORBIDDEN_IN_ENVELOPE: &[&str] = &["solid", "liquid", "gas", "plasma"];

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn lib_text() -> String {
    std::fs::read_to_string(facts_stdlib().join("chemistry/states-of-matter.adj"))
        .expect("read shipped states-of-matter.adj")
}

/// A scratch directory unique to THIS CALL, not merely to this tag.
///
/// The first version keyed the path on `tag + process id` alone. Tests run in
/// parallel threads of ONE process, and two different tests here both call
/// `ask("solid", ..)` and `ask("gas", ..)` from inside loops -- so they derive
/// the SAME path, and each begins with `remove_dir_all` followed by
/// `create_dir_all`. One thread can therefore delete the files another is
/// about to read.
///
/// That is an intermittent failure by construction: it flakes rather than
/// fails, which is worse, because a green run proves nothing. The counter
/// makes every call's directory distinct, so the race cannot occur at all.
fn scratch(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "adjcli_factssom_{tag}_{}_{n}",
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

fn ask(tag: &str, goal: &str) -> String {
    let dir = scratch(tag);
    let src = facts_stdlib().join("chemistry/states-of-matter.adj");
    std::fs::copy(&src, dir.join("states-of-matter.adj"))
        .expect("copy shipped states-of-matter.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"states-of-matter.adj\"\n? {goal}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

/// The whole serialised citation object for a row with NO corroboration.
/// Written against the CLI's compact output (no spaces after colons), checked
/// against real stdout rather than guessed.
fn citation(span: &str) -> String {
    format!(
        "{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}"
    )
}

/// The liquid row's citation, closing on its corroboration object. A
/// corroboration renders as `{"source":...,"locator":...}` -- no `trust`, no
/// nesting -- so this cannot be satisfied by a differently-shaped object.
fn citation_with_corroboration(span: &str, corroboration: &str) -> String {
    format!(
        "{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\
         \"corroborations\":[{{\"source\":\"{corroboration}\",\"locator\":\"{LOCATOR}\"}}]}}"
    )
}

#[test]
fn each_state_recalls_its_property_warranted_by_its_own_sentence() {
    // (state, property, its own span, the spans of the OTHER two rows)
    let cases = [
        ("solid", "fixed_shape", SOLID_SPAN, [LIQUID_SPAN, GAS_SPAN]),
        (
            "liquid",
            "takes_shape_of_container",
            LIQUID_SPAN,
            [SOLID_SPAN, GAS_SPAN],
        ),
        ("gas", "fills_container", GAS_SPAN, [SOLID_SPAN, LIQUID_SPAN]),
    ];

    for (state, property, own, others) in cases {
        let out = ask(state, &format!("matter_state({state}, $Property)"));

        assert!(
            out.contains(&format!("\"Property\":\"{property}\"")),
            "{state} should bind {property}: {out}"
        );
        assert!(
            out.contains(own),
            "{state} must be warranted by its OWN sentence: {out}"
        );
        // WHOLE-SPAN negative arms. The three sentences share vocabulary
        // ("shape", "volume", "container"), so a short needle would match more
        // than one and pass whichever sentence came back.
        for other in others {
            assert!(
                !out.contains(other),
                "{state} must not be warranted by another row's sentence: {out}"
            );
        }
        assert!(
            !out.contains(ENVELOPE),
            "the framing sentence must warrant no answer: {out}"
        );
    }
}

#[test]
fn solid_and_gas_carry_a_whole_citation_with_no_corroboration() {
    for (state, span) in [("solid", SOLID_SPAN), ("gas", GAS_SPAN)] {
        let out = ask(state, &format!("matter_state({state}, $Property)"));
        assert!(
            out.contains(&citation(span)),
            "{state} must carry its own WHOLE citation, corroborations empty: {out}"
        );
    }
}

#[test]
fn the_liquid_row_carries_its_corroboration_and_it_is_not_empty() {
    let out = ask("liquidcite", "matter_state(liquid, $Property)");

    assert!(
        out.contains(&citation_with_corroboration(LIQUID_SPAN, LIQUID_CITES)),
        "the liquid answer carries its whole citation INCLUDING the \
         fixed-volume corroboration: {out}"
    );
    // This is the one row in the table whose corroborations array is NOT empty,
    // so an assertion that it is empty would be wrong here -- the opposite of
    // every other conversion in this batch.
    assert!(
        out.contains("\"corroborations\":[{"),
        "the liquid row's corroborations array must be non-empty: {out}"
    );
    // THE ELIDED FORM MUST NEVER APPEAR -- AND THIS ARM HAS TO LOOK WHERE IT
    // COULD ACTUALLY LIVE.
    //
    // The first version asserted this over the CLI's stdout. The stitched
    // quotation was in the .adj's HEADER COMMENT, and the CLI emits no comment
    // text at all: measured, stdout is ~3.3 KB and contains no "% ", no
    // "Provenance", no header prose. So that assertion could not have caught
    // the original defect and could not catch its return -- an assertion never
    // observed to fire is decoration.
    //
    // It now reads the shipped FILE, and screens both the ASCII "..." and the
    // U+2026 "…" form, because the typographic one is what a later edit is
    // most likely to introduce.
    let text = lib_text();
    for form in ["container ... a liquid", "container \u{2026} a liquid"] {
        assert!(
            !text.contains(form),
            "the stitched ellipsis quotation must not be shipped (form {form:?}): {text}"
        );
    }
}

#[test]
fn abstains_on_plasma_the_fourth_state() {
    let out = ask("plasma", "matter_state(plasma, $Property)");
    assert!(
        out.contains("\"abstained\":true"),
        "plasma is the 4th state and is not tabled here -- honest abstention: {out}"
    );
}

#[test]
fn the_envelope_frames_matter_and_names_no_state() {
    let text = lib_text();

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
    assert_eq!(row_level, 3, "each row carries its own source: {text}");

    // Read the envelope OUT OF THE SHIPPED FILE rather than trusting the
    // constant above -- otherwise editing the constant could retire this check.
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
            "the framing sentence must name no state, but it contains \
             {token:?}: {envelope}"
        );
    }

    // WHAT THESE TOKEN ARMS DO AND DO NOT ESTABLISH.
    //
    // They do NOT select this sentence. Measured on the page: FIVE sentences
    // contain "matter" and name none of solid/liquid/gas/plasma, including
    // "Changes in the phase of matter are physical changes, not chemical
    // changes." and "The three normal phases of matter have unique
    // characteristics which are listed on the slide." All five pass both arms.
    //
    // The sentence is pinned by the `assert_eq!` above. These arms exist to
    // constrain a CONST CO-EDIT: a mutant that rewrites ENVELOPE alongside the
    // file still has to pass them, which is what kills a frame naming a state
    // (M06) or one that frames nothing at all (M05). They rule out the page's
    // nav string and its off-subject prose; they do not prove the sentence
    // frames the subject.
    assert!(
        lowered.contains("matter"),
        "a co-edited envelope must still name the subject: {envelope}"
    );
}

/// The span sitting under a row header, read out of the shipped file. Anchored
/// on the ROW HEADER so it cannot return a neighbouring row's sentence.
fn row_span(text: &str, state: &str, property: &str) -> String {
    let head = format!("row ({state}, {property}) {{\n        source \"");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("no row header for {state}: {text}"));
    let rest = &text[start + head.len()..];
    let end = rest.find('"').expect("row source must be a closed string");
    rest[..end].to_string()
}

#[test]
fn each_row_header_is_followed_by_its_own_span() {
    let text = lib_text();
    for (state, property, span) in [
        ("solid", "fixed_shape", SOLID_SPAN),
        ("liquid", "takes_shape_of_container", LIQUID_SPAN),
        ("gas", "fills_container", GAS_SPAN),
    ] {
        assert_eq!(
            row_span(&text, state, property),
            span,
            "the {state} row is followed by the {state} sentence"
        );
    }
}

#[test]
fn each_span_states_the_container_behaviour_its_row_compresses() {
    // Knowing WHICH span sits under a row is not the same as knowing the span
    // SUPPORTS that row. A row handed real page prose about the right subject
    // but stating no container behaviour -- "In microgravity, a liquid forms a
    // ball inside a free surface.", the sentence that sits BETWEEN the two
    // halves of the old stitched quote -- reads as entirely plausible
    // provenance. That is the #15318 shape.
    //
    // THE THREAT MODEL, stated precisely rather than flattered: a bare swap is
    // already caught by `each_row_header_is_followed_by_its_own_span`, which
    // asserts full equality against LIQUID_SPAN. This arm is the second lock,
    // load-bearing exactly when a mutation ALSO co-edits LIQUID_SPAN -- which
    // is how the harness runs it (M07). It is in turn defeatable by a two-const
    // co-edit that rewrites the phrase literal below as well; no single arm
    // here is unconditional.
    //
    // (state, property, the phrase the atom compresses, the state word)
    let cases = [
        ("solid", "fixed_shape", "holds its shape", "solid"),
        (
            "liquid",
            "takes_shape_of_container",
            "will take the shape of its container",
            "liquid",
        ),
        ("gas", "fills_container", "fills its container", "gas"),
    ];

    let text = lib_text();
    for (state, property, phrase, word) in cases {
        let span = row_span(&text, state, property);
        assert!(
            span.contains(phrase),
            "the {state} span must state the container behaviour its atom \
             compresses, but reads: {span}"
        );
        assert!(
            span.to_lowercase().contains(word),
            "the {state} span must name its own state, but reads: {span}"
        );
    }
}

#[test]
fn one_locator_one_trust_and_the_only_cites_is_the_liquid_rows() {
    let text = lib_text();

    // SCOPE: this pins a convention local to THIS table, not a language rule.
    // ADJ-A9 allows `cites` at table or row level, and 85 row-level `cites`
    // lines ship across this corpus.
    assert_eq!(
        text.matches("cites \"").count(),
        1,
        "exactly one cites, on the liquid row: {text}"
    );
    assert!(
        text.contains(&format!(
            "        cites \"{LIQUID_CITES}\" locator \"{LOCATOR}\""
        )),
        "the liquid cites carries its own locator -- ADJ-A9 requires it and a \
         bare cites does not parse: {text}"
    );
    // ONE line BEGINS with four spaces and `locator "` -- the envelope's.
    //
    // The file contains a SECOND locator, on the liquid row's `cites` line,
    // but it sits mid-line after `cites "..." ` and so never matches a needle
    // anchored at that indent. I first asserted 2 here by reasoning about how
    // many locators the file has rather than counting how many match the
    // needle, and the test failed (left: 1, right: 2). The file was right.
    // That row's locator is pinned verbatim by the assertion above instead.
    assert_eq!(
        text.matches("    locator \"").count(),
        1,
        "one envelope locator line at that indent: {text}"
    );
    assert_eq!(
        text.matches("    trust ").count(),
        1,
        "one inherited trust: {text}"
    );
}
