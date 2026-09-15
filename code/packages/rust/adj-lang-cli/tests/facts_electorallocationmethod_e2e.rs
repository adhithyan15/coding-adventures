//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/elector-allocation-method.adj`) driven through
//! the built CLI: a native `table` recording how a jurisdiction assigns its
//! presidential electors, grounding USA.gov's "Electoral College" page.
//!
//! The TENTH library in the `civics/` domain, and it CLOSES A DOCUMENTED
//! ABSTENTION: `electoral-college-count.adj` holds `winner_take_all_states
//! -> 48` and deliberately declined the Maine/Nebraska fact, its header
//! recording that a proportional system is "a METHOD, not a count ...
//! Different axis, its own future table". This is that table, and there is
//! a test below importing both to show they compose rather than overlap.
//!
//! The abstention worth reading is `california`. The source describes the
//! other 48 only as a GROUP, so binding a specific state name would require
//! deciding it is one of the 48 -- an inference the page does not license
//! for any particular state. A model asked "how does California award its
//! electors?" answers confidently; this table declines, which is the whole
//! point. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the 48-states sentence, the primary source of all three answers, so Maine's
//! answer was cited to the sentence about the OTHER 48 states, and this test
//! pinned exactly that. The Maine and Nebraska rows now carry the sentence that
//! names them; the envelope is a framing sentence from the page, which every
//! row overrides. Each answer's citations array is pinned whole.

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
        "adjcli_factselectoralloc_{tag}_{}",
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

fn place(dir: &Path, names: &[&str]) {
    for name in names {
        let src = facts_stdlib().join("civics").join(name);
        std::fs::copy(&src, dir.join(name))
            .unwrap_or_else(|e| panic!("copy shipped {name}: {e}"));
    }
}

const LOCATOR: &str = "https://www.usa.gov/electoral-college";
const ENVELOPE: &str = "The Electoral College decides who will be elected president and vice president of the U.S.";
const RULE: &str = "In 48 states and Washington, D.C., the winner gets all the electoral votes for that state.";
const MAINE_AND_NEBRASKA: &str = "Maine and Nebraska assign their electors using a proportional system.";

/// (jurisdiction, method, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("forty_eight_states_and_dc", "winner_take_all", RULE),
    ("maine", "proportional", MAINE_AND_NEBRASKA),
    ("nebraska", "proportional", MAINE_AND_NEBRASKA),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("civics/elector-allocation-method.adj"))
        .expect("read shipped elector-allocation-method.adj");
    adj[adj.find("table elector_allocation_method").expect("table")..].to_string()
}

#[test]
fn elector_allocation_method_binds_maines_method_with_citation() {
    let dir = scratch("direct");
    place(&dir, &["elector-allocation-method.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elector-allocation-method.adj\"\n\
         ? elector_allocation_method(maine, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATIONS ARRAY, not a fragment (#13916, #13918). This used to
    // pin the 48-STATES sentence as the source of Maine's answer: the envelope
    // was primary for every row (#14986). Maine's answer now carries the
    // sentence that names Maine, and nothing else.
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"M\":\"proportional\""), "maine is proportional: {out}");
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(MAINE_AND_NEBRASKA)),
        "the Maine and Nebraska sentence is the only citation, whole: {out}"
    );
    assert!(!out.contains(RULE), "the 48-states sentence no longer reaches Maine's answer: {out}");
}

#[test]
fn elector_allocation_method_reverse_returns_both_exception_states() {
    let dir = scratch("reverse");
    place(&dir, &["elector-allocation-method.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elector-allocation-method.adj\"\n\
         ? elector_allocation_method($J, proportional)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // "Which states are the exception?" is the direction a learner is
    // usually quizzed in, and one cited sentence names both.
    for state in ["maine", "nebraska"] {
        assert!(
            out.contains(&format!("\"J\":\"{state}\"")),
            "{state} assigns electors proportionally: {out}"
        );
    }
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers: {out}");
    assert_eq!(
        out.matches(&only_citation(MAINE_AND_NEBRASKA)).count(),
        2,
        "both exception states carry only the sentence that names them: {out}"
    );
}

#[test]
fn elector_allocation_method_keeps_the_rule_the_exception_is_an_exception_to() {
    let dir = scratch("rule");
    place(&dir, &["elector-allocation-method.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elector-allocation-method.adj\"\n\
         ? elector_allocation_method(forty_eight_states_and_dc, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Shipping the exception without the rule would leave a learner able to
    // recall that Maine is proportional without being able to recall that
    // almost nowhere else is.
    assert!(
        out.contains("\"M\":\"winner_take_all\""),
        "the 48 states and D.C. are winner-take-all: {out}"
    );
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(RULE)), "the rule sentence is the only citation, whole: {out}");
    assert!(!out.contains(MAINE_AND_NEBRASKA), "the exception sentence does not reach the rule's answer: {out}");
}

#[test]
fn elector_allocation_method_composes_with_the_electoral_college_counts() {
    let dir = scratch("compose");
    place(
        &dir,
        &["elector-allocation-method.adj", "electoral-college-count.adj"],
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"elector-allocation-method.adj\"\n\
         import \"electoral-college-count.adj\"\n\
         ? elector_allocation_method(forty_eight_states_and_dc, $M)\n\
         ? electoral_college_count(winner_take_all_states, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The two libraries hold different halves of the same sentence: the
    // count table says HOW MANY jurisdictions are winner-take-all, this one
    // says WHICH METHOD that group uses. They compose rather than overlap,
    // which is why the count table abstained on the method and pointed here.
    assert!(
        out.contains("\"M\":\"winner_take_all\""),
        "the method: winner-take-all: {out}"
    );
    assert!(out.contains("\"N\":\"48\""), "the count: 48 states: {out}");
}

#[test]
fn elector_allocation_method_abstains_on_unplaced_states_and_on_the_mechanism() {
    let dir = scratch("abstain");
    place(&dir, &["elector-allocation-method.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elector-allocation-method.adj\"\n\
         ? elector_allocation_method(california, $M)\n\
         ? elector_allocation_method($J, by_congressional_district)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The source describes the other 48 only as a GROUP. Binding a specific
    // state would require deciding it is one of them -- an inference the
    // page does not license for any particular state. And the page names
    // the proportional SYSTEM without explaining its mechanism, so the
    // well-known congressional-district detail is not available from this
    // source and must not be filled in from outside it.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains on the unplaced state and on the unexplained mechanism: {out}"
    );
}

#[test]
fn every_jurisdiction_answer_carries_its_own_sentence() {
    for (jurisdiction, method, sentence) in ROWS {
        let dir = scratch(&format!("row_{jurisdiction}"));
        place(&dir, &["elector-allocation-method.adj"]);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"elector-allocation-method.adj\"\n? elector_allocation_method({jurisdiction}, $M)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {jurisdiction}: {out}");
        assert!(out.contains(&format!("\"M\":\"{method}\"")), "{jurisdiction} uses {method}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{jurisdiction}: its own sentence, whole, and the only citation: {out}");
        for (_, _, other_sentence) in ROWS {
            if other_sentence != sentence {
                assert!(!out.contains(other_sentence), "another row's sentence must not reach {jurisdiction}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {jurisdiction}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (jurisdiction, method, sentence) in ROWS {
        let expected = format!("    row ({jurisdiction}, {method}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({jurisdiction}, {method}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites \""), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{RULE}\"\n    locator")), "not the 48-states sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["48", "maine", "nebraska", "winner", "proportional", "state"] {
        assert!(!folded.contains(word), "the envelope must name no jurisdiction or method, but contains {word:?}");
    }
}
