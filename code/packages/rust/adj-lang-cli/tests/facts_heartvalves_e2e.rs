//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-valves.adj`) driven through the built CLI:
//! a native `table` of heart-valve → boundary resolves binding-query recalls
//! (forward and backward) with the source's NCI SEER citation, and abstains on
//! a valve that is not one of the four cardiac valves (the eustachian valve) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

#[test]
fn anatomy_heart_valves_recall_binds_boundary_with_citation() {
    let dir = scratch("heartvalves");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/heart-valves.adj");
    std::fs::copy(&src, dir.join("heart-valves.adj")).expect("copy shipped heart-valves.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-valves.adj\"\n\
         ? valve_separates(tricuspid, $B)\n\
         ? valve_separates(pulmonary, $B)\n\
         ? valve_separates($V, left_ventricle_and_aorta)\n\
         ? valve_separates(eustachian, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The tricuspid is the right atrioventricular valve — between the right
    // atrium and right ventricle; the pulmonary sits at the pulmonary trunk
    // (the recalled boundaries, forward binds).
    assert!(
        out.contains("\"B\":\"right_atrium_and_right_ventricle\""),
        "tricuspid → right_atrium_and_right_ventricle: {out}"
    );
    assert!(
        out.contains("\"B\":\"right_ventricle_and_pulmonary_trunk\""),
        "pulmonary → right_ventricle_and_pulmonary_trunk: {out}"
    );
    // The relation runs BACKWARD: bind the boundary, recall the valve.
    assert!(
        out.contains("\"V\":\"aortic\""),
        "left_ventricle_and_aorta → aortic (reverse recall): {out}"
    );
    // ONE CONTIGUOUS SPAN, and it now pins a READ row so the tier means
    // something. `contains(host) && contains("\"trust\":\"authoritative\"")`
    // was here -- the #15209 two-loose-needles shape, and it could not
    // distinguish this table's two kinds of row at all, because
    // `authoritative` appears in the output either way on the envelope.
    assert!(
        out.contains(
            "\"source\":\"The valve between the right ventricle and pulmonary trunk is the pulmonary semilunar valve.\",\"locator\":\"https://training.seer.cancer.gov/anatomy/cardiovascular/heart/structure.html\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "the pulmonary row is READ: one span, envelope tier, no corroboration: {out}"
    );
    // The eustachian valve is not one of the four cardiac valves in this table —
    // honest abstention, never a fabricated boundary.
    assert!(out.contains("\"abstained\":true"), "eustachian abstains: {out}");
}

const LOCATOR: &str = "https://training.seer.cancer.gov/anatomy/cardiovascular/heart/structure.html";
const AV_SPAN: &str = "The valves between the atria and ventricles are called atrioventricular valves (also called cuspid valves), while those at the bases of the large vessels leaving the ventricles are called semilunar valves.";

/// (valve, boundary, its own span, tier, whether it corroborates with AV_SPAN)
const VALVES: [(&str, &str, &str, &str, bool); 4] = [
    (
        "tricuspid",
        "right_atrium_and_right_ventricle",
        "The right atrioventricular valve is the tricuspid valve.",
        "inferred",
        true,
    ),
    (
        "mitral",
        "left_atrium_and_left_ventricle",
        "The left atrioventricular valve is the bicuspid, or mitral, valve.",
        "inferred",
        true,
    ),
    (
        "pulmonary",
        "right_ventricle_and_pulmonary_trunk",
        "The valve between the right ventricle and pulmonary trunk is the pulmonary semilunar valve.",
        "authoritative",
        false,
    ),
    (
        "aortic",
        "left_ventricle_and_aorta",
        "The valve between the left ventricle and the aorta is the aortic semilunar valve.",
        "authoritative",
        false,
    ),
];

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    let src = facts_stdlib().join("anatomy/heart-valves.adj");
    std::fs::copy(&src, dir.join("heart-valves.adj")).expect("copy shipped heart-valves.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"heart-valves.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn each_valve_carries_its_own_span_at_the_tier_that_span_earns() {
    // THE HEART OF THIS TABLE. Until #14986 was applied here every row was
    // warranted by the TRICUSPID sentence -- three of the four by a sentence
    // about a different valve.
    //
    // The pin below is ONE CONTIGUOUS RUN binding source, locator and TIER.
    // That is what makes the two kinds of row distinguishable: promoting a
    // reasoned row to `authoritative` changes these bytes. Checking the tier
    // and the span separately would pass on either row.
    for (valve, boundary, span, tier, corroborates) in VALVES {
        let out = ask(&format!("tier_{valve}"), &format!("valve_separates({valve}, $B)"));
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {valve}, so the absences below are about it: {out}"
        );
        assert!(
            out.contains(&format!("\"B\":\"{boundary}\"")),
            "{valve} still binds {boundary}: {out}"
        );
        let tail = if corroborates {
            format!("\"corroborations\":[{{\"source\":\"{AV_SPAN}\",\"locator\":\"{LOCATOR}\"}}]")
        } else {
            "\"corroborations\":[]".to_string()
        };
        assert!(
            out.contains(&format!(
                "\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"{tier}\",{tail}"
            )),
            "{valve}: own span at tier {tier}, corroborated={corroborates}: {out}"
        );
        // NEGATIVE ARM: no other valve's sentence reaches this answer. The
        // tricuspid sentence reached all four before this change.
        for (other, _, other_span, _, _) in VALVES {
            if other != valve {
                assert!(
                    !out.contains(other_span),
                    "the {other} sentence must not reach the {valve} answer: {out}"
                );
            }
        }
    }
}

#[test]
fn a_read_row_is_never_downgraded_and_a_reasoned_row_is_never_promoted() {
    // THE HONEST TIER, asserted in both directions. A reasoned row shipped at
    // `authoritative` would claim the page STATES the boundary; it does not,
    // it states that the valve is atrioventricular and, separately, what an
    // atrioventricular valve sits between. A read row shipped at `inferred`
    // would understate a sentence that names the valve and both chambers.
    for (valve, _, _, tier, corroborates) in VALVES {
        let out = ask(&format!("tiers_{valve}"), &format!("valve_separates({valve}, $B)"));
        let wrong = if tier == "inferred" { "authoritative" } else { "inferred" };
        assert!(
            out.contains(&format!("\"trust\":\"{tier}\"")),
            "{valve} is at {tier}: {out}"
        );
        // THE STRONGER ARM IS AVAILABLE HERE, and an earlier comment
        // talked itself out of it on a premise that is false for THIS test:
        // it said the envelope's `authoritative` appears in the output
        // either way. It does not. The envelope is not a fact, and every row
        // overrides `source`, so on a single-valve query the envelope's tier
        // never reaches stdout -- measured, the tricuspid query contains
        // `authoritative` zero times and the pulmonary query contains
        // `inferred` zero times. So the wrong tier must be absent outright.
        assert!(
            !out.contains(&format!("\"trust\":\"{wrong}\"")),
            "{valve} must not appear at {wrong} at all: {out}"
        );
        assert_eq!(
            out.contains(AV_SPAN),
            corroborates,
            "{valve} corroborates with the atrioventricular definition: {corroborates}: {out}"
        );
    }
}

#[test]
fn the_table_ships_two_reasoned_rows_and_two_read_rows() {
    // STRUCTURAL, READ FROM THE SHIPPED FILE. The split is the design, so a
    // future edit that quietly makes every row `inferred` -- or none --
    // should fail here rather than pass as a wash.
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/heart-valves.adj"))
        .expect("read shipped heart-valves.adj");
    let body = &adj[adj.find("table valve_separates").expect("table")..];

    assert_eq!(body.matches("\n    row (").count(), 4, "four rows");
    // INDENTATION-INSENSITIVE. `"\n        source \""` matches only at
    // exactly eight spaces, and a row field at seven or nine evades it --
    // the shape that was the water-cycle review's main finding one entry
    // ago. Counting by trimmed prefix leaves the envelope's line plus one
    // per row.
    let source_lines: Vec<&str> = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect();
    assert_eq!(
        source_lines.len(),
        5,
        "four row sources plus the envelope's: {source_lines:?}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim() == "trust inferred")
            .count(),
        2,
        "exactly two rows are reasoned"
    );
    assert_eq!(
        body.matches("\n        cites \"").count(),
        2,
        "and exactly those two carry a second span"
    );
    // The `cites` locator is the corroboration's own mandatory address, not
    // a row locator override -- of which there are none, because every span
    // is on the one page the envelope names.
    let row_locators: Vec<&str> = body
        .lines()
        .filter(|l| l.trim_start().starts_with("locator \""))
        .filter(|l| !l.starts_with("    locator \""))
        .collect();
    assert!(
        row_locators.is_empty(),
        "no row overrides the locator at any indentation: {row_locators:?}"
    );

    // AND THE REASONED ROWS ARE THE RIGHT TWO. Counting two of each would
    // pass if the tiers were swapped onto the wrong pair.
    for (valve, tier) in [
        ("tricuspid", "inferred"),
        ("mitral", "inferred"),
        ("pulmonary", "authoritative"),
        ("aortic", "authoritative"),
    ] {
        let start = body
            .find(&format!("row ({valve},"))
            .unwrap_or_else(|| panic!("row for {valve}"));
        // NOT `unwrap_or(0)`. An empty block satisfies
        // `!contains("trust inferred")`, which is the EXPECTED value for the
        // two read rows -- so a missing closing brace would make the
        // assertion named "the reasoned rows are the right two" pass while
        // asserting nothing about them.
        let block_end = body[start..]
            .find("\n    }")
            .unwrap_or_else(|| panic!("row block for {valve} must close"))
            + start;
        let block = &body[start..block_end];
        assert_eq!(
            block.contains("trust inferred"),
            tier == "inferred",
            "{valve} is {tier}: {block}"
        );
    }
}
