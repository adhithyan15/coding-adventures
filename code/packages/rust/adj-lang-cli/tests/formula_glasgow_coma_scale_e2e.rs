//! End-to-end tests for the `clinical/glasgow-coma-scale.adj` library — the definition of the
//! total Glasgow Coma Scale score (GCS = eye + verbal + motor) and its three exact
//! rearrangements (each element score = the total minus the other two) — driven through the
//! built CLI binary against the SHIPPED stdlib. This is the first n-ary (three-input) clinical
//! inverter: the same invariant as every other formula library, extended one dimension. A
//! consumer states NO arithmetic; it imports the grounded library, binds the measured element
//! scores with `observe`, and the engine applies the cited relation on the CPU, computing the
//! EXACT value and rendering the relation's citation and trust tier in the `derived` section
//! (the auditable answer). The four formulas INVERT around the worked case E = 4, V = 5,
//! M = 6: 4 + 5 + 6 = 15, and 15 minus any two element scores recovers the third.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped glasgow-coma-scale library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_gcs_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/glasgow-coma-scale.adj")
        .canonicalize()
        .expect("shipped glasgow-coma-scale.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_gcs_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_gcs_lib()).unwrap();
    std::fs::write(dir.join("glasgow-coma-scale.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// glasgow_coma_scale — the definition: the sum of the three element scores.
// ---------------------------------------------------------------------------

#[test]
fn imports_gcs_library_and_computes_total_with_citation() {
    let dir = scratch("total");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glasgow-coma-scale.adj\"\n\
         observe eye_response(4)\n\
         observe verbal_response(5)\n\
         observe motor_response(6)\n\
         ? glasgow_coma_scale(eye_response, verbal_response, motor_response)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 4 + 5 + 6 = 15.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"glasgow_coma_scale\"") && s.contains("\"value\":15"),
        "glasgow_coma_scale(4, 5, 6) = 15: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// eye_response — the same relation solved for E: GCS − V − M, which INVERTS the total.
// ---------------------------------------------------------------------------

#[test]
fn computes_eye_response_from_total_and_others_with_citation() {
    let dir = scratch("eye");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glasgow-coma-scale.adj\"\n\
         observe glasgow_coma_scale(15)\n\
         observe verbal_response(5)\n\
         observe motor_response(6)\n\
         ? eye_response(glasgow_coma_scale, verbal_response, motor_response)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 - 5 - 6 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"eye_response\"") && s.contains("\"value\":4"),
        "eye_response(15, 5, 6) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "eye_response carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// verbal_response — the same relation solved for V: GCS − E − M.
// ---------------------------------------------------------------------------

#[test]
fn computes_verbal_response_from_total_and_others_with_citation() {
    let dir = scratch("verbal");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glasgow-coma-scale.adj\"\n\
         observe glasgow_coma_scale(15)\n\
         observe eye_response(4)\n\
         observe motor_response(6)\n\
         ? verbal_response(glasgow_coma_scale, eye_response, motor_response)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 - 4 - 6 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"verbal_response\"") && s.contains("\"value\":5"),
        "verbal_response(15, 4, 6) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "verbal_response carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// motor_response — the same relation solved for M: GCS − E − V, the fourth reading of the one
// definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_motor_response_from_total_and_others_with_citation() {
    let dir = scratch("motor");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glasgow-coma-scale.adj\"\n\
         observe glasgow_coma_scale(15)\n\
         observe eye_response(4)\n\
         observe verbal_response(5)\n\
         ? motor_response(glasgow_coma_scale, eye_response, verbal_response)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 - 4 - 5 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"motor_response\"") && s.contains("\"value\":6"),
        "motor_response(15, 4, 5) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "motor_response carries its StatPearls citation: {s}"
    );
}
