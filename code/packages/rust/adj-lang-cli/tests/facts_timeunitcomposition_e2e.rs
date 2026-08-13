//! End-to-end test for the metrology FACTS library
//! (`adj-facts-stdlib/metrology/time-unit-composition.adj`) driven through
//! the built CLI: a native `table` naming the unit-to-unit composition the
//! SAME NIST source span already states for two time units -- a sibling to
//! the already-shipped `time-units.adj` (which only carries each unit's
//! length in seconds, not a unit-to-unit relation), decoding the
//! composition half of a span already sitting unused inside that table's
//! own `source` field. Resolves binding-query recall (both directions)
//! with the source's citation, and abstains on a unit (minute) the cited
//! span gives no unit-to-unit composition for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_timeunitcomposition_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("metrology/time-unit-composition.adj");
    std::fs::copy(&src, dir.join("time-unit-composition.adj"))
        .expect("copy shipped time-unit-composition.adj");
}

#[test]
fn time_unit_composition_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition(hour, $SubUnit, $Count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"time_unit_composition(hour, minute, 60)\""),
        "an hour is 60 minutes: {out}"
    );
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST citation: {out}"
    );
}

#[test]
fn time_unit_composition_recalls_backward_from_a_bound_sub_unit_and_count() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition($Unit, minute, 60)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"time_unit_composition(hour, minute, 60)\""),
        "60 minutes composes an hour: {out}"
    );
}

#[test]
fn time_unit_composition_abstains_honestly_on_minute() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition(minute, $SubUnit, $Count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "minute has no unit-to-unit composition in the cited span -- honest abstention: {out}"
    );
}
