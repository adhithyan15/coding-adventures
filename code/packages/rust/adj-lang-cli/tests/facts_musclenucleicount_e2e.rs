//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-nuclei-count.adj`) driven through the
//! built CLI: a native `table` recording whether skeletal or cardiac
//! muscle fibers are multinucleated or have a single nucleus -- a sibling
//! to the already-shipped `tissue-types.adj` (which only carries a
//! representative example/location per tissue type), decoding the
//! nuclei-count clause already sitting unused inside that table's own
//! muscle-row header quotes. Resolves forward and backward recall queries
//! with the source's citation, plus honest abstention on smooth muscle
//! (outside this cited span) -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the `skeletal` span, and the cardiac sentence rode as a table-level
//! `cites`, so a `cardiac` recall was warranted primarily by a sentence about
//! SKELETAL muscle. Relocating it empties `corroborations` on every answer.
//!
//! THE ENVELOPE MUST NAME NO MUSCLE TYPE -- not merely neither row key. This
//! page describes a THIRD type, `smooth`, which is deliberately no row here,
//! and it states a real nuclei count for it ("Smooth muscle cells are spindle
//! shaped, have a single, centrally located nucleus, and lack striations.").
//! That sentence names neither row key, so a "names no row key" rule would
//! admit it as an envelope -- and planted as a ROW SOURCE it would read as
//! entirely plausible provenance. `the_table_shape_matches_the_measured_rows`
//! forbids the whole taxonomy.
//!
//! NO INVISIBLE-BYTE HAZARD HERE: the page carries zero U+00A0, U+2013 and
//! U+2019 -- in fact zero non-ASCII codepoints at all -- measured up front.
//! Shards 03570 (`mixture-types`, an NBSP envelope) and 03590 (`sun-layer`, an
//! en dash and a right single quote) each shipped one; 03580
//! (`blood-cell-types`) did not, so this is not a property of every conversion
//! in the batch.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_musclenucleicount_{tag}_{}",
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
    let src = facts_stdlib().join("biology/muscle-nuclei-count.adj");
    std::fs::copy(&src, dir.join("muscle-nuclei-count.adj"))
        .expect("copy shipped muscle-nuclei-count.adj");
}

const LOCATOR: &str =
    "https://training.seer.cancer.gov/anatomy/cells_tissues_membranes/tissues/muscle.html";

/// The page's framing sentence. Names no muscle type at all, so it warrants
/// neither row by itself.
const ENVELOPE: &str = "Muscle tissue is composed of cells that have the special ability to shorten or contract in order to produce movement of the body parts.";

const SKELETAL: &str =
    "Skeletal muscle fibers are cylindrical, multinucleated, striated, and under voluntary control.";
const CARDIAC: &str =
    "Cardiac muscle has branching fibers, one nucleus per cell, striations, and intercalated disks.";

/// (muscle type, its nuclei atom, the SEER sentence stating that count)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("skeletal", "multinucleated", SKELETAL),
        ("cardiac", "single_nucleus", CARDIAC),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
///
/// NOTE: before the RS-5e conversion this table carried a table-level `cites`,
/// so `corroborations` was NON-empty and this needle could not match at all.
/// It became satisfiable only when that `cites` moved into the cardiac row.
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/muscle-nuclei-count.adj"))
        .expect("read shipped muscle-nuclei-count.adj");
    adj[adj.find("table muscle_nuclei_count").expect("table")..].to_string()
}

#[test]
fn muscle_nuclei_count_recalls_skeletal_as_multinucleated_with_citation() {
    let dir = scratch("skeletal");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count(skeletal, $Nuclei)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"muscle_nuclei_count(skeletal, multinucleated)\""),
        "skeletal muscle should recall as multinucleated: {out}"
    );
    // THIS PIN WAS `contains("seer.cancer.gov") &&
    // contains("\"trust\":\"authoritative\"")` -- satisfied by ANY SEER
    // citation, constraining no sentence text at all. Now the answer is pinned
    // to its own whole citations array.
    assert!(
        out.contains(&only_citation(SKELETAL)),
        "the skeletal answer carries the sentence about skeletal muscle: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
}

#[test]
fn muscle_nuclei_count_backward_recalls_cardiac_for_single_nucleus() {
    let dir = scratch("cardiac");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count($Muscle, single_nucleus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"muscle_nuclei_count(cardiac, single_nucleus)\""),
        "cardiac muscle should be the only recalled single-nucleus type: {out}"
    );
    assert!(
        !out.contains("muscle_nuclei_count(skeletal, single_nucleus)"),
        "skeletal muscle is multinucleated, not single-nucleus: {out}"
    );
    // THE WHOLE DEFECT, AS A NEEDLE. Before this change the cardiac answer
    // carried the SKELETAL sentence as its warrant.
    assert!(
        out.contains(&only_citation(CARDIAC)),
        "the cardiac answer carries the sentence about cardiac muscle: {out}"
    );
    assert!(
        !out.contains(SKELETAL),
        "the skeletal sentence must not warrant the cardiac answer: {out}"
    );
}

#[test]
fn muscle_nuclei_count_abstains_on_smooth_muscle() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count(smooth, $Nuclei)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "smooth muscle is not part of tissue-types.adj's muscle citation -- honest abstention expected: {out}"
    );
    // An abstaining query emits no citations array at all.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        0,
        "an abstention carries no citation: {out}"
    );
}

#[test]
fn every_muscle_answer_carries_only_the_sentence_about_that_muscle() {
    for (muscle, nuclei, span) in scale() {
        let dir = scratch(&format!("only_{muscle}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"muscle-nuclei-count.adj\"\n? muscle_nuclei_count({muscle}, $Nuclei)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {muscle}: {out}"
        );
        assert!(
            out.contains(&format!("\"Nuclei\":\"{nuclei}\"")),
            "{muscle} -> {nuclei}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{muscle}: the SEER sentence about it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS. The two spans share only "muscle" and "fibers", so no
        // short needle is exclusive to either row.
        for other in [SKELETAL, CARDIAC] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "the other muscle's sentence must not reach {muscle}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {muscle}: {out}"
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
    // THIS ARM WOULD HAVE FAILED ON THE PRE-CONVERSION FILE, which shipped the
    // cardiac sentence as a table-level `cites` with its own redundant
    // `locator`. Keyword-anchored, so a `cites` at any indent fails it.
    //
    // SCOPE: this table's rows differ only in `source`. A row-level `cites` is
    // legal ADJ -- 18 shipped tables use one -- so this pins a convention local
    // to this table, not a language rule.
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
        "the envelope is the page's framing sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{SKELETAL}\"\n    locator")),
        "not the skeletal span as the envelope again"
    );
    // THE ENVELOPE MUST NAME NO MUSCLE TYPE, and the list is the page's whole
    // taxonomy rather than just the two row keys.
    //
    // Forbidding only the row keys leaves the frame free to be about SMOOTH
    // muscle -- a real third type this page describes, which is deliberately no
    // row here and for which the page states an actual nuclei count. In shard
    // 03590 exactly this shape survived a whole suite: a frame that named no
    // row key but framed a different subject entirely.
    //
    // A LOCAL RULE, NOT A GENERAL ONE -- and the distinction matters, because
    // #15344 (`blood-cell-types`, two conversions ago in this batch) settled
    // the general principle the other way: "the envelope may ENUMERATE the
    // keys, and may not STATE any of the per-row facts the table asserts."
    // That table's shape test REQUIRES every row key in its envelope.
    //
    // This table's envelope is DEFINITIONAL rather than enumerating -- it says
    // what muscle tissue is, naming no type -- so forbidding the taxonomy is
    // the right pin HERE. It would be wrong for an enumerating frame: this very
    // page's `tissue-types.adj` ships "Muscle tissue can be categorized into
    // skeletal muscle tissue, smooth muscle tissue, and cardiac muscle
    // tissue." -- a genuine framing sentence that names the set BY LISTING ITS
    // MEMBERS, states no nuclei count, and this arm would reject it.
    //
    // So the claim is: for a definitional frame, naming a member is a defect.
    // An enumerating frame needs the #15344 shape instead.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["skeletal", "cardiac", "smooth"] {
        assert!(
            !tokens.contains(&word),
            "the envelope must name no muscle type, but contains {word:?} as a \
             whole word: {shipped_envelope:?}"
        );
    }
}

#[test]
fn every_row_nuclei_count_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check (#15318).
    let body = shipped_table();
    for (muscle, nuclei, span) in scale() {
        // ANCHORED ON THE ROW HEADER, not the bare `source` line: the weaker
        // needle proves a span sits among the eight-space source lines
        // SOMEWHERE, not that it belongs to THIS row. On `mixture-types` a
        // mutant swapping two rows' spans satisfied the weaker needle fully.
        assert!(
            body.contains(&format!(
                "row ({muscle}, {nuclei}) {{\n        source \"{span}\"\n"
            )),
            "{muscle} carries its own span IN ITS OWN ROW: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        assert!(
            tokens.contains(&muscle),
            "{muscle}: its own span must name the muscle type as a whole word, \
             but it is not a token of {normalized:?}"
        );
        // And it must state the content the atom compresses. EXHAUSTIVE, no
        // catch-all: a `_` arm would silently apply one row's needle to any row
        // added later. Note `single_nucleus` is a COMPRESSION of "one nucleus
        // per cell" -- the page never uses the atom's own spelling, so the
        // needle is the page's phrase, not the atom.
        let content: &[&str] = match muscle {
            "skeletal" => &["multinucleated"],
            "cardiac" => &["one nucleus per cell"],
            other => panic!("no content needle registered for row {other}"),
        };
        for needle in content {
            assert!(
                normalized.contains(*needle),
                "{muscle}: its span must state {needle:?}, the content the atom \
                 {nuclei:?} compresses, but it is not in {normalized:?}"
            );
        }
    }
}
