//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/planet-criterion.adj`) driven through the
//! built CLI: a native `table` naming the three IAU requirements a body
//! must meet to count as a full planet, quoted verbatim from NASA
//! Science's "Dwarf Planets" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_planet_criterion_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/planet-criterion.adj");
    std::fs::copy(&src, dir.join("planet-criterion.adj")).expect("copy shipped planet-criterion.adj");
}

#[test]
fn planet_criterion_recall_binds_the_requirement_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"planet-criterion.adj\"\n\
         ? planet_criterion(roundness, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"R\":\"is_mostly_round\""),
        "roundness requires is_mostly_round: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn planet_criterion_reverse_binds_the_criterion_for_that_requirement() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"planet-criterion.adj\"\n\
         ? planet_criterion($C, orbits_its_host_star)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"orbit\""),
        "the shipped orbits_its_host_star example is orbit: {out}"
    );
}

#[test]
fn planet_criterion_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"planet-criterion.adj\"\n\
         ? planet_criterion(dwarf_planet, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "dwarf_planet is a real classification the source covers but is a compound category built from these criteria, not a fourth criterion itself -- honest abstention, never invented: {out}"
    );
}
