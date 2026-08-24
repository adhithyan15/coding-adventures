//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/weathering-cause-type.adj`) driven
//! through the built CLI: a native `table` naming five causes of weathering
//! and which of the two basic weathering types (physical or chemical) each
//! one belongs to, grounding the U.S. National Park Service's "Weathering
//! and Erosion" article (Scotts Bluff National Monument). Runs the relation
//! BACKWARD as a genuine one-to-many recall in both directions (three
//! physical causes, two chemical causes), and abstains honestly on
//! `erosion` -- a real Earth process the same cited article discusses at
//! length, but as a distinct, LATER process, not a weathering type. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_weatheringcausetype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/weathering-cause-type.adj");
    std::fs::copy(&src, dir.join("weathering-cause-type.adj"))
        .expect("copy shipped weathering-cause-type.adj");
}

#[test]
fn weathering_cause_type_recall_binds_type_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weathering-cause-type.adj\"\n\
         ? weathering_cause_type(heating_and_cooling, $Type)\n\
         ? weathering_cause_type(acid_exposure, $Type)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("weathering_cause_type(heating_and_cooling, physical)"),
        "heating and cooling is physical weathering: {out}"
    );
    assert!(
        out.contains("weathering_cause_type(acid_exposure, chemical)"),
        "acid exposure is chemical weathering: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the National Park Service citation at authoritative trust: {out}"
    );
}

#[test]
fn weathering_cause_type_reverse_binds_every_cause_of_each_type() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weathering-cause-type.adj\"\n\
         ? weathering_cause_type($C, physical)\n\
         ? weathering_cause_type($C, chemical)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The relation runs BACKWARD as a genuine one-to-many recall: binding
    // `physical` recalls all THREE causes the source assigns to it.
    for cause in ["heating_and_cooling", "foreign_crystal_growth", "rock_collision"] {
        assert!(
            out.contains(&format!("weathering_cause_type({cause}, physical)")),
            "physical recalls {cause}: {out}"
        );
    }
    // Binding `chemical` recalls both causes the source assigns to it.
    for cause in ["acid_exposure", "oxygen_exposure"] {
        assert!(
            out.contains(&format!("weathering_cause_type({cause}, chemical)")),
            "chemical recalls {cause}: {out}"
        );
    }
}

#[test]
fn weathering_cause_type_abstains_honestly_on_erosion() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"weathering-cause-type.adj\"\n\
         ? weathering_cause_type(erosion, $Type)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "erosion is a real Earth process the cited article also discusses, but its own \
         structure treats erosion as a distinct, LATER process that moves weathering's \
         products away -- not a weathering type -- honest abstention, never invented: {out}"
    );
}
