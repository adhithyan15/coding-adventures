//! End-to-end tests for the `physics/pressure.adj` library — the definition of
//! pressure (P = F/A) and its exact rearrangement (F = P·A) — driven through the
//! built CLI binary against the SHIPPED stdlib. Each proves the same invariant as
//! the other formula libraries: a consumer states NO arithmetic; it imports the
//! grounded library, binds the mechanical quantities with `observe`, and the engine
//! applies the cited definition on the CPU — computing the EXACT value and rendering
//! the definition's citation + trust tier in the `derived` section (the auditable
//! answer). The two formulas INVERT: 50 N / 2 m² = 25 Pa, and 25 Pa · 2 m² = 50 N.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped pressure library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_pressure_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/pressure.adj")
        .canonicalize()
        .expect("shipped pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_press_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_pressure_lib()).unwrap();
    std::fs::write(dir.join("pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// pressure — the definition: the force divided by the area.
// ---------------------------------------------------------------------------

#[test]
fn imports_pressure_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pressure.adj\"\n\
         observe force(50)\n\
         observe area(2)\n\
         ? pressure(force, area)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 50 / 2 = 25.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"pressure\"") && s.contains("\"value\":25"),
        "pressure(50, 2) = 25: {s}"
    );
    // … AND the HyperPhysics citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/press.html"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// force — the same definition solved for F: the pressure times the area, which
// INVERTS the pressure just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_force_as_pressure_times_area_with_citation() {
    let dir = scratch("force");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pressure.adj\"\n\
         observe pressure(25)\n\
         observe area(2)\n\
         ? force(pressure, area)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 25 * 2 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"force\"") && s.contains("\"value\":50"),
        "force(25, 2) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/press.html"),
        "force carries its HyperPhysics citation: {s}"
    );
}
