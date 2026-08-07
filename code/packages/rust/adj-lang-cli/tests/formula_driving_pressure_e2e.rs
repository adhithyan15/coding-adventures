//! End-to-end tests for the `clinical/driving-pressure.adj` library — the driving-pressure relation
//! (driving pressure = plateau pressure − PEEP) and its two exact rearrangements — driven through the
//! built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the measured airway pressures
//! with `observe`, and the engine applies the cited definition on the CPU, computing the EXACT value
//! and rendering the citation and trust tier in the `derived` section (the auditable answer). The three
//! formulas INVERT around the worked case plateau pressure = 30, PEEP = 10: 30 − 10 = 20 (driving
//! pressure), 20 + 10 = 30 (plateau pressure), 30 − 20 = 10 (PEEP). The three asserted values (20, 30,
//! 10) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped driving-pressure library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_dp_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/driving-pressure.adj")
        .canonicalize()
        .expect("shipped driving-pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_dp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_dp_lib()).unwrap();
    std::fs::write(dir.join("driving-pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// driving_pressure — the relation: plateau pressure − PEEP.
// ---------------------------------------------------------------------------

#[test]
fn imports_driving_pressure_library_and_computes_it_with_citation() {
    let dir = scratch("dp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"driving-pressure.adj\"\n\
         observe plateau_pressure(30)\n\
         observe peep(10)\n\
         ? driving_pressure(plateau_pressure, peep)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 30 − 10 = 20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"driving_pressure\"") && s.contains("\"value\":20"),
        "driving_pressure(30, 10) = 20: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plateau_pressure — the same definition solved for the plateau pressure: ΔP + PEEP.
// ---------------------------------------------------------------------------

#[test]
fn computes_plateau_pressure_from_driving_pressure_with_citation() {
    let dir = scratch("pplat");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"driving-pressure.adj\"\n\
         observe driving_pressure(20)\n\
         observe peep(10)\n\
         ? plateau_pressure(driving_pressure, peep)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 + 10 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plateau_pressure\"") && s.contains("\"value\":30"),
        "plateau_pressure(20, 10) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plateau_pressure carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// peep — the same definition solved for the PEEP: plateau pressure − ΔP, the third reading of the one
// definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_peep_from_plateau_and_driving_pressure_with_citation() {
    let dir = scratch("peep");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"driving-pressure.adj\"\n\
         observe plateau_pressure(30)\n\
         observe driving_pressure(20)\n\
         ? peep(plateau_pressure, driving_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 − 20 = 10, computed on the CPU.
    assert!(
        s.contains("\"name\":\"peep\"") && s.contains("\"value\":10"),
        "peep(30, 20) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "peep carries its cited provenance: {s}"
    );
}
