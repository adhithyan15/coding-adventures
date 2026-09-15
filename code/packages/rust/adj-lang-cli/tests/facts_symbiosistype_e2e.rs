//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/symbiosis-type.adj`) driven through the built
//! CLI: a native `table` naming three types of symbiotic relationship and
//! what actually defines each, quoted verbatim from Wikipedia's
//! "Symbiosis" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the mutualism sentence, the primary source of all three answers; it is now
//! a framing sentence from the page, which every row overrides. Each answer's
//! citations array is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_symbiosis_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/symbiosis-type.adj");
    std::fs::copy(&src, dir.join("symbiosis-type.adj")).expect("copy shipped symbiosis-type.adj");
}

#[test]
fn symbiosis_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type(mutualism, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"both_parties_benefit\""),
        "mutualism means both_parties_benefit: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("en.wikipedia.org") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(MUTUALISM)), "the mutualism sentence is the only citation: {out}");
}

#[test]
fn symbiosis_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type($T, the_parasite_benefits_while_the_host_is_harmed)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"parasitism\""),
        "the shipped the_parasite_benefits_while_the_host_is_harmed example is parasitism: {out}"
    );
}

#[test]
fn symbiosis_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type(amensalism, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "amensalism is a real interaction category the source names, but its own sentence bundles it together with competition rather than stating one clean fact -- honest abstention, never invented: {out}"
    );
}

const SYMBIOSIS_TYPE_PIN: &str = r#""bindings":{"D":"both_parties_benefit"},"citations":[{"source":"Finally, where both parties benefit, the relationship is described as mutualistic.","locator":"https://en.wikipedia.org/wiki/Symbiosis","trust":"consensus""#;

#[test]
fn symbiosis_type_citation_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"
? symbiosis_type(mutualism, $D)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped value was a FRAGMENT PUNCTUATED INTO A SENTENCE: the page's
    // wording, with one character changed so it would read as standalone. It
    // therefore appeared on no page. Every quote-keyed screen passed it,
    // because the quotes were all correct.
    assert!(
        out.contains(SYMBIOSIS_TYPE_PIN),
        "the mutualism citation is the page's own sentence: {out}"
    );
}

const LOCATOR: &str = "https://en.wikipedia.org/wiki/Symbiosis";
const ENVELOPE: &str = "Symbiosis is diverse and can be classified in multiple ways.";
const MUTUALISM: &str = "Finally, where both parties benefit, the relationship is described as mutualistic.";
const COMMENSALISM: &str = "Commensalism describes a relationship between two living organisms where one benefits and the other is not significantly harmed or helped.";
const PARASITISM: &str = "In a parasitic relationship, the parasite benefits while the host is harmed.";

/// (type, description, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("mutualism", "both_parties_benefit", MUTUALISM),
    ("commensalism", "one_organism_benefits_and_the_other_is_not_significantly_harmed_or_helped", COMMENSALISM),
    ("parasitism", "the_parasite_benefits_while_the_host_is_harmed", PARASITISM),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/symbiosis-type.adj"))
        .expect("read shipped symbiosis-type.adj");
    adj[adj.find("table symbiosis_type").expect("table")..].to_string()
}

#[test]
fn every_type_answer_carries_its_own_sentence() {
    for (kind, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"symbiosis-type.adj\"\n? symbiosis_type($T, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"T\":\"{kind}\"")), "{desc} binds {kind}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{kind}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, desc, sentence) in ROWS {
        let expected = format!("    row ({kind}, {desc}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{MUTUALISM}\"\n    locator")), "not the mutualism sentence as the envelope again");
    assert!(!body.contains("harmed.[49]"), "the page's citation marker is not part of the parasitism quote");
    let folded = ENVELOPE.to_lowercase();
    for word in ["mutual", "commensal", "parasit", "benefit", "harm", "host"] {
        assert!(!folded.contains(word), "the envelope must name no type or description, but contains {word:?}");
    }
}
