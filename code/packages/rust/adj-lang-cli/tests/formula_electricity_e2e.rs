//! End-to-end tests for the `physics/electricity.adj` library — the foundational
//! circuit laws (Ohm's law and electric power) — driven through the built CLI
//! binary against the SHIPPED stdlib. Each proves the same invariant as the other
//! formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds the electrical quantities with `observe`, and the engine applies
//! the cited law on the CPU — computing the EXACT value and rendering the applied
//! law's citation + trust tier in the `derived` section (the auditable answer).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped electricity library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_electricity_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/electricity.adj")
        .canonicalize()
        .expect("shipped electricity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_elec_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_electricity_lib()).unwrap();
    std::fs::write(dir.join("electricity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// voltage — Ohm's law: the current times the resistance.
// ---------------------------------------------------------------------------

#[test]
fn imports_electricity_library_and_computes_ohms_law_with_citation() {
    let dir = scratch("ohm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electricity.adj\"\n\
         observe current(2)\n\
         observe resistance(3)\n\
         ? voltage(current, resistance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied law's result: 2 * 3 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"voltage\"") && s.contains("\"value\":6"),
        "voltage(2, 3) = 6: {s}"
    );
    // … AND the HyperPhysics citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/electric/ohmlaw.html"),
        "applied law carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// power — electric power: the voltage times the current (a distinct law and a
// distinct source), which COMPOSES with the voltage Ohm's law just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_electric_power_as_voltage_times_current_with_citation() {
    let dir = scratch("power");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electricity.adj\"\n\
         observe voltage(6)\n\
         observe current(2)\n\
         ? power(voltage, current)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 * 2 = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"power\"") && s.contains("\"value\":12"),
        "power(6, 2) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("hyperphysics.gsu.edu/hbase/electric/elepow.html"),
        "electric power carries its HyperPhysics citation: {s}"
    );
}
