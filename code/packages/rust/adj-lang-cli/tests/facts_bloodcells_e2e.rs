//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/blood-cell-types.adj`) driven through the built
//! CLI: a native `table` of blood-cell-type → main-function resolves a
//! binding-query recall with the source's citation, runs the relation backward
//! (function → cell type), and abstains on a cell that is not a blood cell —
//! 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the `red_blood_cells` span, so asking what platelets do returned `clotting`
//! warranted primarily by a sentence about red cells. The other two spans sat
//! in the header comment block, reaching no answer.
//!
//! THE ENVELOPE IS AN ENUMERATING FRAME, and that is an established shape here
//! rather than one invented for this table: `biology/rainforest-layer.adj` (4
//! rows), `earth-science/atmosphere-layers.adj` (5) and `astronomy/planets.adj`
//! (8) each ship an envelope naming every row key. The rule they follow is not
//! "name no row key" but: the envelope may ENUMERATE the keys, and may not
//! STATE any of the per-row facts the table asserts. This table maps cell type
//! to FUNCTION, and the envelope states no function at all.
//!
//! NEGATIVE ARMS NAME WHOLE SPANS, because no single word is exclusive to one
//! row. Measured over the shipped spans: all three share "blood", and the two
//! cell rows also share "cells". No span is a substring of another, but a bare
//! needle is exclusive to nothing — the same hazard `plant-parts` has with
//! "roots" and `mixture-types` with "salt"/"water".

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
        "adjcli_factsbloodcells_{tag}_{}",
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
    std::fs::copy(
        facts_stdlib().join("biology/blood-cell-types.adj"),
        dir.join("blood-cell-types.adj"),
    )
    .expect("copy shipped blood-cell-types.adj");
}

const LOCATOR: &str = "https://medlineplus.gov/blood.html";

/// The page's sentence naming the three solid components together. It
/// ENUMERATES every row key and states no function, so it warrants no row's
/// answer by itself.
const ENVELOPE: &str =
    "The solid part of your blood contains red blood cells, white blood cells, and platelets.";

const RBC: &str =
    "Red blood cells (RBC) deliver oxygen from your lungs to your tissues and organs.";
const WBC: &str =
    "White blood cells (WBC) fight infection and are part of your immune system.";
const PLT: &str = "Platelets help blood to clot when you have a cut or wound.";

/// (cell type, its function atom, the MedlinePlus sentence stating that function)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("red_blood_cells", "carry_oxygen", RBC),
        ("white_blood_cells", "fight_infection", WBC),
        ("platelets", "clotting", PLT),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
///
/// The empty `corroborations` is meaningful here: this table never carried a
/// `cites`, so the arm pins that the conversion introduced none.
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/blood-cell-types.adj"))
        .expect("read shipped blood-cell-types.adj");
    adj[adj.find("table blood_cell_function").expect("table")..].to_string()
}

#[test]
fn biology_blood_cells_recall_binds_function_with_citation() {
    let dir = scratch("bloodcells");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"blood-cell-types.adj\"\n\
         ? blood_cell_function(red_blood_cells, $F)\n\
         ? blood_cell_function(platelets, $F)\n\
         ? blood_cell_function($C, fight_infection)\n\
         ? blood_cell_function(neuron, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // THREE, not four. This case asks FOUR queries -- red_blood_cells,
    // platelets, the backward `fight_infection` bind, and `neuron` -- but
    // `neuron` ABSTAINS, and an abstaining query produces no citations array.
    // Read off the query block above, not counted in my head.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        3,
        "three citations arrays -- one per ANSWERING query; neuron abstains: {out}"
    );

    // Red blood cells carry oxygen; platelets do the clotting — the recalled
    // main-function atoms.
    assert!(
        out.contains("\"F\":\"carry_oxygen\""),
        "red_blood_cells → carry_oxygen: {out}"
    );
    assert!(
        out.contains("\"F\":\"clotting\""),
        "platelets → clotting: {out}"
    );
    // The relation runs backward: the job fight_infection recalls white_blood_cells.
    assert!(
        out.contains("\"C\":\"white_blood_cells\""),
        "fight_infection → white_blood_cells (reverse recall): {out}"
    );

    // THIS PIN WAS `contains("medlineplus.gov/blood.html") &&
    // contains("\"trust\":\"authoritative\"")` -- satisfied by ANY MedlinePlus
    // citation, constraining no sentence text, and before the conversion
    // satisfied by the red-cell span riding on every answer, including the
    // platelet one this same test binds. Now each answer is pinned to its own
    // whole citations array, as SEPARATE assertions rather than one `&&`:
    // joined, a failure cannot say which arm broke.
    assert!(
        out.contains(&only_citation(RBC)),
        "the red-cell answer carries the sentence about red cells: {out}"
    );
    assert!(
        out.contains(&only_citation(PLT)),
        "the platelet answer carries the sentence about platelets: {out}"
    );
    assert!(
        out.contains(&only_citation(WBC)),
        "the reverse answer carries the white-cell sentence: {out}"
    );

    // The envelope's wording is primary for no answer.
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
    // The whole defect, stated as a needle: before the conversion the red-cell
    // span appeared on ALL THREE answers above.
    assert!(
        !out.contains(&format!("\"F\":\"clotting\"}},\"citations\":[{{\"source\":\"{RBC}\"")),
        "the red-cell sentence must not warrant the platelet answer: {out}"
    );

    // A neuron is a nerve cell, not a blood cell — honest abstention, never a
    // fabricated function.
    assert!(
        out.contains("\"abstained\":true"),
        "non-blood-cell abstains: {out}"
    );
}

#[test]
fn every_cell_answer_carries_only_the_sentence_about_that_cell() {
    for (cell, function, span) in scale() {
        let dir = scratch(&format!("cell_{cell}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"blood-cell-types.adj\"\n? blood_cell_function({cell}, $F)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {cell}: {out}"
        );
        assert!(
            out.contains(&format!("\"F\":\"{function}\"")),
            "{cell} -> {function}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{cell}: the MedlinePlus sentence about it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS only. All three spans share "blood" and the two cell rows
        // also share "cells", so a bare needle is exclusive to no row.
        for other in [RBC, WBC, PLT] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another cell's sentence must not reach {cell}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {cell}: {out}"
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
        3,
        "three row sources"
    );
    // Keyword-anchored, so a `cites` at any indent fails without
    // false-positiving on the word in header prose. SCOPE: this table's rows
    // differ only in `source`. A row-level `cites` is legal ADJ -- 18 shipped
    // tables use one -- so this pins a convention local to this table, not a
    // language rule.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "this table ships no corroboration at any indent: {body}"
    );
    // Indent-independent, and local to this table for the same reason: a row
    // may legally override `locator` or `trust`, and shipped tables do.
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
        "the envelope is the page's three-component sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{RBC}\"\n    locator")),
        "not the red-cell span as the envelope again"
    );
    // THE ENUMERATING-FRAME RULE, as an assertion. The envelope may name the
    // row keys -- it does, all three -- but must state none of the FUNCTIONS
    // the table asserts. Whole words, because "clot" is a substring of
    // "clotting" and "immune" of "immunity".
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in [
        "oxygen", "deliver", "infection", "fight", "immune", "clot", "clotting", "wound",
    ] {
        assert!(
            !tokens.contains(&word),
            "the envelope must state no per-row FUNCTION, but contains {word:?} \
             as a whole word: {shipped_envelope:?}"
        );
    }
    // THE POSITIVE HALF OF THE SAME RULE, and it was missing. The loop above
    // pins only that the envelope states no function -- which ANY sentence
    // stating no function satisfies, including one that frames the wrong
    // subject entirely. A mutant replacing the envelope with the page's
    // "Over half of your blood is plasma." (real page text, zero functions,
    // and zero row keys) SURVIVED the whole suite until this arm existed.
    //
    // An enumerating frame must actually enumerate: it defends the table as a
    // whole by naming what the rows ARE. Note the precedent tables --
    // rainforest-layer, atmosphere-layers, planets -- do not pin this either;
    // this arm is an improvement on them, not a convention copied from them.
    // THE CONTIGUOUS PHRASE, not scattered tokens. A per-underscore-part check
    // is satisfied by "white", "blood" and "cells" appearing anywhere in the
    // sentence, so an envelope carrying the right words in an unrelated
    // arrangement would pass while naming nothing. Compare the phrase.
    let phrase_haystack = tokens.join(" ");
    for (cell, _, _) in scale() {
        let phrase = cell.replace('_', " ");
        assert!(
            phrase_haystack.contains(&phrase),
            "the envelope must ENUMERATE every row key as a contiguous phrase, \
             but {phrase:?} is not in {phrase_haystack:?}"
        );
    }
}

#[test]
fn every_row_function_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs. This is what `element-categories` (#15318) failed.
    let body = shipped_table();
    for (cell, function, span) in scale() {
        // ANCHORED ON THE ROW HEADER, not on the bare `source` line. Matching
        // only `        source "<span>"` proves the span sits among the
        // eight-space source lines SOMEWHERE -- not that it belongs to THIS
        // row. On `mixture-types` a mutant swapping two rows' spans satisfied
        // the weaker needle completely.
        assert!(
            body.contains(&format!(
                "row ({cell}, {function}) {{\n        source \"{span}\"\n"
            )),
            "{cell} carries its own span IN ITS OWN ROW: {body}"
        );
        // The token checks below read `span`, a const in THIS FILE, so on their
        // own they cannot fail for any `.adj` edit. The assertion above is what
        // ties the literal to the row it describes; the chain is the pin.
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        // The span must NAME its cell type as whole words.
        for part in cell.split('_') {
            assert!(
                tokens.contains(&part),
                "{cell}: its own span must name {part:?} as a whole word, but it is \
                 not a token of {normalized:?}"
            );
        }
        // And it must state the CONTENT its atom compresses. EXHAUSTIVE, no
        // catch-all: a `_` arm would silently apply one row's needle to any row
        // added later.
        let content: &[&str] = match cell {
            "red_blood_cells" => &["oxygen"],
            "white_blood_cells" => &["infection"],
            "platelets" => &["clot"],
            other => panic!("no content needle registered for row {other}"),
        };
        for needle in content {
            assert!(
                normalized.contains(*needle),
                "{cell}: its span must state {needle:?}, the content the atom \
                 {function:?} compresses, but it is not in {normalized:?}"
            );
        }
    }
}
