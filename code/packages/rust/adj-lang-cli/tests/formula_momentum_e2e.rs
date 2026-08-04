//! End-to-end tests for the `physics/momentum.adj` library — the definition of
//! linear momentum (p = m·v) and its two exact rearrangements (v = p/m, m = p/v) —
//! driven through the built CLI binary against the SHIPPED stdlib. Each proves the
//! same invariant as the other formula libraries: a consumer states NO arithmetic;
//! it imports the grounded library, binds the physical quantities with `observe`,
//! and the engine applies the cited definition on the CPU — computing the EXACT
//! value and rendering the definition's citation + trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT: 2 * 5 = 10, and both
//! 10 / 2 = 5 and 10 / 5 = 2 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped momentum library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_momentum_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/momentum.adj")
        .canonicalize()
        .expect("shipped momentum.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mom_{tag}_{}", std::process::id()));
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

/// Copy the shipped library next to a consumer that imports it, so the CLI's
/// sandbox-checked relative import resolves.
fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_momentum_lib()).unwrap();
    std::fs::write(dir.join("momentum.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// momentum — the definition: the mass times the velocity.
// ---------------------------------------------------------------------------

#[test]
fn imports_momentum_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"momentum.adj\"\n\
         observe mass(2)\n\
         observe velocity(5)\n\
         ? momentum(mass, velocity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 2 * 5 = 10.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"momentum\"") && s.contains("\"value\":10"),
        "momentum(2, 5) = 10: {s}"
    );
    // … AND the HyperPhysics citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/mom.html"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// velocity — the same definition solved for v: the momentum divided by the mass,
// which INVERTS the momentum just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_velocity_as_momentum_over_mass_with_citation() {
    let dir = scratch("vel");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"momentum.adj\"\n\
         observe momentum(10)\n\
         observe mass(2)\n\
         ? velocity(momentum, mass)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 / 2 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"velocity\"") && s.contains("\"value\":5"),
        "velocity(10, 2) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/mom.html"),
        "velocity carries its HyperPhysics citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// mass — the same definition solved for m: the momentum divided by the velocity,
// the third exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_mass_as_momentum_over_velocity_with_citation() {
    let dir = scratch("mass");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"momentum.adj\"\n\
         observe momentum(10)\n\
         observe velocity(5)\n\
         ? mass(momentum, velocity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 / 5 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mass\"") && s.contains("\"value\":2"),
        "mass(10, 5) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/mom.html"),
        "mass carries its HyperPhysics citation: {s}"
    );
}
