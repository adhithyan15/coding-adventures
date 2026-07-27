//! End-to-end tests for the `clinical/mean-arterial-pressure.adj` library — the
//! definition of mean arterial pressure (MAP = diastolic + (systolic − diastolic)/3)
//! and its two exact rearrangements (SBP = 3·MAP − 2·DBP, DBP = (3·MAP − SBP)/2) —
//! driven through the built CLI binary against the SHIPPED stdlib.
//! Each proves the same invariant as the other formula libraries: a consumer states
//! NO arithmetic; it imports the grounded library, binds the pressures with
//! `observe`, and the engine applies the cited definition on the CPU, computing the
//! EXACT value and rendering the definition's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case SBP = 110 mmHg, DBP = 80 mmHg: 80 + (110 − 80)/3 = 90, and both
//! 3·90 − 2·80 = 110 and (3·90 − 110)/2 = 80 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped mean-arterial-pressure library, resolved from this
/// crate's manifest dir so the test is location-independent.
fn shipped_map_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/mean-arterial-pressure.adj")
        .canonicalize()
        .expect("shipped mean-arterial-pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_map_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_map_lib()).unwrap();
    std::fs::write(dir.join("mean-arterial-pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// mean_arterial_pressure — the definition: diastolic plus one-third of the pulse
// pressure (systolic minus diastolic).
// ---------------------------------------------------------------------------

#[test]
fn imports_map_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mean-arterial-pressure.adj\"\n\
         observe systolic_pressure(110)\n\
         observe diastolic_pressure(80)\n\
         ? mean_arterial_pressure(systolic_pressure, diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 80 + (110 − 80)/3 = 90.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"mean_arterial_pressure\"") && s.contains("\"value\":90"),
        "mean_arterial_pressure(110, 80) = 90: {s}"
    );
    // … AND the OpenStax/LibreTexts citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// systolic_pressure — the same definition solved for SBP: 3·MAP − 2·DBP, which
// INVERTS the mean arterial pressure just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_systolic_from_map_and_diastolic_with_citation() {
    let dir = scratch("sbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mean-arterial-pressure.adj\"\n\
         observe mean_arterial_pressure(90)\n\
         observe diastolic_pressure(80)\n\
         ? systolic_pressure(mean_arterial_pressure, diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3·90 − 2·80 = 110, computed on the CPU.
    assert!(
        s.contains("\"name\":\"systolic_pressure\"") && s.contains("\"value\":110"),
        "systolic_pressure(90, 80) = 110: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "systolic_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// diastolic_pressure — the same definition solved for DBP: (3·MAP − SBP)/2, the
// third exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_diastolic_from_map_and_systolic_with_citation() {
    let dir = scratch("dbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mean-arterial-pressure.adj\"\n\
         observe mean_arterial_pressure(90)\n\
         observe systolic_pressure(110)\n\
         ? diastolic_pressure(mean_arterial_pressure, systolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (3·90 − 110)/2 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"diastolic_pressure\"") && s.contains("\"value\":80"),
        "diastolic_pressure(90, 110) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "diastolic_pressure carries its LibreTexts citation: {s}"
    );
}
