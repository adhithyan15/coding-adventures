//! End-to-end tests for the `chemistry/combined-gas-law.adj` library — the combined gas
//! law (P1V1/T1 = P2V2/T2, the conserved quantity P*V/T for a fixed amount of gas) and
//! its SIX exact readings (solve for any one of the six state variables from the other
//! five) — driven through the built CLI binary against the SHIPPED stdlib. Each proves
//! the same invariant as the other formula libraries: a consumer states NO arithmetic;
//! it imports the grounded library, binds the measured quantities with `observe`, and the
//! engine applies the cited relation on the CPU — computing the EXACT value and rendering
//! the relation's citation and trust tier in the `derived` section (the auditable
//! answer). The six readings INVERT around the worked case P1 = 2, V1 = 3, T1 = 6,
//! P2 = 4, V2 = 3, T2 = 12 (P1V1/T1 = 2*3/6 = 1 = 4*3/12 = P2V2/T2).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped combined-gas-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_combined_gas_law_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/combined-gas-law.adj")
        .canonicalize()
        .expect("shipped combined-gas-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_combgas_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_combined_gas_law_lib()).unwrap();
    std::fs::write(dir.join("combined-gas-law.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// final_pressure — the pressure a fixed amount of gas reaches at a new volume and
// absolute temperature (P2 = P1V1T2 / (T1V2)).
// ---------------------------------------------------------------------------

#[test]
fn imports_combined_gas_law_library_and_computes_final_pressure_with_citation() {
    let dir = scratch("p2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe initial_pressure(2)\n\
         observe initial_volume(3)\n\
         observe initial_temperature(6)\n\
         observe final_volume(3)\n\
         observe final_temperature(12)\n\
         ? final_pressure(initial_pressure, initial_volume, initial_temperature, final_volume, final_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2*3*12/(6*3) = 4.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"final_pressure\"") && s.contains("\"value\":4"),
        "final_pressure(2,3,6,3,12) = 4: {s}"
    );
    // … AND the LibreTexts citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// final_volume — the volume at a new pressure and temperature (V2 = P1V1T2 / (T1P2)).
// ---------------------------------------------------------------------------

#[test]
fn computes_final_volume_with_citation() {
    let dir = scratch("v2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe initial_pressure(2)\n\
         observe initial_volume(3)\n\
         observe initial_temperature(6)\n\
         observe final_pressure(4)\n\
         observe final_temperature(12)\n\
         ? final_volume(initial_pressure, initial_volume, initial_temperature, final_pressure, final_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2*3*12/(6*4) = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"final_volume\"") && s.contains("\"value\":3"),
        "final_volume(2,3,6,4,12) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "final_volume carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// final_temperature — the absolute temperature at a new pressure and volume
// (T2 = P2V2T1 / (P1V1)).
// ---------------------------------------------------------------------------

#[test]
fn computes_final_temperature_with_citation() {
    let dir = scratch("t2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe final_pressure(4)\n\
         observe final_volume(3)\n\
         observe initial_pressure(2)\n\
         observe initial_volume(3)\n\
         observe initial_temperature(6)\n\
         ? final_temperature(final_pressure, final_volume, initial_pressure, initial_volume, initial_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4*3*6/(2*3) = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"final_temperature\"") && s.contains("\"value\":12"),
        "final_temperature(4,3,2,3,6) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "final_temperature carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_pressure — the pressure the gas started at (P1 = P2V2T1 / (T2V1)).
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_pressure_with_citation() {
    let dir = scratch("p1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe final_pressure(4)\n\
         observe final_volume(3)\n\
         observe initial_temperature(6)\n\
         observe final_temperature(12)\n\
         observe initial_volume(3)\n\
         ? initial_pressure(final_pressure, final_volume, initial_temperature, final_temperature, initial_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4*3*6/(12*3) = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_pressure\"") && s.contains("\"value\":2"),
        "initial_pressure(4,3,6,12,3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_volume — the volume the gas started at (V1 = P2V2T1 / (T2P1)).
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_volume_with_citation() {
    let dir = scratch("v1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe final_pressure(4)\n\
         observe final_volume(3)\n\
         observe initial_temperature(6)\n\
         observe final_temperature(12)\n\
         observe initial_pressure(2)\n\
         ? initial_volume(final_pressure, final_volume, initial_temperature, final_temperature, initial_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4*3*6/(12*2) = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_volume\"") && s.contains("\"value\":3"),
        "initial_volume(4,3,6,12,2) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_volume carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_temperature — the absolute temperature the gas started at
// (T1 = P1V1T2 / (P2V2)), the sixth exact reading of the one conserved quantity.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_temperature_with_citation() {
    let dir = scratch("t1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"combined-gas-law.adj\"\n\
         observe initial_pressure(2)\n\
         observe initial_volume(3)\n\
         observe final_pressure(4)\n\
         observe final_volume(3)\n\
         observe final_temperature(12)\n\
         ? initial_temperature(initial_pressure, initial_volume, final_pressure, final_volume, final_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2*3*12/(4*3) = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_temperature\"") && s.contains("\"value\":6"),
        "initial_temperature(2,3,4,3,12) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_temperature carries its LibreTexts citation: {s}"
    );
}
