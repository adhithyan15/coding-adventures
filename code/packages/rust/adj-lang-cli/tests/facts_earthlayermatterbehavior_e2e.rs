//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/earth-layer-matter-behavior.adj`) driven
//! through the built CLI: a `rule` composing the already-shipped `has_state`
//! table (`geology/earth-layers.adj`) with the already-shipped `matter_state`
//! table (`chemistry/states-of-matter.adj`, a CROSS-DIRECTORY import via
//! `../chemistry/states-of-matter.adj`, the same shape
//! `earth-science/season-start-month-number.adj` already established) to
//! DERIVE `earth_layer_matter_behavior($Layer, $Behavior)` -- the THIRD
//! `rule`-based CAUSAL-EXPLANATION fact in this loop's science curriculum
//! sweep, mirroring the discipline `heat-causes-phase-change.adj` and
//! `force-causes-acceleration.adj` already established, applied here to a
//! genuinely CROSS-DIRECTORY pair (geology + chemistry). 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_earthlayerbehavior_{tag}_{}", std::process::id()));
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
/// structure: `earth-layer-matter-behavior.adj` (in `geology/`) imports
/// `earth-layers.adj` (same dir) and `../chemistry/states-of-matter.adj`
/// (cross-directory), so the entry program must sit at a root that contains
/// both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("geology/earth-layers.adj", "geology/earth-layers.adj"),
        (
            "geology/earth-layer-matter-behavior.adj",
            "geology/earth-layer-matter-behavior.adj",
        ),
        (
            "chemistry/states-of-matter.adj",
            "chemistry/states-of-matter.adj",
        ),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn outer_core_derives_takes_shape_of_container_with_dual_citations() {
    let dir = scratch("outercore");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geology/earth-layer-matter-behavior.adj\"\n\
         ? earth_layer_matter_behavior(outer_core, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"B\":\"takes_shape_of_container\""),
        "the liquid outer core takes the shape of its container: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: USGS
    // (has_state, geology) AND NASA GRC (matter_state, chemistry).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the causal fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("pubs.usgs.gov") && out.contains("grc.nasa.gov"),
        "carries citations from BOTH composed libraries (earth-layers.adj and states-of-matter.adj): {out}"
    );
}

#[test]
fn fixed_shape_reverse_binds_to_inner_core() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geology/earth-layer-matter-behavior.adj\"\n\
         ? earth_layer_matter_behavior($L, fixed_shape)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"inner_core\""),
        "the solid inner core holds a fixed shape: {out}"
    );
}

#[test]
fn crust_abstains_honestly_as_rigid_is_not_a_keyed_state_of_matter() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geology/earth-layer-matter-behavior.adj\"\n\
         ? earth_layer_matter_behavior(crust, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the crust's own state word is \"rigid\", not one of matter_state's three keyed states (solid/liquid/gas) -- honest abstention, never invented: {out}"
    );
}
