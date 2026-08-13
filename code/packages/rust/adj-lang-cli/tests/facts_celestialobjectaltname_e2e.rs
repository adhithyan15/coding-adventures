//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/celestial-object-alt-name.adj`) driven
//! through the built CLI: a native `table` naming an ALTERNATE NAME NASA
//! gives a basic celestial object, where the source states one -- a
//! sibling to the already-shipped `celestial-objects.adj` (which only
//! carries ONE defining property per object), decoding spans already
//! sitting unused inside that table's own provenance block. Resolves
//! binding-query recall with the source's citation, and abstains on an
//! object the cited spans do not give a second name for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_celestialaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/celestial-object-alt-name.adj");
    std::fs::copy(&src, dir.join("celestial-object-alt-name.adj"))
        .expect("copy shipped celestial-object-alt-name.adj");
}

#[test]
fn celestial_object_alt_name_recalls_both_terms_with_citation() {
    let dir = scratch("both");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-object-alt-name.adj\"\n\
         ? celestial_object_alt_name(moon, $AltName)\n\
         ? celestial_object_alt_name(asteroid, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"celestial_object_alt_name(moon, planetary_satellites)\""),
        "moon's alternate name is planetary_satellites: {out}"
    );
    assert!(
        out.contains("\"term\":\"celestial_object_alt_name(asteroid, minor_planets)\""),
        "asteroid's alternate name is minor_planets: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn celestial_object_alt_name_abstains_honestly_on_an_undistinguished_object() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-object-alt-name.adj\"\n\
         ? celestial_object_alt_name(star, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "star's cited span names no alternate term -- honest abstention: {out}"
    );
}
