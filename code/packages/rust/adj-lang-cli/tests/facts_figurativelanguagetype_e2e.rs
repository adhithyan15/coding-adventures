//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/figurative-language-type.adj`) driven
//! through the built CLI: a native `table` naming three figures of speech
//! and what each actually does, quoted verbatim from Grammarly's
//! "Figurative Language Examples: 6 Common Types and Definitions"
//! article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the METAPHOR sentence, so a personification or hyperbole answer was
//! warranted primarily by a sentence about metaphors -- which is why the
//! glyph-for-glyph pin below used to bind a hyperbole answer to the metaphor
//! citation. It now pins the METAPHOR answer to the metaphor sentence, which
//! is also the row carrying the curly apostrophe that pin exists to protect.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_figurative_language_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/figurative-language-type.adj");
    std::fs::copy(&src, dir.join("figurative-language-type.adj")).expect("copy shipped figurative-language-type.adj");
}

#[test]
fn figurative_language_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type(hyperbole, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"a_great_exaggeration_used_to_add_emphasis\""),
        "hyperbole means a_great_exaggeration_used_to_add_emphasis: {out}"
    );
    // This was `contains("grammarly.com") && contains("\"trust\":\"consensus\"")`,
    // which any Grammarly citation satisfies and which constrains no sentence
    // text. The needle is now the whole citations array as the serialiser emits
    // it, closing on both the corroborations `]` and the citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(HYPERBOLE)),
        "the hyperbole answer carries the hyperbole definition, whole: {out}"
    );
    assert!(!out.contains(METAPHOR), "the metaphor sentence reaches no hyperbole answer: {out}");
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/writing-tips/figurative-language/";
const ENVELOPE: &str = "Figurative language is a type of communication that does not use a word\u{2019}s strict or literal meaning.";
/// The page renders the apostrophe as U+2019. An ASCII-apostrophe variant of
/// this sentence occurs ZERO times on the page -- the whole premise being that
/// a caller can check a citation against its locator.
const METAPHOR: &str = "A metaphor describes an object or action in a way that isn\u{2019}t literally true but helps explain an idea or make a comparison.";
const PERSONIFICATION: &str = "Personification means giving human characteristics to nonhuman or abstract things.";
const HYPERBOLE: &str = "Hyperbole is a great exaggeration, often unrealistic, to add emphasis to a sentiment.";

/// (type, its description atom, the definition that names both)
fn types() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "metaphor",
            "describes_something_in_a_way_thats_not_literally_true_to_make_a_comparison",
            METAPHOR,
        ),
        (
            "personification",
            "gives_human_characteristics_to_nonhuman_or_abstract_things",
            PERSONIFICATION,
        ),
        ("hyperbole", "a_great_exaggeration_used_to_add_emphasis", HYPERBOLE),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/figurative-language-type.adj"))
        .expect("read shipped figurative-language-type.adj");
    adj[adj.find("table figurative_language_type").expect("table")..].to_string()
}

#[test]
fn every_type_answer_carries_only_the_definition_that_names_it() {
    for (kind, description, span) in types() {
        let dir = scratch(&format!("type_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"figurative-language-type.adj\"\n? figurative_language_type({kind}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"D\":\"{description}\"")), "{kind} -> {description}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{kind}: its own definition, whole, at the consensus tier, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(kind),
            "the span carried by {kind} names {kind} -- the defect was that it did not"
        );
        for other in [METAPHOR, PERSONIFICATION, HYPERBOLE] {
            if other != span {
                assert!(!out.contains(other), "another type's definition must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the trust tier
    // and the page's curly apostrophes.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    // Not `"\n    cites "`: that is indent-scoped and would miss a row-level
    // `cites` at eight spaces. The per-type test's closed needle catches one
    // either way, but this assertion should stand on its own.
    assert!(!body.contains("cites "), "no corroboration at any indent");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n"
        )),
        "the envelope is the article's framing sentence, at the consensus tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{METAPHOR}\"\n    locator")),
        "not the metaphor sentence as the envelope again"
    );
    assert!(
        body.contains(&format!("        source \"{METAPHOR}\"\n")),
        "the metaphor row keeps the page's U+2019, not an ASCII apostrophe"
    );
    // SCOPED TO `source "` LINES, not the whole table block -- the correction
    // #15337 made for plant-parts and #15338 for solar-eclipse-type. Read
    // against the block, this arm fails on a harmless `%` comment quoting the
    // sentence in ASCII, and a comment is not a shipped citation.
    //
    // Stated with its side: this table carries ZERO `%` comments inside its
    // block today, so the false alarm here is AVAILABLE rather than ACTIVE.
    // eye-part-property.adj, corrected in the same change, has four comments in
    // its block and is the live case. That is the only difference -- both arms
    // were measured firing, this one at its own assert line under a mutant that
    // plants the ASCII form on a NON-metaphor row (the three obvious routes are
    // each dominated by an earlier guard: the row-source count, the
    // not-metaphor-as-envelope arm, and the row-keeps-U+2019 arm).
    //
    // The trade, recorded in shards 03560 and 03670: scoping lets a
    // comment-borne ASCII form survive. Nothing now pins "no ASCII metaphor
    // span anywhere in the block", only "no `source` line carries one".
    //
    // The positive half is already anchored above at eight spaces, so it needs
    // no new scoping: a mutant planting this span at table level would fail it.
    let source_lines: String = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !source_lines.contains(&METAPHOR.replace('\u{2019}', "'")),
        "no `source` line may carry an ASCII-apostrophe metaphor span -- that \
         spelling occurs zero times on the page: {source_lines}"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in ["metaphor", "personification", "hyperbole", "comparison", "exaggeration"] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no type and no description, but contains {word:?}"
        );
    }
}

#[test]
fn figurative_language_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type($T, gives_human_characteristics_to_nonhuman_or_abstract_things)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"personification\""),
        "the shipped gives_human_characteristics_to_nonhuman_or_abstract_things example is personification: {out}"
    );
}

#[test]
fn figurative_language_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type(allusion, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "allusion is a real device the source names with its own clean sentence, but one that works by referencing an external work/person/event, a different mechanism than the three tabled here -- honest abstention, never invented: {out}"
    );
}

// RE-POINTED BY THE RS-5e CONVERSION. This pin used to bind the HYPERBOLE
// answer's `D` to the METAPHOR sentence -- which was correct only because the
// metaphor sentence was the table envelope and rode on every answer. That is
// the defect #14986 removes, so the pin now binds the metaphor answer to the
// metaphor sentence. What it was written to protect is unchanged: the page
// renders U+2019, an ASCII-apostrophe variant occurs zero times there, and a
// citation a caller cannot find on its own locator grounds nothing.
const FIGURATIVE_LANGUAGE_TYPE_PIN: &str = r#""bindings":{"D":"describes_something_in_a_way_thats_not_literally_true_to_make_a_comparison"},"citations":[{"source":"A metaphor describes an object or action in a way that isn’t literally true but helps explain an idea or make a comparison.","locator":"https://www.grammarly.com/blog/writing-tips/figurative-language/","trust":"consensus""#;

#[test]
fn figurative_language_type_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"
? figurative_language_type(metaphor, $D)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped citation carried an ASCII apostrophe where the page renders
    // U+2019, so it did not appear on its own page -- the whole premise being
    // that a caller can check a citation against its locator.
    assert!(
        out.contains(FIGURATIVE_LANGUAGE_TYPE_PIN),
        "the figurative language type citation matches its page: {out}"
    );
}
