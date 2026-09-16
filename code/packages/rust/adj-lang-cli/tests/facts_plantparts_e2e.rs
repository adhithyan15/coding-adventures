//! End-to-end test for the biology PLANT-PARTS facts library
//! (`adj-facts-stdlib/biology/plant-parts.adj`) driven through the built CLI:
//! a native `table` of plant-part → primary-function resolves binding-query
//! recalls (forward and backward) with the source's citation, and abstains on a
//! non-plant-part — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the roots span, so a stem, leaves or flower answer was warranted primarily
//! by a sentence about roots.
//!
//! THE STEM SPAN NAMES FOUR ROW KEYS -- "...transport water and nutrients
//! between roots, leaves, and flowers" -- so a bare part-name needle is
//! exclusive to no row. Every negative arm here names a WHOLE SPAN. Measured,
//! not assumed: roots names {roots}, stem names {roots, stems, leaves,
//! flowers}, leaves names {leaves}, flower names {flowers}.
//!
//! THE ENVELOPE'S APOSTROPHES ARE U+2019. The same sentence with ASCII
//! apostrophes occurs zero times on the page, so shipping one would cite a
//! sentence the source does not contain; `the_envelope_keeps_the_pages_own_apostrophes`
//! pins both directions.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn biology_plant_parts_recall_binds_function_with_citation() {
    let dir = scratch("plantparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/plant-parts.adj");
    std::fs::copy(&src, dir.join("plant-parts.adj")).expect("copy shipped plant-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-parts.adj\"\n\
         ? plant_part_function(roots, $Function)\n\
         ? plant_part_function($Part, photosynthesis)\n\
         ? plant_part_function(mushroom, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The roots absorb water — the recalled forward binding.
    assert!(
        out.contains("\"Function\":\"absorb_water\""),
        "roots → absorb_water: {out}"
    );
    // The relation runs BACKWARD too: what does photosynthesis? — the leaves.
    assert!(
        out.contains("\"Part\":\"leaves\""),
        "photosynthesis ← leaves (reverse recall): {out}"
    );
    // This was `contains("blogs.ifas.ufl.edu") && contains("\"trust\":\"authoritative\"")`,
    // which any UF/IFAS citation satisfies and which constrains no sentence
    // text. Before the RS-5e conversion it was satisfied by the ROOTS span
    // riding on every answer -- including the leaves one this same test binds.
    //
    // Separate assertions, not one `&&`: joined, a failure cannot say which arm
    // broke.
    // TWO, not three. This case asks THREE queries -- roots, the backward
    // `photosynthesis` bind, and `mushroom` -- but `mushroom` ABSTAINS, and an
    // abstaining query produces no citations array at all. I first wrote 3 by
    // counting queries in my head instead of reading the block four lines
    // above; the run said `left: 2, right: 3`.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        2,
        "two citations arrays -- one per ANSWERING query; mushroom abstains: {out}"
    );
    assert!(
        out.contains(&only_citation(ROOTS)),
        "the roots answer carries the sentence about roots, whole, and as its \
         only citation: {out}"
    );
    assert!(
        out.contains(&only_citation(LEAVES)),
        "the leaves answer carries the sentence about leaves, not the roots one: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
    // A mushroom is a fungus, not a plant part — honest abstention, never a
    // fabricated function.
    assert!(out.contains("\"abstained\":true"), "mushroom abstains: {out}");
}

const LOCATOR: &str = "https://blogs.ifas.ufl.edu/pascoco/2025/01/23/plant-biology-organs/";
/// The page's sentence framing the organs as a set. It names no part, and
/// carries the page's own U+2019 apostrophes in "plant's" -- the ASCII
/// spelling occurs ZERO times on the page, so shipping one would cite a
/// sentence the source does not contain.
const ENVELOPE: &str = "Each of the plant\u{2019}s organs play a vital role in the plant\u{2019}s survival.";
const ROOTS: &str = "Roots anchor a plant in the soil and absorb air, water, and nutrients.";
const STEM: &str = "Stems provide structural support and transport water and nutrients between roots, leaves, and flowers.";
const LEAVES: &str = "Leaves capture sunlight for photosynthesis.";
const FLOWER: &str = "Flowers are the reproductive structures of plants, classified as perfect (having both male and female parts) or imperfect (male or female).";

/// (part, its function atom, the UF/IFAS sentence stating that function)
///
/// CAUTION FOR NEGATIVE ARMS: the `stem` span names roots, leaves AND flowers
/// ("...between roots, leaves, and flowers"), so the bare word `roots` is not
/// exclusive to the roots span. Every negative arm therefore names a WHOLE
/// SPAN -- the same hazard `comet-part.adj` has with "nucleus".
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("roots", "absorb_water", ROOTS),
        ("stem", "support", STEM),
        ("leaves", "photosynthesis", LEAVES),
        ("flower", "reproduction", FLOWER),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/plant-parts.adj"))
        .expect("read shipped plant-parts.adj");
    adj[adj.find("table plant_part_function").expect("table")..].to_string()
}

#[test]
fn every_part_answer_carries_only_the_sentence_about_that_part() {
    for (part, function, span) in scale() {
        let dir = scratch(&format!("part_{part}"));
        let src = facts_stdlib().join("biology/plant-parts.adj");
        std::fs::copy(&src, dir.join("plant-parts.adj")).expect("copy shipped plant-parts.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"plant-parts.adj\"\n? plant_part_function({part}, $Function)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {part}: {out}"
        );
        assert!(
            out.contains(&format!("\"Function\":\"{function}\"")),
            "{part} -> {function}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{part}: the UF/IFAS sentence about it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS only. The stem span names roots, leaves and flowers, so a
        // bare part-name needle would not be exclusive to any one row.
        for other in [ROOTS, STEM, LEAVES, FLOWER] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another part's sentence must not reach {part}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {part}: {out}"
        );
    }
}

#[test]
fn the_envelope_keeps_the_pages_own_apostrophes() {
    // The page writes "plant\u{2019}s" with U+2019. The ASCII spelling occurs
    // ZERO times on the page, so shipping it would cite a sentence the source
    // does not contain -- the defect #15324 records for soil-texture-class, and
    // the one volcano-type and metamorphism-cause both shipped before their
    // conversions.
    // SCOPED TO `source "` LINES, not the whole table block. Read against the
    // block, this arm fails on a harmless in-table `%` comment that quotes the
    // envelope in ASCII -- and a comment is not a shipped citation, so that
    // would be a false alarm. What must be pinned is the string the table
    // SHIPS as provenance.
    let body = shipped_table();

    // TWO DIFFERENT SCOPES, because the two arms make different claims.
    //
    // The POSITIVE arm is about the TABLE-LEVEL envelope, so it reads only
    // four-space `source` lines. Scoped with `trim_start()` it would also see
    // the eight-space row sources, and would then be satisfied by a mutant that
    // deleted the envelope line and planted the string on a row -- leaving its
    // message ("the envelope ships with...") claiming more than it checked.
    let envelope_line: String = body
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        envelope_line.contains(ENVELOPE),
        "the table-level envelope ships with the page's own U+2019: {envelope_line}"
    );

    // The NEGATIVE arm is about ANY shipped citation, so it reads every
    // `source` line at any indent -- a row carrying an ASCII-apostrophe
    // envelope is just as wrong as the envelope line carrying one. It stays
    // scoped to `source` lines rather than the whole block, because a `%`
    // comment quoting the sentence in ASCII is not a shipped citation.
    let source_lines: String = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    let ascii_variant = ENVELOPE.replace('\u{2019}', "'");
    assert!(
        !source_lines.contains(&ascii_variant),
        "no `source` line at any indent may carry an ASCII-apostrophe envelope -- \
         that spelling occurs zero times on the page: {source_lines}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        4,
        "four row sources"
    );
    // Keyword-anchored, not indent-scoped: a row-level `cites` at eight spaces
    // must fail this too, without false-positiving on prose in a comment.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent"
    );
    // Indent-independent: the envelope has a `locator` and a `trust` of its
    // own, so the pin is EXACTLY one of each.
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
        "the envelope is the page's organ-framing sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{ROOTS}\"\n    locator")),
        "not the roots span as the envelope again"
    );
    // WHOLE WORDS, not substrings: "stem" is a substring of "system", "leaf" of
    // "leaflet". Compare tokens, as the solar-eclipse "partial"/"partially"
    // case required in reverse.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["roots", "root", "stem", "stems", "leaves", "leaf", "flower", "flowers"] {
        assert!(
            !tokens.contains(&word),
            "the shipped envelope must name no plant part, but contains {word:?} \
             as a whole word: {shipped_envelope}"
        );
    }
}

#[test]
fn every_row_function_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs. This is what `element-categories` (#15318) failed -- a row whose
    // span named no category at all.
    let body = shipped_table();
    for (part, function, span) in scale() {
        assert!(
            body.contains(&format!("        source \"{span}\"\n")),
            "{part} carries its own span: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        // The span must NAME its part as a whole word. `stem` is singular in the
        // key and plural on the page, so accept either -- stated per row rather
        // than guessed at with a stemmer.
        let naming: &[&str] = match part {
            "roots" => &["roots"],
            "stem" => &["stems", "stem"],
            "leaves" => &["leaves"],
            "flower" => &["flowers", "flower"],
            other => panic!("no naming form registered for row {other}"),
        };
        assert!(
            naming.iter().any(|n| tokens.contains(n)),
            "{part}: its own span must name the part as a whole word (one of {naming:?}), \
             but none is a token of {normalized:?}"
        );
        // EXHAUSTIVE, no catch-all: a `_` arm would silently apply one row's
        // needle to any row added later.
        let content: &[&str] = match part {
            "roots" => &["absorb"],
            "stem" => &["support"],
            "leaves" => &["photosynthesis"],
            "flower" => &["reproductive"],
            other => panic!("no content needle registered for row {other}"),
        };
        for needle in content {
            assert!(
                normalized.contains(*needle),
                "{part}: its span must state {needle:?}, the content the atom {function:?} \
                 compresses, but it is not in {normalized:?}"
            );
        }
    }
}
