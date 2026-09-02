//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/checks-and-balances.adj`) driven through the
//! built CLI: a THREE-column native `table` naming which act each part of
//! the U.S. federal government can take against the others, grounding
//! USA.gov's "Branches of the U.S. government" page. The third library in
//! the `civics/` domain and the first that is genuinely RELATIONAL rather
//! than a lookup -- its rows are "actor A can do action B to object C", so
//! EVERY column can be the bound one. Exercises all three directions
//! (bind the actor, bind the action, bind the object), asserts all five
//! grounding sentences ride along as source + corroborations, checks that
//! all three branches are represented (the point of the object-typed third
//! column), and abstains honestly on a passive restatement and on a real
//! power that is a branch's own key role rather than a check on another
//! branch. 0 answer-time model calls.

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
        "adjcli_factschecksbalances_{tag}_{}",
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
    let src = facts_stdlib().join("civics/checks-and-balances.adj");
    std::fs::copy(&src, dir.join("checks-and-balances.adj"))
        .expect("copy shipped checks-and-balances.adj");
}

#[test]
fn checks_and_balances_binds_both_presidential_checks_with_citations() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"checks-and-balances.adj\"\n\
         ? checks_and_balances(president, $A, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle in this file
    // matched only part of the sentence, so the citation could be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once.
    //
    // Several tests load this library, because siblings import it as a
    // dependency. The pin belongs in its OWN test: the others are not
    // responsible for its provenance. That is also why the assertion has
    // to be unique -- where a co-loaded sibling carries a byte-identical
    // citation, an assertion either one satisfies pins neither.
    // See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"The president can veto legislation created by Congress.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Both presidential checks the source states, each from its own sentence.
    assert!(out.contains("\"A\":\"veto\""), "president can veto: {out}");
    assert!(
        out.contains("\"O\":\"legislation_created_by_congress\""),
        "veto applies to legislation created by Congress: {out}"
    );
    assert!(out.contains("\"A\":\"nominate\""), "president nominates: {out}");
    // The answer carries the USA.gov citation as proof.
    assert!(
        out.contains("usa.gov/branches-of-government")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn checks_and_balances_carries_every_grounding_sentence() {
    let dir = scratch("cites");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"checks-and-balances.adj\"\n\
         ? checks_and_balances(president, veto, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Five rows come from five different sentences: one `source` plus four
    // `cites`, which ride along as corroborations. Every row must remain
    // auditable back to the sentence that grounds it.
    for sentence in [
        "The president can veto legislation created by Congress.",
        "He or she also nominates heads of federal agencies and high court appointees.",
        "Congress confirms or rejects the president",
        "It can also remove the president from office in exceptional circumstances.",
        "The Justices of the Supreme Court can overturn unconstitutional laws.",
    ] {
        assert!(
            out.contains(sentence),
            "grounding sentence carried: {sentence}: {out}"
        );
    }
}

#[test]
fn checks_and_balances_runs_backward_on_the_action_and_on_the_object() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"checks-and-balances.adj\"\n\
         ? checks_and_balances($A, veto, $O)\n\
         ? checks_and_balances($A, $Act, president)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Ternary relations run backward on ANY column. "Who can veto?" binds
    // the actor from the action...
    assert!(
        out.contains("checks_and_balances(president, veto, legislation_created_by_congress)"),
        "who can veto -> the president: {out}"
    );
    // ...and "who acts on the president?" binds actor AND action from the
    // object, a question neither sibling civics library can answer.
    assert!(
        out.contains("checks_and_balances(congress, remove_from_office, president)"),
        "who acts on the president -> Congress, by removal from office: {out}"
    );
}

#[test]
fn checks_and_balances_represents_all_three_branches() {
    let dir = scratch("branches");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"checks-and-balances.adj\"\n\
         ? checks_and_balances($A, $Act, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // This is the whole point of typing the third column as the OBJECT
    // acted upon rather than the BRANCH acted upon. The Supreme Court's
    // own sentence names LAWS, not a branch -- a branch-typed target would
    // have forced abstaining on it and leaving the judicial branch absent
    // from a table about the balance BETWEEN the three branches.
    for actor in ["president", "congress", "supreme_court"] {
        assert!(
            out.contains(&format!("\"A\":\"{actor}\"")),
            "all three branches act in this table, including {actor}: {out}"
        );
    }
}

#[test]
fn checks_and_balances_abstains_on_passive_restatements_and_own_key_roles() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"checks-and-balances.adj\"\n\
         ? checks_and_balances(justices, $Act, $O)\n\
         ? checks_and_balances(congress, declare_war, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // `justices` as an ACTOR would double-count: "These justices are
    // nominated by the president and confirmed by the Senate" is a PASSIVE
    // restatement of two checks already held in the active voice.
    // `declare_war` is a real power the SAME page names, but in the
    // legislative branch's own "key roles" list -- what Congress does in
    // its own right, not a check it applies to another branch.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "both the passive restatement and the own-key-role abstain: {out}"
    );
}
