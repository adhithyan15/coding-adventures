//! End-to-end tests for the `clinical/cardiac-output.adj` library — the definition
//! of cardiac output (CO = HR·SV) and its two exact rearrangements (HR = CO/SV,
//! SV = CO/HR) — driven through the built CLI binary against the SHIPPED stdlib.
//! Each proves the same invariant as the other formula libraries: a consumer states
//! NO arithmetic; it imports the grounded library, binds the physiological
//! quantities with `observe`, and the engine applies the cited definition on the
//! CPU, computing the EXACT value and rendering the definition's citation and trust
//! tier in the `derived` section (the auditable answer). The three formulas INVERT
//! around the worked case HR = 75 bpm, SV = 70 mL: 75 * 70 = 5250, and both
//! 5250 / 70 = 75 and 5250 / 75 = 70 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped cardiac-output library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_cardiac_output_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/cardiac-output.adj")
        .canonicalize()
        .expect("shipped cardiac-output.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_co_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cardiac_output_lib()).unwrap();
    std::fs::write(dir.join("cardiac-output.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// cardiac_output — the definition: the heart rate times the stroke volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_cardiac_output_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-output.adj\"\n\
         observe heart_rate(75)\n\
         observe stroke_volume(70)\n\
         ? cardiac_output(heart_rate, stroke_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 75 * 70 = 5250.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"cardiac_output\"") && s.contains("\"value\":5250"),
        "cardiac_output(75, 70) = 5250: {s}"
    );
    // … AND the OpenStax/LibreTexts citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// heart_rate — the same definition solved for HR: the cardiac output divided by the
// stroke volume, which INVERTS the cardiac output just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_heart_rate_as_cardiac_output_over_stroke_volume_with_citation() {
    let dir = scratch("hr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-output.adj\"\n\
         observe cardiac_output(5250)\n\
         observe stroke_volume(70)\n\
         ? heart_rate(cardiac_output, stroke_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 5250 / 70 = 75, computed on the CPU.
    assert!(
        s.contains("\"name\":\"heart_rate\"") && s.contains("\"value\":75"),
        "heart_rate(5250, 70) = 75: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "heart_rate carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// stroke_volume — the same definition solved for SV: the cardiac output divided by
// the heart rate, the third exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_stroke_volume_as_cardiac_output_over_heart_rate_with_citation() {
    let dir = scratch("sv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-output.adj\"\n\
         observe cardiac_output(5250)\n\
         observe heart_rate(75)\n\
         ? stroke_volume(cardiac_output, heart_rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 5250 / 75 = 70, computed on the CPU.
    assert!(
        s.contains("\"name\":\"stroke_volume\"") && s.contains("\"value\":70"),
        "stroke_volume(5250, 75) = 70: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "stroke_volume carries its LibreTexts citation: {s}"
    );
}
