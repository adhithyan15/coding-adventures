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
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"M\":\"proportional\""), "maine is proportional: {out}");
    assert!(
        out.contains("Maine and Nebraska assign their electors using a proportional system."),
        "carries the exception sentence verbatim: {out}"
    );
    assert!(
        out.contains("usa.gov/electoral-college") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
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
    assert!(
        out.contains("In 48 states and Washington, D.C., the winner gets all the electoral votes"),
        "carries the rule sentence verbatim: {out}"
    );
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
