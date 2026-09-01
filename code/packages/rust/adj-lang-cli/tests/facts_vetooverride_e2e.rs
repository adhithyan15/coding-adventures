//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/veto-override.adj`) driven through the built
//! CLI: a native `table` recording whether Congress can override each kind
//! of presidential veto, grounding USA.gov's "How laws are made" page.
//!
//! The SIXTH library in the `civics/` domain. Its point is a contrast the
//! shipped `checks-and-balances.adj` cannot express: that table holds
//! `checks_and_balances(president, veto, legislation_created_by_congress)`
//! -- the veto EXISTS as an act one branch takes against another -- while
//! this one records whether that act can be UNDONE, and by whom. The check
//! and its own counter-check.
//!
//! The most important assertion here is that THE HEDGE SURVIVES. The
//! source says "in most cases Congress can vote to override that veto",
//! not that Congress always can, so the ordinary-veto row binds
//! `congress_can_override_in_most_cases` rather than a bare `yes`. A test
//! that accepted `yes` would let a future edit quietly overclaim.
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
        "adjcli_factsvetooverride_{tag}_{}",
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
fn veto_override_keeps_the_sources_hedge_on_the_ordinary_veto() {
    let dir = scratch("hedge");
    place(&dir, &["veto-override.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"veto-override.adj\"\n\
         ? veto_override(veto, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The source says "in most cases Congress can vote to override that
    // veto" -- NOT that Congress always can. The atom must keep the
    // qualifier; a bare `yes` would state something the source declines to.
    assert!(
        out.contains("\"S\":\"congress_can_override_in_most_cases\""),
        "the hedge survives into the recalled atom: {out}"
    );
    assert!(
        !out.contains("\"S\":\"yes\""),
        "must not flatten the hedge to a bare yes: {out}"
    );
    // Guard the STRING-literal spelling too. `Term::Str` renders via `{:?}`,
    // so a future row written `row (veto, "yes")` rather than as a bare atom
    // would emit `"S":"\"yes\""` and would otherwise slip past the check
    // above -- the anti-overclaim guard has to cover both spellings or it
    // only covers the one that happens to be in use today.
    assert!(
        !out.contains("\\\"yes\\\""),
        "must not flatten the hedge to a quoted yes either: {out}"
    );
    // And the qualifying sentence itself rides along as proof.
    assert!(
        out.contains("in most cases Congress can vote to override that veto"),
        "carries the hedged sentence verbatim: {out}"
    );
    assert!(
        out.contains("usa.gov/how-laws-are-made") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn veto_override_binds_the_pocket_veto_as_not_overridable() {
    let dir = scratch("pocket");
    place(&dir, &["veto-override.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"veto-override.adj\"\n\
         ? veto_override(pocket_veto, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // This row carries NO hedge, because its own sentence carries none.
    // The asymmetry between the two atoms is faithful to the source.
    assert!(
        out.contains("\"S\":\"cannot_be_overridden_by_congress\""),
        "pocket veto cannot be overridden: {out}"
    );
    assert!(
        out.contains("This action is called a pocket veto, and it cannot be overridden by Congress."),
        "carries the pocket-veto sentence verbatim: {out}"
    );
}

#[test]
fn veto_override_runs_backward_from_the_status_to_the_veto_kind() {
    let dir = scratch("reverse");
    place(&dir, &["veto-override.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"veto-override.adj\"\n\
         ? veto_override($T, cannot_be_overridden_by_congress)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("veto_override(pocket_veto, cannot_be_overridden_by_congress)"),
        "the un-overridable kind is the pocket veto: {out}"
    );
}

#[test]
fn veto_override_composes_with_the_checks_and_balances_act() {
    let dir = scratch("compose");
    place(&dir, &["veto-override.adj", "checks-and-balances.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"veto-override.adj\"\n\
         import \"checks-and-balances.adj\"\n\
         ? checks_and_balances(president, veto, $O)\n\
         ? veto_override(veto, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The two libraries hold different halves of the same institution:
    // checks_and_balances says the veto EXISTS as an act against Congress;
    // veto_override says whether that act can be UNDONE. `veto` appears in
    // both, but as an ACTION there and a KIND here -- distinct predicates,
    // so the two recalls can never be confused for one another.
    assert!(
        out.contains("\"O\":\"legislation_created_by_congress\""),
        "the act: the president vetoes legislation created by Congress: {out}"
    );
    assert!(
        out.contains("\"S\":\"congress_can_override_in_most_cases\""),
        "the counter-check: Congress can override, in most cases: {out}"
    );
}

#[test]
fn veto_override_abstains_on_unnamed_kinds_and_on_the_triggering_condition() {
    let dir = scratch("abstain");
    place(&dir, &["veto-override.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"veto-override.adj\"\n\
         ? veto_override(line_item_veto, $S)\n\
         ? veto_override($T, congress_out_of_session)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // `line_item_veto` is a real term in U.S. civics vocabulary that this
    // source never names. `congress_out_of_session` is part of the
    // CONDITION producing a pocket veto -- stated by the same paragraph,
    // but answering WHEN one happens, not whether it can be overridden.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains on the unnamed kind and on the different axis: {out}"
    );
}
