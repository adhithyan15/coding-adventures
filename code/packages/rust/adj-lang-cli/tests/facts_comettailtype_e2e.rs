//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/comet-tail-type.adj`) driven through the
//! built CLI: a native `table` naming the two separate tails a comet
//! actually has and the defining path each one traces, quoted verbatim from
//! NASA Space Place's "What Is a Comet?" page -- the same page the sibling
//! `comet-part.adj` already cites. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the dust-tail span, so an ion-tail answer was warranted primarily by a
//! sentence about the DUST tail -- while the ion-tail sentence sat in a
//! table-level `cites`, riding along as a corroboration rather than as that
//! row's warrant.
//!
//! THAT `cites` IS WHY `only_citation()` CLOSES ON AN EMPTY corroborations
//! ARRAY. Before the conversion the array was non-empty, so that needle could
//! not have matched at all; it became satisfiable only when the `cites` moved
//! into the ion_tail row's own `source`.
//!
//! The two row spans are MUTUALLY EXCLUSIVE -- the dust span names "dust", the
//! ion span names "ion", neither names the other -- so a negative arm may name
//! a whole span without the shared-word hazard `comet-part.adj` has, where all
//! three spans contain "nucleus".

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comet_tail_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/comet-tail-type.adj");
    std::fs::copy(&src, dir.join("comet-tail-type.adj")).expect("copy shipped comet-tail-type.adj");
}

#[test]
fn comet_tail_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type(dust_tail, $D)\n",
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
    //
    // RE-POINTED (RS-5e, #14986). This pinned the whole sentence but not the
    // whole CITATION, and that sentence was the table's envelope -- so it rode
    // on every answer, including the ion_tail one. It is now the dust_tail
    // row's own citations array, closing on both the corroborations `]` and the
    // citations `]`, which additionally pins that the corroborations array is
    // EMPTY. Before this conversion it could not have been: the ion-tail
    // sentence shipped as a table-level `cites`.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one citations array: {out}"
    );
    assert!(
        out.contains(&only_citation(DUST)),
        "the dust_tail answer carries the sentence defining the dust tail, whole, \
         and as its only citation: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"traces_a_broad_gently_curving_path_away_from_the_sun\""),
        "dust_tail traces a broad, gently curving path away from the Sun: {out}"
    );
    // This was `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any NASA citation satisfies and which constrains no sentence text.
    // The whole-array needle above is what pins the sentence now; this keeps
    // the locator pinned as its own assertion rather than joined by `&&`,
    // because a joined failure cannot say which arm broke.
    assert!(
        out.contains(&format!("\"locator\":\"{LOCATOR}\"")),
        "carries the NASA locator: {out}"
    );
}

#[test]
fn comet_tail_type_reverse_binds_the_tail_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type($T, always_points_directly_away_from_the_sun)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"ion_tail\""),
        "the shipped always_points_directly_away_from_the_sun example is ion_tail: {out}"
    );
}

#[test]
fn comet_tail_type_abstains_honestly_on_a_different_physical_part() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type(coma, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "coma is a real comet physical part (tabled in the sibling comet-part.adj) but not one of the two tail sub-types tabled here -- honest abstention, never invented: {out}"
    );
}

const LOCATOR: &str = "https://spaceplace.nasa.gov/comets/en/";
/// The page's sentence framing the two-tail split. It names neither tail -- not
/// as the phrases "dust tail"/"ion tail", and not as the bare tokens
/// "dust"/"ion" -- and shares no content word with either row's value.
const ENVELOPE: &str = "When astronomers look closely, they find that comets actually have two separate tails.";
const DUST: &str = "This dust tail traces a broad, gently curving path away from the Sun.";
const ION: &str = "The ion tail always points directly away from the Sun.";

/// (tail, its description atom, the NASA sentence defining that tail)
///
/// Unlike `comet-part.adj`, whose three spans all contain the word "nucleus",
/// these two are mutually exclusive: the dust span names "dust", the ion span
/// names "ion", and neither names the other. So a negative arm may name a whole
/// span without the shared-word hazard that table has.
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("dust_tail", "traces_a_broad_gently_curving_path_away_from_the_sun", DUST),
        ("ion_tail", "always_points_directly_away_from_the_sun", ION),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
///
/// NOTE: before the RS-5e conversion this table carried a table-level `cites`,
/// so `corroborations` was NON-empty and this needle could not match at all.
/// It became satisfiable only when that `cites` was relocated into the ion_tail
/// row's own `source`.
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/comet-tail-type.adj"))
        .expect("read shipped comet-tail-type.adj");
    adj[adj.find("table comet_tail_type").expect("table")..].to_string()
}

#[test]
fn every_tail_answer_carries_only_the_sentence_defining_that_tail() {
    for (tail, description, span) in scale() {
        let dir = scratch(&format!("tail_{tail}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"comet-tail-type.adj\"\n? comet_tail_type({tail}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {tail}: {out}"
        );
        assert!(
            out.contains(&format!("\"D\":\"{description}\"")),
            "{tail} -> {description}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{tail}: the NASA sentence defining it, whole, and the only citation: {out}"
        );
        for other in [DUST, ION] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "the other tail's sentence must not reach {tail}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {tail}: {out}"
        );
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        2,
        "two row sources"
    );
    // THIS ARM WOULD HAVE FAILED ON THE PRE-CONVERSION FILE. This table shipped
    // the ion-tail sentence as a table-level `cites` with its own redundant
    // `locator`, so the ion_tail row's warrant was the DUST-tail envelope and
    // its own sentence was a mere corroboration. Keyword-anchored, so a `cites`
    // at any indent fails it.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent: {body}"
    );
    // Indent-independent: the envelope has a `locator` and a `trust` of its
    // own, so the pin is EXACTLY one of each. The pre-conversion file had TWO
    // locator lines -- the envelope's and the one inside the `cites`.
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
        "the envelope is the page's two-tail framing sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{DUST}\"\n    locator")),
        "not the dust_tail span as the envelope again"
    );
    // WHOLE WORDS, not substrings. "tail" is a token of both row keys and of
    // the envelope itself, so the needles are the DISTINGUISHING words: an
    // envelope naming one tail specifically is what must fail. Compare tokens
    // because a substring check for "ion" matches inside "formation".
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["dust", "ion"] {
        assert!(
            !tokens.contains(&word),
            "the shipped envelope must name neither tail, but contains {word:?} \
             as a whole word: {shipped_envelope}"
        );
    }
}

#[test]
fn every_row_description_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs: the atom is a phrase drawn from the sentence, so the normalized
    // span must contain the distinguishing word AND the atom's content.
    // This is what `element-categories` (#15318) failed.
    let body = shipped_table();
    for (tail, description, span) in scale() {
        assert!(
            body.contains(&format!("        source \"{span}\"\n")),
            "{tail} carries its own span: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        // Whole word, not substring: the comet-part test shipped a substring
        // check for the part name and the reviewer caught it -- "tail" is
        // satisfied by "tails", "ion" by "formation".
        let distinguishing = tail.split('_').next().expect("row key has a prefix");
        assert!(
            tokens.contains(&distinguishing),
            "{tail}: its own span must name {distinguishing:?} as a whole word, \
             but it is not a token of {normalized:?}"
        );
        // THE WHOLE ATOM, not hand-picked fragments of it. In this table each
        // atom is a CONTIGUOUS phrase of its own span once underscores become
        // spaces, so the strongest available check is that the span states the
        // atom entire.
        //
        // Hand-picked needles leave the parts they omit unchecked, and the
        // omitted part is where a mutation hides: with `ion_tail` pinned only
        // on "always points directly away", a span reading "...directly away
        // from the COMET" passes while failing to support
        // `always_points_directly_away_from_the_sun`. Same for `dust_tail` and
        // "...a broad, gently curving path TOWARD the Sun".
        //
        // `comet-part.adj` cannot do this -- its atoms REORDER their sentences
        // ("solid_frozen_core_at_the_heart_of_the_comet" against "At the heart
        // of every comet is a solid, frozen core...") -- which is why that
        // table needs distinctive phrases and this one does not.
        let atom_phrase = description.replace('_', " ");
        assert!(
            normalized.contains(&atom_phrase),
            "{tail}: its span must state the whole atom {description:?} as the contiguous \
             phrase {atom_phrase:?}, but it is not in {normalized:?}"
        );
    }
}
