//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/chamber-exclusive-power.adj`) driven through
//! the built CLI: a native `table` naming the lawmaking power each chamber
//! of Congress holds EXCLUSIVELY, grounding USA.gov's "How laws are made"
//! page.
//!
//! The FIFTH library in the `civics/` domain and the FIRST from a source
//! other than the "Branches of the U.S. government" page, which the four
//! earlier libraries between them fully decode. It deliberately reuses the
//! `house_of_representatives`/`senate` atoms `congress-chamber.adj`
//! already binds, so it composes with the shipped civics graph rather than
//! forking a parallel vocabulary -- there is a test below that chains an
//! exclusive power all the way to a branch through the derived
//! `chamber_branch` rule. Also abstains honestly on a body that is not a
//! chamber at all, and on the procedural contrast the same source section
//! states (a different axis from an exclusive power).
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
        "adjcli_factschamberexclpower_{tag}_{}",
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
fn chamber_exclusive_power_binds_each_chambers_power_with_citation() {
    let dir = scratch("direct");
    place(&dir, &["chamber-exclusive-power.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-exclusive-power.adj\"\n\
         ? chamber_exclusive_power(house_of_representatives, $P)\n\
         ? chamber_exclusive_power(senate, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"P\":\"initiate_tax_and_revenue_legislation\""),
        "only the House can initiate tax and revenue legislation: {out}"
    );
    assert!(
        out.contains("\"P\":\"draft_legislation_on_presidential_nominations_and_treaties\""),
        "only the Senate can draft nomination/treaty legislation: {out}"
    );
    // Both rows come from their own "Only the ..." sentence; both must ride
    // along, one as `source` and one as a `cites` corroboration.
    assert!(
        out.contains("Only the House can initiate tax and revenue-related legislation")
            && out.contains(
                "Only the Senate can draft legislation related to presidential nominations and treaties"
            ),
        "both grounding sentences carried: {out}"
    );
    assert!(
        out.contains("usa.gov/how-laws-are-made") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov 'How laws are made' citation: {out}"
    );
}

#[test]
fn chamber_exclusive_power_runs_backward_from_the_power_to_the_chamber() {
    let dir = scratch("reverse");
    place(&dir, &["chamber-exclusive-power.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-exclusive-power.adj\"\n\
         ? chamber_exclusive_power($C, initiate_tax_and_revenue_legislation)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // "Which chamber can initiate tax legislation?" is the direction a
    // learner is usually quizzed in.
    assert!(
        out.contains(
            "chamber_exclusive_power(house_of_representatives, initiate_tax_and_revenue_legislation)"
        ),
        "tax and revenue legislation originates in the House: {out}"
    );
}

#[test]
fn chamber_exclusive_power_composes_through_to_a_branch() {
    let dir = scratch("compose");
    place(
        &dir,
        &[
            "chamber-exclusive-power.adj",
            "chamber-branch.adj",
            "congress-chamber.adj",
            "government-branch-member.adj",
        ],
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-exclusive-power.adj\"\n\
         import \"chamber-branch.adj\"\n\
         ? chamber_exclusive_power($C, initiate_tax_and_revenue_legislation)\n\
         ? chamber_branch(house_of_representatives, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Reusing the sibling table's chamber atoms is what lets an exclusive
    // power chain all the way to a branch: the power names the chamber, and
    // the derived `chamber_branch` rule names the chamber's branch. A
    // forked vocabulary would silently break this.
    assert!(
        out.contains("\"C\":\"house_of_representatives\""),
        "power resolves to a chamber: {out}"
    );
    assert!(
        out.contains("\"B\":\"legislative\""),
        "that chamber resolves to the legislative branch: {out}"
    );
}

#[test]
fn chamber_exclusive_power_abstains_on_non_chambers_and_on_procedure() {
    let dir = scratch("abstain");
    place(&dir, &["chamber-exclusive-power.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chamber-exclusive-power.adj\"\n\
         ? chamber_exclusive_power(supreme_court, $P)\n\
         ? chamber_exclusive_power($C, majority_vote)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The Supreme Court is not a chamber of Congress at all. And the
    // majority-vote/deliberation contrast IS stated by the same source
    // section, but it describes a PROCEDURE each chamber uses rather than a
    // kind of legislation only that chamber may originate -- a different
    // axis, deliberately not flattened in beside two "Only the ..." powers.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains on the non-chamber and on the procedural contrast: {out}"
    );
}
