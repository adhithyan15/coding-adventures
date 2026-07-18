//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/forces.adj`) driven through the built CLI:
//! a native `table` of common force → everyday example resolves a binding
//! query recall with the NASA citation, runs backward (example → force), and
//! abstains on a force the source never illustrates — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsforces_{tag}_{}", std::process::id()));
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
fn physics_forces_recall_binds_example_with_citation() {
    let dir = scratch("forces");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/forces.adj");
    std::fs::copy(&src, dir.join("forces.adj")).expect("copy shipped forces.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"forces.adj\"\n\
         ? force_example(gravity, $Ex)\n\
         ? force_example(applied, $Ex)\n\
         ? force_example(friction, $Ex)\n\
         ? force_example($F, ropes)\n\
         ? force_example(magnetism, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each force to the everyday example NASA names.
    assert!(
        out.contains("\"Ex\":\"waterfall\""),
        "gravity binds to waterfall: {out}"
    );
    assert!(
        out.contains("force_example(gravity, waterfall)"),
        "gravity is governing-bound to waterfall: {out}"
    );
    assert!(
        out.contains("force_example(applied, ball)"),
        "applied is governing-bound to ball: {out}"
    );
    assert!(
        out.contains("force_example(friction, wheels)"),
        "friction is governing-bound to wheels: {out}"
    );
    // The relation runs BACKWARD: bind the example, recall the force.
    assert!(
        out.contains("force_example(tension, ropes)"),
        "reverse recall binds F=tension from ropes: {out}"
    );
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Magnetism is a real force, but the poster never illustrates it — honest abstention.
    assert!(out.contains("\"abstained\":true"), "magnetism abstains: {out}");
}
