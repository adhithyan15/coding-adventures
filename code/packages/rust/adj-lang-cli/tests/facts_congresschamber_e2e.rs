//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/congress-chamber.adj`) driven through the
//! built CLI: a native `table` naming the two chambers the U.S. Congress
//! is divided into, grounding USA.gov's "Branches of the U.S. government"
//! page. This library exists to CLOSE the abstention its sibling
//! `government-branch-member.adj` deliberately opened -- that table
//! abstains on `senate`/`house_of_representatives` because the source
//! nests them UNDER Congress in a colon-introduced sub-list rather than
//! naming them as direct members of the branch, and names this table as
//! their intended home. Exercises the relation MULTI-VALUED on `parent`
//! (the "what are the two chambers of Congress?" direction a learner is
//! actually asked), and abstains honestly on `supreme_court` and
//! `president` -- named institutions the SAME source names, but of the
//! judicial and executive branches, not chambers of Congress.
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
        "adjcli_factscongresschamber_{tag}_{}",
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
    let src = facts_stdlib().join("civics/congress-chamber.adj");
    std::fs::copy(&src, dir.join("congress-chamber.adj"))
        .expect("copy shipped congress-chamber.adj");
}

#[test]
fn congress_chamber_recall_binds_parent_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"congress-chamber.adj\"\n\
         ? congress_chamber(senate, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"P\":\"congress\""), "senate -> congress: {out}");
    // The answer carries the USA.gov citation as proof.
    assert!(
        out.contains("usa.gov/branches-of-government")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn congress_chamber_enumerates_both_chambers_from_the_parent() {
    let dir = scratch("multi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"congress-chamber.adj\"\n\
         ? congress_chamber($C, congress)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // "What are the two chambers of Congress?" is ONE query returning TWO
    // solutions -- the source's own <ul> has exactly these two <li> items.
    for chamber in ["senate", "house_of_representatives"] {
        assert!(
            out.contains(&format!("\"C\":\"{chamber}\"")),
            "Congress includes the {chamber}: {out}"
        );
    }
}

#[test]
fn congress_chamber_closes_the_sibling_tables_abstention() {
    let dir = scratch("closes");
    place_lib(&dir);
    let src = facts_stdlib().join("civics/government-branch-member.adj");
    std::fs::copy(&src, dir.join("government-branch-member.adj"))
        .expect("copy shipped government-branch-member.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"congress-chamber.adj\"\n\
         import \"government-branch-member.adj\"\n\
         ? congress_chamber(senate, $P)\n\
         ? government_branch_member(legislative, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The two libraries import together and meet at the atom `congress`:
    // congress_chamber(senate, congress) + government_branch_member(
    // legislative, congress) are the two premises that ground "the Senate
    // is part of the legislative branch" as an auditable two-hop
    // derivation, rather than as a third asserted row. Both must bind the
    // SAME `congress` atom for that composition to be available.
    assert!(out.contains("\"P\":\"congress\""), "senate -> congress: {out}");
    assert!(
        out.contains("\"M\":\"congress\""),
        "legislative -> congress: {out}"
    );
}

#[test]
fn congress_chamber_abstains_honestly_on_other_branches_institutions() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"congress-chamber.adj\"\n\
         ? congress_chamber(supreme_court, $P)\n\
         ? congress_chamber(president, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Both are genuinely named institutions the SAME source names, but of
    // the judicial and executive branches -- not chambers of Congress.
    // They are already correctly tabled in government-branch-member.adj
    // against their own branches, so this table abstains rather than
    // inventing them into a chamber.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "both other-branch institutions abstain honestly: {out}"
    );
}
