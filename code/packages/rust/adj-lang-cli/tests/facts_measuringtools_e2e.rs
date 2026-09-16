//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/measuring-tools.adj`) driven through the
//! built CLI: a native `table` naming which quantity each of four common
//! lab tools measures, quoted verbatim from a Chemistry LibreTexts intro
//! lab manual -- a genuinely new "observation and measurement" axis (ADJ-
//! STDLIB-COVERAGE.md 5.1's named Major Gap for K-8 science), distinct from
//! the sibling `lab-equipment.adj`'s tool->purpose-verb table. Deliberately
//! NOT a 5th ordinal-bridge instance -- the science lane's four prior slices
//! (season/planet/moon-phase/mitosis) already saturate that pattern. 0
//! answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the RULER sentence, so a graduated_cylinder, balance or thermometer answer
//! was warranted primarily by a sentence about a ruler.
//!
//! THE BALANCE ROW QUOTES A PAGE TYPO ON PURPOSE (#15320). The page writes
//! "will be to measure mass in grams (g)" -- the word "used" is missing. The
//! library header used to repair that silently and mark it with brackets; a
//! repaired citation is a paraphrase wearing quotation marks, and the repaired
//! form occurs zero times on the page. Same discipline as cloud-types, which
//! reproduces its page's singular slip rather than correcting it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_measuringtools_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("chemistry/measuring-tools.adj");
    std::fs::copy(&src, dir.join("measuring-tools.adj"))
        .expect("copy shipped measuring-tools.adj");
}

#[test]
fn measuring_tool_recall_binds_the_quantity_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool(thermometer, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Q\":\"temperature\""),
        "a thermometer measures temperature: {out}"
    );
    // This was `contains("chem.libretexts.org") && contains("\"trust\":\"consensus\"")`,
    // which any LibreTexts citation satisfies and which constrains no sentence
    // text at all. The needle is now the whole citations array as the serialiser
    // emits it, closing on both the corroborations `]` and the citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(THERMOMETER)),
        "the thermometer answer carries the Part D sentence, whole: {out}"
    );
    assert!(!out.contains(RULER), "the ruler sentence reaches no thermometer answer: {out}");
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
}

const LOCATOR: &str = "https://chem.libretexts.org/Courses/BethuneCookman_University/B-CU:_CHL-141_General_Chemistry_1_Lab/Labs/1:_Introducing_Measurements_in_the_Laboratory_(Experiment)";
const ENVELOPE: &str = "In this lab, students will be introduced to some common measuring instruments so that they can practice making measurements, and to learn about instrument precision.";
const RULER: &str = "In Part A of this lab, a metric ruler will be used to measure length in centimeters (cm).";
const CYLINDER: &str = "In Part B, a beaker and a graduated cylinder will be used to measure liquid volume in milliliters (mL).";
/// The page's OWN wording: it is missing the word "used" (#15320). Quoting the
/// grammatical repair would be quoting a sentence the page does not contain.
const BALANCE: &str = "In Part C, an electronic balance and a triple-beam balance will be to measure mass in grams (g).";
const BALANCE_REPAIRED: &str = "In Part C, an electronic balance and a triple-beam balance will be used to measure mass in grams (g).";
const THERMOMETER: &str = "In Part D, a thermometer will be used to measure temperature in degrees Celsius (\u{b0}C).";

/// (tool, the quantity it measures, the lab-part sentence naming both)
fn tools() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("ruler", "length", RULER),
        ("graduated_cylinder", "volume", CYLINDER),
        ("balance", "mass", BALANCE),
        ("thermometer", "temperature", THERMOMETER),
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
    let adj = std::fs::read_to_string(facts_stdlib().join("chemistry/measuring-tools.adj"))
        .expect("read shipped measuring-tools.adj");
    adj[adj.find("table measuring_tool").expect("table")..].to_string()
}

#[test]
fn every_tool_answer_carries_only_the_sentence_that_names_that_tool() {
    for (tool, quantity, span) in tools() {
        let dir = scratch(&format!("tool_{tool}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"measuring-tools.adj\"\n? measuring_tool({tool}, $Q)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {tool}: {out}");
        assert!(out.contains(&format!("\"Q\":\"{quantity}\"")), "{tool} -> {quantity}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{tool}: its own lab-part sentence, whole, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(tool.split('_').next_back().unwrap()),
            "the span carried by {tool} names {tool} -- the defect was that it did not"
        );
        for other in [RULER, CYLINDER, BALANCE, THERMOMETER] {
            if other != span {
                assert!(!out.contains(other), "another tool's sentence must not reach {tool}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {tool}: {out}");
    }
}

#[test]
fn the_balance_row_quotes_the_page_typo_rather_than_repairing_it() {
    // #15320: the page writes "will be to measure mass" -- "used" is missing.
    // The header used to quote a bracketed repair under a heading reading "The
    // exact sentences"; measured, both the bracketed form and the plain-"used"
    // form occur ZERO times on the page. A citation that fixes its page is a
    // citation that matches no page.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{BALANCE}\"\n")),
        "the balance row quotes the page's own wording: {body}"
    );
    assert!(
        !body.contains(BALANCE_REPAIRED),
        "the grammatical repair must not be quoted as the page's words"
    );
    let whole = std::fs::read_to_string(facts_stdlib().join("chemistry/measuring-tools.adj"))
        .expect("read shipped measuring-tools.adj");
    assert!(!whole.contains("[used]"), "the bracketed repair is gone from the library");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the trust tier.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 4, "four row sources");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n"
        )),
        "the envelope is the page's framing sentence, at the consensus tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{RULER}\"\n    locator")),
        "not the ruler sentence as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in [
        "ruler", "cylinder", "balance", "thermometer", "beaker", "length", "volume", "mass",
        "temperature",
    ] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no tool and no quantity, but contains {word:?}"
        );
    }
}

#[test]
fn measuring_tool_reverse_binds_the_tool_for_that_quantity() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool($T, volume)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"graduated_cylinder\""),
        "a graduated cylinder measures volume: {out}"
    );
}

#[test]
fn measuring_tool_abstains_honestly_on_an_unshipped_tool() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool(microscope, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a microscope has no shipped row -- honest abstention, never invented: {out}"
    );
}
