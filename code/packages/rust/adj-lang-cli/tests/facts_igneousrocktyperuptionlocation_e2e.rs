//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/igneous-rock-type-eruption-location.adj`)
//! driven through the built CLI: a native `table` recording, for the
//! extrusive igneous type already tabled in `igneous-rock-type.adj`, each
//! individual location an already-quoted NPS sentence lists as where it
//! erupts -- a sibling decoding each listed location as its own row instead
//! of folding the whole clause into one compound `description` atom.
//! Resolves forward (multi-answer) and backward recall queries with the
//! source's citation, plus honest abstention on intrusive (whose cited span
//! names only one location) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_igneouseruptionlocation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/igneous-rock-type-eruption-location.adj");
    std::fs::copy(&src, dir.join("igneous-rock-type-eruption-location.adj"))
        .expect("copy shipped igneous-rock-type-eruption-location.adj");
}

#[test]
fn igneous_rock_type_eruption_location_recalls_extrusive_locations_with_citation() {
    let dir = scratch("extrusive");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type-eruption-location.adj\"\n\
         ? igneous_rock_type_eruption_location(extrusive, $Location)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"igneous_rock_type_eruption_location(extrusive, surface)\""),
        "extrusive should recall surface: {out}"
    );
    assert!(
        out.contains("\"term\":\"igneous_rock_type_eruption_location(extrusive, atmosphere)\""),
        "extrusive should recall atmosphere: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn igneous_rock_type_eruption_location_backward_recalls_extrusive_for_atmosphere() {
    let dir = scratch("atmosphere");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type-eruption-location.adj\"\n\
         ? igneous_rock_type_eruption_location($Type, atmosphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"igneous_rock_type_eruption_location(extrusive, atmosphere)\""),
        "extrusive should be the only recalled type for atmosphere: {out}"
    );
}

#[test]
fn igneous_rock_type_eruption_location_abstains_on_intrusive() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type-eruption-location.adj\"\n\
         ? igneous_rock_type_eruption_location(intrusive, $LocationIntrusive)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "intrusive's cited span names only one location, no listed alternatives -- honest abstention expected: {out}"
    );
}
