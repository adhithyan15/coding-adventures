//! End-to-end tests for the `physics/density.adj` library — the definition of
//! density (ρ = m/V) and its exact rearrangement (m = ρ·V) — driven through the
//! built CLI binary against the SHIPPED stdlib. Each proves the same invariant as
//! the other formula libraries: a consumer states NO arithmetic; it imports the
//! grounded library, binds the physical quantities with `observe`, and the engine
//! applies the cited definition on the CPU — computing the EXACT value and rendering
//! the definition's citation + trust tier in the `derived` section (the auditable
//! answer). The two formulas INVERT: 60 / 3 = 20, and 20 * 3 = 60.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped density library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_density_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/density.adj")
        .canonicalize()
        .expect("shipped density.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_dens_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_density_lib()).unwrap();
    std::fs::write(dir.join("density.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// density — the definition: the mass divided by the volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_density_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"density.adj\"\n\
         observe mass(60)\n\
         observe volume(3)\n\
         ? density(mass, volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 60 / 3 = 20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"density\"") && s.contains("\"value\":20"),
        "density(60, 3) = 20: {s}"
    );
    // … AND the HyperPhysics citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/dens.html"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mass — the same definition solved for m: the density times the volume, which
// INVERTS the density just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_mass_as_density_times_volume_with_citation() {
    let dir = scratch("mass");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"density.adj\"\n\
         observe density(20)\n\
         observe volume(3)\n\
         ? mass(density, volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 * 3 = 60, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mass\"") && s.contains("\"value\":60"),
        "mass(20, 3) = 60: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/dens.html"),
        "mass carries its HyperPhysics citation: {s}"
    );
}
