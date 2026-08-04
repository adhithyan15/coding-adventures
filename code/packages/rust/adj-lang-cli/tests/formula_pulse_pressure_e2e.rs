//! End-to-end tests for the `clinical/pulse-pressure.adj` library — the definition of
//! pulse pressure (PP = SBP − DBP) and its two exact rearrangements (SBP = PP + DBP,
//! DBP = SBP − PP) — driven through the built CLI binary against the SHIPPED stdlib.
//! Each proves the same invariant as the other formula libraries: a consumer states NO
//! arithmetic; it imports the grounded library, binds the pressures with `observe`, and
//! the engine applies the cited definition on the CPU, computing the EXACT value and
//! rendering the definition's citation and trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT around the worked case SBP = 120 mmHg,
//! DBP = 80 mmHg: 120 − 80 = 40, and both 40 + 80 = 120 and 120 − 40 = 80 recover the
//! inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped pulse-pressure library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_pulse_pressure_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/pulse-pressure.adj")
        .canonicalize()
        .expect("shipped pulse-pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_pulse_pressure_lib()).unwrap();
    std::fs::write(dir.join("pulse-pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// pulse_pressure — the definition: the systolic minus the diastolic.
// ---------------------------------------------------------------------------

#[test]
fn imports_pulse_pressure_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pulse-pressure.adj\"\n\
         observe systolic_pressure(120)\n\
         observe diastolic_pressure(80)\n\
         ? pulse_pressure(systolic_pressure, diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 120 − 80 = 40.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"pulse_pressure\"") && s.contains("\"value\":40"),
        "pulse_pressure(120, 80) = 40: {s}"
    );
    // … AND the OpenStax/LibreTexts citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// systolic_pressure — the same definition solved for SBP: PP + DBP, which INVERTS the
// pulse pressure just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_systolic_as_pulse_pressure_plus_diastolic_with_citation() {
    let dir = scratch("sbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pulse-pressure.adj\"\n\
         observe pulse_pressure(40)\n\
         observe diastolic_pressure(80)\n\
         ? systolic_pressure(pulse_pressure, diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 40 + 80 = 120, computed on the CPU.
    assert!(
        s.contains("\"name\":\"systolic_pressure\"") && s.contains("\"value\":120"),
        "systolic_pressure(40, 80) = 120: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "systolic_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// diastolic_pressure — the same definition solved for DBP: SBP − PP, the third exact
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_diastolic_as_systolic_minus_pulse_pressure_with_citation() {
    let dir = scratch("dbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pulse-pressure.adj\"\n\
         observe systolic_pressure(120)\n\
         observe pulse_pressure(40)\n\
         ? diastolic_pressure(systolic_pressure, pulse_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 120 − 40 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"diastolic_pressure\"") && s.contains("\"value\":80"),
        "diastolic_pressure(120, 40) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "diastolic_pressure carries its LibreTexts citation: {s}"
    );
}
