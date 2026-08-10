//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/planet-ordinal-position.adj`) driven through
//! the built CLI: a `rule` composing the already-shipped `planet_order`
//! table (`astronomy/planets.adj`) with the already-shipped `ordinal_number`
//! table (`mathematics/ordinal-numbers.adj`, a CROSS-DIRECTORY import via
//! `../mathematics/ordinal-numbers.adj`, the same shape
//! `earth-science/season-start-month-number.adj` already established) to
//! DERIVE `planet_ordinal_position($Planet, $Ordinal)` -- the SECOND
//! cross-DIRECTORY `rule` composition in this loop's science curriculum
//! sweep. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_planetordinal_{tag}_{}", std::process::id()));
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

/// Copy BOTH shipped libraries, preserving their real relative directory
/// structure: `planet-ordinal-position.adj` (in `astronomy/`) imports
/// `planets.adj` (same dir) and `../mathematics/ordinal-numbers.adj`
/// (cross-directory), so the entry program must sit at a root that contains
/// both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("astronomy/planets.adj", "astronomy/planets.adj"),
        (
            "astronomy/planet-ordinal-position.adj",
            "astronomy/planet-ordinal-position.adj",
        ),
        (
            "mathematics/ordinal-numbers.adj",
            "mathematics/ordinal-numbers.adj",
        ),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn earth_derives_third_with_dual_citations() {
    let dir = scratch("earth");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"astronomy/planet-ordinal-position.adj\"\n\
         ? planet_ordinal_position(earth, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"O\":\"third\""),
        "Earth is the third planet: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: NASA
    // (planet_order) AND the ordinal-word convention (ordinal_number).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derivation is a rule composing two fact steps: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("ef.edu"),
        "carries citations from BOTH composed libraries (planets.adj and ordinal-numbers.adj): {out}"
    );
}

#[test]
fn eighth_reverse_binds_to_neptune() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"astronomy/planet-ordinal-position.adj\"\n\
         ? planet_ordinal_position($P, eighth)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"neptune\""),
        "Neptune is the eighth planet: {out}"
    );
}

#[test]
fn pluto_abstains_honestly_as_not_one_of_the_eight_major_planets() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"astronomy/planet-ordinal-position.adj\"\n\
         ? planet_ordinal_position(pluto, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "Pluto has no shipped row (reclassified a dwarf planet in 2006) -- honest abstention, never invented: {out}"
    );
}
