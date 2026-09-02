//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/government-branch-member.adj`) driven through
//! the built CLI: a native `table` naming which officer or institution
//! makes up each branch of the U.S. federal government, grounding USA.gov's
//! "Branches of the U.S. government" page. This is the FIRST library in the
//! new `civics/` domain — the first entry against the "Social knowledge →
//! Civics" Major Gap in ADJ-STDLIB-COVERAGE.md §5.1. Runs the relation
//! BACKWARD as a genuine reverse recall (member -> branch, the direction a
//! learner is actually quizzed in), exercises it as a MULTI-VALUED relation
//! (the executive branch has three named members), and abstains honestly on
//! `senate` (an institution the SAME source names, but nests UNDER Congress
//! as a chamber rather than as a direct member of the branch) and on
//! `executive_departments` (an open-ended category phrase naming no
//! specific body). 0 answer-time model calls.

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
        "adjcli_factsgovbranchmember_{tag}_{}",
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
    let src = facts_stdlib().join("civics/government-branch-member.adj");
    std::fs::copy(&src, dir.join("government-branch-member.adj"))
        .expect("copy shipped government-branch-member.adj");
}

#[test]
fn government_branch_member_recall_binds_member_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"government-branch-member.adj\"\n\
         ? government_branch_member(legislative, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The legislative branch is made up of Congress.
    assert!(out.contains("\"M\":\"congress\""), "legislative -> congress: {out}");
    // The answer carries the USA.gov citation as proof.
    assert!(
        out.contains("usa.gov/branches-of-government")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn government_branch_member_reverse_binds_branch_from_member() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"government-branch-member.adj\"\n\
         ? government_branch_member($B, supreme_court)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The relation runs BACKWARD: "which branch is the Supreme Court part
    // of?" is the direction an elementary civics learner is quizzed in.
    assert!(
        out.contains("government_branch_member(judicial, supreme_court)"),
        "the Supreme Court is in the judicial branch: {out}"
    );
}

#[test]
fn government_branch_member_is_multi_valued_on_the_executive_branch() {
    let dir = scratch("multi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"government-branch-member.adj\"\n\
         ? government_branch_member(executive, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // One cited sentence names all three executive members, so the relation
    // yields three solutions rather than one.
    for member in ["president", "vice_president", "president_cabinet"] {
        assert!(
            out.contains(&format!("\"M\":\"{member}\"")),
            "executive branch includes {member}: {out}"
        );
    }
}

#[test]
fn government_branch_member_abstains_honestly_on_chambers_and_categories() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"government-branch-member.adj\"
         ? government_branch_member($B, senate)
         ? government_branch_member($B, executive_departments)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // `senate` IS named by the same source, but nested UNDER Congress in a
    // colon-introduced sub-list -- a CHAMBER of Congress, not a direct
    // member of the branch. Flattening it into a row here would silently
    // discard the nesting the source actually states, so this table
    // abstains and leaves it to a future `congress_chamber` table.
    // `executive_departments` is an open-ended category phrase naming no
    // specific body -- there is nothing to bind a stable atom to.
    // Both are asked in VARIABLE form ("which branch is X part of?"), the
    // direction that produces a real recall entry to abstain on.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "both the chamber and the category phrase abstain honestly: {out}"
    );
}
