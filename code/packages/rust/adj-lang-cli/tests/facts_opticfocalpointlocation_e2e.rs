//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/optic-focal-point-location.adj`) driven
//! through the built CLI: a native `table` recording, for three of the
//! four basic optical elements already tabled in `lens-types.adj`, WHERE
//! the same already-quoted OpenStax sentence puts the element's focal
//! point -- a sibling decoding the location half of each already-verified
//! quote. Resolves forward and backward recall queries with the source's
//! citation, plus honest abstention on concave_lens (whose cited span
//! describes ray divergence but never states a focal-point location) --
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
    let dir = std::env::temp_dir().join(format!("adjcli_opticfocalpointlocation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/optic-focal-point-location.adj");
    std::fs::copy(&src, dir.join("optic-focal-point-location.adj"))
        .expect("copy shipped optic-focal-point-location.adj");
}

#[test]
fn optic_focal_point_location_recalls_convex_lens_with_citation() {
    let dir = scratch("lens");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"optic-focal-point-location.adj\"\n\
         ? optic_focal_point_location(convex_lens, $Location)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"optic_focal_point_location(convex_lens, opposite_side_of_the_lens)\""),
        "convex lens should recall its cited focal-point location: {out}"
    );
    assert!(
        out.contains("phys.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the OpenStax/LibreTexts citation: {out}"
    );
}

#[test]
fn optic_focal_point_location_backward_recalls_convex_mirror_for_behind() {
    let dir = scratch("mirror");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"optic-focal-point-location.adj\"\n\
         ? optic_focal_point_location($Optic, behind_the_mirror)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"optic_focal_point_location(convex_mirror, behind_the_mirror)\""),
        "convex mirror should be the only recalled behind-the-mirror element: {out}"
    );
    assert!(
        !out.contains("optic_focal_point_location(concave_mirror, behind_the_mirror)"),
        "concave mirror's cited location is same_side, not behind the mirror: {out}"
    );
}

#[test]
fn optic_focal_point_location_abstains_on_concave_lens() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"optic-focal-point-location.adj\"\n\
         ? optic_focal_point_location(concave_lens, $LocationLens)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "concave_lens's cited span describes ray divergence but never states a focal-point location -- honest abstention expected: {out}"
    );
}
