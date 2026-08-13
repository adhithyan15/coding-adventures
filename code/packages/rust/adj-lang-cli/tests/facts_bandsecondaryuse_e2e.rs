//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/band-secondary-use.adj`) driven through the
//! built CLI: a native `table` naming a SECOND everyday use NASA's source
//! states for an EM band, where the source names two -- a sibling to the
//! already-shipped `em-spectrum.adj` (which only carries the FIRST everyday
//! use per band), decoding spans already sitting unused inside that table's
//! own provenance block. Resolves binding-query recall (both directions)
//! with the source's citation, and abstains on a band (radio) the cited
//! spans give no second use for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_bandsecondaryuse_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/band-secondary-use.adj");
    std::fs::copy(&src, dir.join("band-secondary-use.adj"))
        .expect("copy shipped band-secondary-use.adj");
}

#[test]
fn band_secondary_use_recalls_both_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-secondary-use.adj\"\n\
         ? band_secondary_use(microwave, $Application)\n\
         ? band_secondary_use(x_ray, $Application)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"band_secondary_use(microwave, astronomy)\""),
        "microwave's secondary use is astronomy: {out}"
    );
    assert!(
        out.contains("\"term\":\"band_secondary_use(x_ray, airport_security)\""),
        "x_ray's secondary use is airport_security: {out}"
    );
    assert!(
        out.contains("imagine.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn band_secondary_use_recalls_backward_from_a_bound_application() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-secondary-use.adj\"\n\
         ? band_secondary_use($Band, airport_security)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"band_secondary_use(x_ray, airport_security)\""),
        "airport_security names x_ray: {out}"
    );
}

#[test]
fn band_secondary_use_abstains_honestly_on_radio() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-secondary-use.adj\"\n\
         ? band_secondary_use(radio, $Application)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "radio has no second use in the cited spans -- honest abstention: {out}"
    );
}
