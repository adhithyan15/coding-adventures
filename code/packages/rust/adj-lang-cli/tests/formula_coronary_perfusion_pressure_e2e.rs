//! End-to-end tests for the `clinical/coronary-perfusion-pressure.adj` library — the definition
//! of coronary perfusion pressure (CPP = aortic diastolic pressure − left ventricular
//! end-diastolic pressure) and its two exact rearrangements — driven through the built CLI binary
//! against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the measured pressures with
//! `observe`, and the engine applies the cited relation on the CPU, computing the EXACT value and
//! rendering the relation's citation and trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT around the worked case ADP = 80 mmHg, LVEDP = 12 mmHg:
//! 80 − 12 = 68, 68 + 12 = 80, and 80 − 68 = 12. This is the cardiology counterpart of the
//! shipped cerebral-perfusion-pressure.adj (CPP = MAP − ICP).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped coronary-perfusion-pressure library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_cpp_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/coronary-perfusion-pressure.adj")
        .canonicalize()
        .expect("shipped coronary-perfusion-pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_coronarycpp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cpp_lib()).unwrap();
    std::fs::write(dir.join("coronary-perfusion-pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// coronary_perfusion_pressure — the definition: aortic diastolic pressure minus LVEDP.
// ---------------------------------------------------------------------------

#[test]
fn imports_coronary_perfusion_pressure_library_and_computes_it_with_citation() {
    let dir = scratch("cpp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coronary-perfusion-pressure.adj\"\n\
         observe aortic_diastolic_pressure(80)\n\
         observe left_ventricular_end_diastolic_pressure(12)\n\
         ? coronary_perfusion_pressure(aortic_diastolic_pressure, left_ventricular_end_diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 80 − 12 = 68.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"coronary_perfusion_pressure\"") && s.contains("\"value\":68"),
        "coronary_perfusion_pressure(80, 12) = 68: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// aortic_diastolic_pressure — the same relation solved for ADP: CPP + LVEDP.
// ---------------------------------------------------------------------------

#[test]
fn computes_aortic_diastolic_pressure_from_cpp_and_lvedp_with_citation() {
    let dir = scratch("adp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coronary-perfusion-pressure.adj\"\n\
         observe coronary_perfusion_pressure(68)\n\
         observe left_ventricular_end_diastolic_pressure(12)\n\
         ? aortic_diastolic_pressure(coronary_perfusion_pressure, left_ventricular_end_diastolic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 68 + 12 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"aortic_diastolic_pressure\"") && s.contains("\"value\":80"),
        "aortic_diastolic_pressure(68, 12) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "aortic_diastolic_pressure carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// left_ventricular_end_diastolic_pressure — the same relation solved for LVEDP: ADP − CPP, the
// third reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_lvedp_from_adp_and_cpp_with_citation() {
    let dir = scratch("lvedp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coronary-perfusion-pressure.adj\"\n\
         observe aortic_diastolic_pressure(80)\n\
         observe coronary_perfusion_pressure(68)\n\
         ? left_ventricular_end_diastolic_pressure(aortic_diastolic_pressure, coronary_perfusion_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 80 − 68 = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"left_ventricular_end_diastolic_pressure\"") && s.contains("\"value\":12"),
        "left_ventricular_end_diastolic_pressure(80, 68) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "left_ventricular_end_diastolic_pressure carries its StatPearls citation: {s}"
    );
}
