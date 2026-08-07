//! End-to-end tests for the `physics/energy-work.adj` library — the foundational
//! mechanics laws (work and kinetic energy) — driven through the built CLI binary
//! against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library,
//! binds the physical quantities with `observe`, and the engine applies the cited
//! law on the CPU — computing the EXACT value and rendering the applied law's
//! citation + trust tier in the `derived` section (the auditable answer).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped physics library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_energy_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/energy-work.adj")
        .canonicalize()
        .expect("shipped energy-work.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_energy_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_energy_lib()).unwrap();
    std::fs::write(dir.join("energy-work.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// work — the product of a force and the distance through which it acts.
// ---------------------------------------------------------------------------

#[test]
fn imports_energy_library_and_computes_work_with_citation() {
    let dir = scratch("work");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-work.adj\"\n\
         observe force(10)\n\
         observe distance(3)\n\
         ? work(force, distance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied law's result: 10 * 3 = 30.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"work\"") && s.contains("\"value\":30"),
        "work(10, 3) = 30: {s}"
    );
    // … AND the NASA citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("grc.nasa.gov/www/k-12/airplane/work.html"),
        "applied law carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// kinetic_energy — half the mass times the square of the speed (a distinct law
// and a distinct source).
// ---------------------------------------------------------------------------

#[test]
fn computes_kinetic_energy_as_half_mass_velocity_squared_with_citation() {
    let dir = scratch("ke");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-work.adj\"\n\
         observe mass(2)\n\
         observe velocity(3)\n\
         ? kinetic_energy(mass, velocity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.5 * 2 * 3 * 3 = 9, computed on the CPU.
    assert!(
        s.contains("\"name\":\"kinetic_energy\"") && s.contains("\"value\":9"),
        "kinetic_energy(2, 3) = 9: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("hyperphysics.gsu.edu/hbase/ke.html"),
        "kinetic energy carries its HyperPhysics citation: {s}"
    );
}
