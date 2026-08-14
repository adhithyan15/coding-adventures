//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/mirror-focal-length-sign.adj`) driven
//! through the built CLI: a native `table` recording, for the two mirror
//! types already tabled in `lens-types.adj`, the SIGN convention the
//! same already-quoted OpenStax sentence gives for each mirror's focal
//! length -- a sibling decoding the sign-convention half of each
//! already-verified quote. Resolves forward and backward recall queries
//! with the source's citation, plus honest abstention on convex_lens
//! (whose cited span describes ray behavior but never states a sign
//! convention) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mirrorfocallengthsign_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/mirror-focal-length-sign.adj");
    std::fs::copy(&src, dir.join("mirror-focal-length-sign.adj"))
        .expect("copy shipped mirror-focal-length-sign.adj");
}

#[test]
fn mirror_focal_length_sign_recalls_concave_mirror_as_positive_with_citation() {
    let dir = scratch("concave");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mirror-focal-length-sign.adj\"\n\
         ? mirror_focal_length_sign(concave_mirror, $Sign)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"mirror_focal_length_sign(concave_mirror, positive)\""),
        "concave mirror should recall its cited sign: {out}"
    );
    assert!(
        out.contains("phys.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the OpenStax/LibreTexts citation: {out}"
    );
}

#[test]
fn mirror_focal_length_sign_backward_recalls_convex_mirror_for_negative() {
    let dir = scratch("convex");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mirror-focal-length-sign.adj\"\n\
         ? mirror_focal_length_sign($Mirror, negative)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"mirror_focal_length_sign(convex_mirror, negative)\""),
        "convex mirror should be the only recalled negative-sign mirror: {out}"
    );
    assert!(
        !out.contains("mirror_focal_length_sign(concave_mirror, negative)"),
        "concave mirror's cited sign is positive, not negative: {out}"
    );
}

#[test]
fn mirror_focal_length_sign_abstains_on_convex_lens() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mirror-focal-length-sign.adj\"\n\
         ? mirror_focal_length_sign(convex_lens, $SignLens)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "convex_lens's cited span describes ray behavior but never states a sign convention -- honest abstention expected: {out}"
    );
}
