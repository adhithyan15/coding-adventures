//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/celestial-objects.adj`) driven through the built
//! CLI: a native `table` of the basic celestial-object types → a short defining
//! property resolves binding-query recalls (forward AND backward) with the
//! source's NASA citation, and abstains on a word that is not one of the basic
//! celestial-object types (a cloud) — 0 model calls.

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
fn astronomy_celestial_objects_recall_binds_property_with_citation() {
    let dir = scratch("celestialobjects");
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/celestial-objects.adj");
    std::fs::copy(&src, dir.join("celestial-objects.adj"))
        .expect("copy shipped celestial-objects.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-objects.adj\"\n\
         ? celestial_property(star, $Property)\n\
         ? celestial_property(planet, $Property)\n\
         ? celestial_property(asteroid, $Property)\n\
         ? celestial_property($Object, orbits_planet)\n\
         ? celestial_property(cloud, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A star gives off its own light, a planet revolves around a star, an
    // asteroid is rocky — the recalled properties (forward binds).
    assert!(
        out.contains("\"Property\":\"gives_off_light\""),
        "star → gives_off_light: {out}"
    );
    assert!(
        out.contains("\"Property\":\"revolves_around_star\""),
        "planet → revolves_around_star: {out}"
    );
    assert!(
        out.contains("\"Property\":\"rocky\""),
        "asteroid → rocky: {out}"
    );
    // The relation runs BACKWARD: bind the property `orbits_planet`, recall
    // which object it defines.
    assert!(
        out.contains("\"Object\":\"moon\""),
        "orbits_planet → moon (reverse recall): {out}"
    );
    // The answer carries the NASA citation as its proof, at the `authoritative`
    // trust tier for a primary U.S. government source.
    assert!(
        out.contains("starchild.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A cloud is not one of the basic celestial-object types — honest
    // abstention, never a fabricated property.
    assert!(out.contains("\"abstained\":true"), "cloud abstains: {out}");
}
