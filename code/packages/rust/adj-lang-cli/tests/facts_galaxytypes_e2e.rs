//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/galaxy-types.adj`) driven through the built CLI:
//! a native `table` of the main galaxy types → their defining shape resolves
//! binding-query recalls (forward AND backward) with the source's NASA Science
//! citation, and abstains on a word that is not one of the main galaxy types (a
//! planet) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn astronomy_galaxy_types_recall_binds_shape_with_citation() {
    let dir = scratch("galaxytypes");
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/galaxy-types.adj");
    std::fs::copy(&src, dir.join("galaxy-types.adj")).expect("copy shipped galaxy-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"galaxy-types.adj\"\n\
         ? galaxy_shape(spiral, $Shape)\n\
         ? galaxy_shape(elliptical, $Shape)\n\
         ? galaxy_shape(irregular, $Shape)\n\
         ? galaxy_shape($Type, round_to_oval)\n\
         ? galaxy_shape(planet, $Shape)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A spiral galaxy is defined by its spiral arms, an elliptical ranges from
    // round to oval, an irregular has unusual shapes — the recalled shapes
    // (forward binds).
    assert!(
        out.contains("\"Shape\":\"spiral_arms\""),
        "spiral → spiral_arms: {out}"
    );
    assert!(
        out.contains("\"Shape\":\"round_to_oval\""),
        "elliptical → round_to_oval: {out}"
    );
    assert!(
        out.contains("\"Shape\":\"unusual_shapes\""),
        "irregular → unusual_shapes: {out}"
    );
    // The relation runs BACKWARD: bind the shape `round_to_oval`, recall its
    // galaxy type.
    assert!(
        out.contains("\"Type\":\"elliptical\""),
        "round_to_oval → elliptical (reverse recall): {out}"
    );
    // The answer carries the NASA Science citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A planet is not one of the main galaxy types — honest abstention, never a
    // fabricated shape.
    assert!(out.contains("\"abstained\":true"), "planet abstains: {out}");
}
