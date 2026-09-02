//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/chamber-branch.adj`) driven through the built
//! CLI: the FIRST `rule` in the `civics/` domain, DERIVING which branch of
//! the U.S. federal government a named chamber of Congress belongs to.
//!
//! The point of this library is that the cited source never states the
//! conclusion. USA.gov's "Branches of the U.S. government" page never
//! writes "the Senate is part of the legislative branch" -- it writes two
//! separate facts one level apart ("The legislative branch is made up of
//! Congress:" and, nested under it, "The U.S. Senate"). So the answer is
//! COMPOSED from the two already-shipped sibling tables through the
//! `congress` atom they both bind, and every answer carries BOTH premises'
//! citations in its proof trail -- the provenance of the conclusion is the
//! composition of the provenance of its premises.
//!
//! This is also why `government-branch-member.adj` abstains on the
//! chambers and why `congress-chamber.adj` was split out as its own
//! predicate: the abstention was never a gap to paper over, it was the
//! seam this derivation joins on. Asserts the two-citation proof trail,
//! reverse enumeration of a branch's chambers, and INHERITED abstention
//! (the rule adds no facts, so it abstains exactly where its premises do).
//! 0 answer-time model calls.

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
        "adjcli_factschamberbranch_{tag}_{}",
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

/// The rule and BOTH of its premise libraries must travel together --
/// the derivation is meaningless without them.
fn place_libs(dir: &Path) {
    for name in [
        "chamber-branch.adj",
        "congress-chamber.adj",
        "government-branch-member.adj",
    ] {
        let src = facts_stdlib().join("civics").join(name);
        std::fs::copy(&src, dir.join(name))
            .unwrap_or_else(|e| panic!("copy shipped {name}: {e}"));
    }
}

#[test]
fn chamber_branch_derives_the_senates_branch_from_two_premises() {
    let dir = scratch("derive");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-branch.adj\"\n\
         ? chamber_branch(senate, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The conclusion no single source sentence states.
    assert!(out.contains("\"B\":\"legislative\""), "senate -> legislative: {out}");
    // It is DERIVED, not asserted: the trail must show a rule step.
    assert!(out.contains("\"kind\":\"rule\""), "answer is derived by a rule: {out}");
}

#[test]
fn chamber_branch_carries_both_premise_citations_in_its_proof_trail() {
    let dir = scratch("provenance");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-branch.adj\"\n\
         ? chamber_branch(house_of_representatives, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The provenance of the conclusion is the composition of the
    // provenance of its premises -- BOTH grounding sentences must ride
    // along, or the derived answer would not be auditable.
    //
    // THIS TEST USED TO BE UNABLE TO FAIL, in two separate ways, and both
    // are worth naming because they look like coverage:
    //
    //   1. FRAGMENT NEEDLES. It asserted bare spans of each sentence, with
    //      neither the `"source":"` key nor a closing quote. A citation can
    //      therefore be truncated AT the fragment -- deleting everything
    //      after it -- and the assertion still matches. See #13918.
    //   2. INDEPENDENT SCANS. It asserted the locator and
    //      `"trust":"authoritative"` as their own `contains` calls over the
    //      whole JSON blob. Two independent scans cannot tell WHICH ANSWER
    //      a citation belongs to. That is exactly the shape that let #13928
    //      ship, where a legislative answer carried executive-branch
    //      evidence while every assertion passed.
    //
    // A plain anchored pin is NOT available here. This library's `source` is
    // BYTE-IDENTICAL to `congress-chamber.adj`'s, because chamber_branch is
    // a RULE and its provenance IS the composed table's citation -- and this
    // test loads both. An assertion a co-loaded sibling can satisfy is not a
    // pin for this library. So the answer must be bound to its evidence.
    //
    // NEEDLE A: the RULE STEP -- its goal joined to the rule's OWN complete
    // citation. This is the only span in the output that belongs to THIS
    // library rather than to a premise.
    //
    // My first attempt pinned `"bindings":{"B":"legislative"},"citations":[`
    // plus the first citation object, reasoning that `congress_chamber` never
    // binds `B` so no sibling could satisfy it. That reasoning was wrong in a
    // way only a mutation could show: the needle IS unique to a chamber_branch
    // answer, but `citations[0]` is populated by the PREMISE's envelope, so
    // truncating THIS file's rule `source` left it passing while truncating
    // `congress-chamber.adj` broke it. It pinned the sibling's citation under
    // this library's name -- the exact defect it was written to prevent.
    //
    // The rule step cannot be confused that way: no sibling ever emits a step
    // whose `goal` is `chamber_branch(...)`, and the `source` on that step is
    // the rule's own envelope. Verified by mutation in both directions.
    assert!(
        out.contains(
            "\"kind\":\"rule\",\"step\":0,\"depth\":0,\"goal\":\"chamber_branch(house_of_\
             representatives, B)\",\"source\":\"The legislative branch is made up of \
             Congress:\",\"locator\":\"https://www.usa.gov/branches-of-government\",\"trust\":\
             \"authoritative\",\"corroborations\":[]"
        ),
        "the RULE's own goal is bound to the RULE's own whole citation: {out}"
    );

    // The answer itself carries a complete, unmangled citation. Note this
    // pins the PREMISE's envelope, not the rule's -- see above -- so it is a
    // check on the composed answer's shape, not on this library's provenance.
    assert!(
        out.contains(
            "\"bindings\":{\"B\":\"legislative\"},\"citations\":[{\"source\":\"The legislative \
             branch is made up of Congress:\",\"locator\":\"https://www.usa.gov/branches-of-\
             government\",\"trust\":\"authoritative\",\"corroborations\":[]}"
        ),
        "the derived answer is bound to a WHOLE premise citation: {out}"
    );

    // NEEDLE B: the two premises ADJACENT in one answer's citation array,
    // which is what "the proof trail carries both premises" actually means.
    //
    // Deliberately stops at the second sentence's closing quote rather than
    // pinning the whole 674-character answer span. The remainder is
    // `government-branch-member.adj`'s own corroborations, added in #13941 --
    // pinning those here would couple THIS library's test to a SIBLING's
    // citation list and break it on edits that have nothing to do with this
    // rule. A maximally strict pin is not automatically the right pin when
    // what it over-pins is not this library's responsibility.
    assert!(
        out.contains(
            "\"corroborations\":[]},{\"source\":\"The president, the vice president, and the \
             president's cabinet are the members of the executive branch.\""
        ),
        "both premise citations ride in the SAME answer, in order: {out}"
    );
}

#[test]
fn chamber_branch_runs_backward_to_enumerate_a_branchs_chambers() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-branch.adj\"\n\
         ? chamber_branch($C, legislative)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Derived relations run backward too: bind the branch, get its chambers.
    for chamber in ["senate", "house_of_representatives"] {
        assert!(
            out.contains(&format!("\"C\":\"{chamber}\"")),
            "the legislative branch's chambers include {chamber}: {out}"
        );
    }
}

#[test]
fn chamber_branch_inherits_its_premises_abstentions() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-branch.adj\"\n\
         ? chamber_branch(supreme_court, $B)\n\
         ? chamber_branch(president, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The rule adds NO facts of its own, so it abstains exactly where its
    // premises do. `congress_chamber` abstains on both the Supreme Court
    // and the president (named institutions of the other two branches, not
    // chambers of Congress), so the derivation finds no premise to join on
    // and abstains rather than inventing a chamber-of relationship.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstention is inherited from the premises, not restated: {out}"
    );
}
