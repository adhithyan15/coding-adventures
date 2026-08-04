//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/friction-types.adj`) driven through the built
//! CLI: a native `table` of the four everyday friction types -> the context
//! (what each acts on / when it occurs) resolves binding-query recalls (forward
//! AND backward) with the source's Testbook "Types of Friction" citation, and
//! abstains on a word that is not one of the four friction types (gravity) — 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsf_{tag}_{}", std::process::id()));
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
fn physics_friction_types_recall_binds_context_with_citation() {
    let dir = scratch("frictiontypes");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/friction-types.adj");
    std::fs::copy(&src, dir.join("friction-types.adj")).expect("copy shipped friction-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"friction-types.adj\"\n\
         ? friction_context(static, $Context)\n\
         ? friction_context(rolling, $Context)\n\
         ? friction_context(fluid, $Context)\n\
         ? friction_context($Friction, sliding_motion)\n\
         ? friction_context(gravity, $Context)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Static friction acts on objects kept at rest, rolling friction on a
    // spherical object (wheel/ball), fluid friction between layers of a fluid —
    // the recalled contexts (forward binds).
    assert!(
        out.contains("\"Context\":\"at_rest\""),
        "static -> at_rest: {out}"
    );
    assert!(
        out.contains("\"Context\":\"spherical_object\""),
        "rolling -> spherical_object: {out}"
    );
    assert!(
        out.contains("\"Context\":\"fluid_layers\""),
        "fluid -> fluid_layers: {out}"
    );
    // The relation runs BACKWARD: bind the context `sliding_motion`, recall its
    // friction type.
    assert!(
        out.contains("\"Friction\":\"sliding\""),
        "sliding_motion -> sliding (reverse recall): {out}"
    );
    // The answer carries the Testbook "Types of Friction" citation as its proof,
    // at the `consensus` trust tier for a secondary teaching summary.
    assert!(
        out.contains("testbook.com") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // Gravity is a force, not one of the four everyday friction types — honest
    // abstention, never a fabricated context.
    assert!(out.contains("\"abstained\":true"), "gravity abstains: {out}");
}
