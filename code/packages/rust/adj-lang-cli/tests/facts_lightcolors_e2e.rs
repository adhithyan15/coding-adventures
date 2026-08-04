//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/light-colors.adj`) driven through the built CLI:
//! a native `table` of additive-primary-of-light -> what-it-combines-to resolves
//! a binding query recall with the HyperPhysics (Georgia State University)
//! citation, and abstains on a color that is not an additive primary — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslc_{tag}_{}", std::process::id()));
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
fn physics_light_colors_recall_binds_combined_color_with_citation() {
    let dir = scratch("lightcolors");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/light-colors.adj");
    std::fs::copy(&src, dir.join("light-colors.adj")).expect("copy shipped light-colors.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"light-colors.adj\"\n\
         ? light_primary(red, $Makes)\n\
         ? light_primary($Color, white)\n\
         ? light_primary(yellow, $Makes)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Red is an additive primary of light — it combines to `white`.
    assert!(out.contains("\"Makes\":\"white\""), "red binds to white: {out}");
    assert!(
        out.contains("light_primary(red, white)"),
        "red is governing-bound to white: {out}"
    );
    // The reverse query binds every additive primary that makes white.
    assert!(
        out.contains("light_primary(green, white)")
            && out.contains("light_primary(blue, white)"),
        "reverse recall binds green and blue to white: {out}"
    );
    // The answer carries the HyperPhysics locator + trust tier as its proof.
    assert!(
        out.contains("hyperphysics.gsu.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Yellow is a SECONDARY color of light — honest abstention, never a fabricated answer.
    assert!(out.contains("\"abstained\":true"), "yellow abstains: {out}");
}
