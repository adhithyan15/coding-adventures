//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/force-causes-acceleration.adj`) driven through
//! the built CLI: a `rule` composing Newton's second law's own general
//! statement (from the sibling `newton-laws.adj`) with a specific
//! force->example fact (from the sibling `forces.adj`) to DERIVE
//! `force_causes_acceleration($Force, $Example)` -- the second `rule`-based
//! CAUSAL-EXPLANATION fact in this loop's science curriculum sweep, mirroring
//! the discipline `heat-causes-phase-change.adj` already established. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_forceaccel_{tag}_{}", std::process::id()));
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

/// Copy ALL THREE shipped libraries beside the entry program:
/// `force-causes-acceleration.adj` transitively imports both sibling
/// `newton-laws.adj` and `forces.adj`, so the CLI's sandbox-checked relative
/// import needs all three present.
fn place_libs(dir: &Path) {
    let physics = facts_stdlib().join("physics");
    for name in ["newton-laws.adj", "forces.adj", "force-causes-acceleration.adj"] {
        std::fs::copy(physics.join(name), dir.join(name))
            .unwrap_or_else(|e| panic!("copy shipped physics/{name}: {e}"));
    }
}

#[test]
fn gravity_derives_acceleration_of_its_own_waterfall_example_with_dual_citations() {
    let dir = scratch("gravity");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"force-causes-acceleration.adj\"\n\
         ? force_causes_acceleration(gravity, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Ex\":\"waterfall\""),
        "gravity causes the waterfall's acceleration: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: the
    // rule's own second-law statement AND forces.adj's specific example fact.
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the causal fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("imagine.gsfc.nasa.gov") && out.contains("www1.grc.nasa.gov"),
        "carries citations from BOTH composed libraries (forces.adj and newton-laws.adj): {out}"
    );
}

#[test]
fn wheels_reverse_binds_to_friction() {
    let dir = scratch("wheels");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"force-causes-acceleration.adj\"\n\
         ? force_causes_acceleration($F, wheels)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"F\":\"friction\""),
        "the wheels example illustrates friction: {out}"
    );
}

#[test]
fn magnetism_abstains_honestly_as_not_one_of_the_five_tabled_forces() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"force-causes-acceleration.adj\"\n\
         ? force_causes_acceleration(magnetism, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "magnetism has no shipped row in forces.adj -- honest abstention, never invented: {out}"
    );
}
