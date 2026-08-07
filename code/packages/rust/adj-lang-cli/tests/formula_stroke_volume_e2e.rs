//! End-to-end tests for the `clinical/stroke-volume.adj` library — the definition of
//! stroke volume (SV = end-diastolic volume − end-systolic volume) and its two exact
//! rearrangements (EDV = SV + ESV, ESV = EDV − SV) — driven through the built CLI binary
//! against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured quantities with `observe`, and the engine applies the cited relation on the
//! CPU, computing the EXACT value and rendering the relation's citation and trust tier in
//! the `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case EDV = 120 mL, ESV = 50 mL: 120 − 50 = 70, and both 70 + 50 = 120 and
//! 120 − 70 = 50 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped stroke-volume library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_stroke_volume_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/stroke-volume.adj")
        .canonicalize()
        .expect("shipped stroke-volume.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_sv_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_stroke_volume_lib()).unwrap();
    std::fs::write(dir.join("stroke-volume.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// stroke_volume — the definition: the end-diastolic volume less the end-systolic volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_stroke_volume_library_and_computes_stroke_volume_with_citation() {
    let dir = scratch("sv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stroke-volume.adj\"\n\
         observe end_diastolic_volume(120)\n\
         observe end_systolic_volume(50)\n\
         ? stroke_volume(end_diastolic_volume, end_systolic_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 120 - 50 = 70.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"stroke_volume\"") && s.contains("\"value\":70"),
        "stroke_volume(120, 50) = 70: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is
    // auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// end_diastolic_volume — the same relation solved for EDV: SV + ESV, which INVERTS the
// stroke volume just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_end_diastolic_volume_from_stroke_volume_and_end_systolic_with_citation() {
    let dir = scratch("edv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stroke-volume.adj\"\n\
         observe stroke_volume(70)\n\
         observe end_systolic_volume(50)\n\
         ? end_diastolic_volume(stroke_volume, end_systolic_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 70 + 50 = 120, computed on the CPU.
    assert!(
        s.contains("\"name\":\"end_diastolic_volume\"") && s.contains("\"value\":120"),
        "end_diastolic_volume(70, 50) = 120: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "end_diastolic_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// end_systolic_volume — the same relation solved for ESV: EDV − SV, the third exact reading
// of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_end_systolic_volume_from_end_diastolic_and_stroke_volume_with_citation() {
    let dir = scratch("esv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stroke-volume.adj\"\n\
         observe end_diastolic_volume(120)\n\
         observe stroke_volume(70)\n\
         ? end_systolic_volume(end_diastolic_volume, stroke_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 120 - 70 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"end_systolic_volume\"") && s.contains("\"value\":50"),
        "end_systolic_volume(120, 70) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "end_systolic_volume carries its StatPearls citation: {s}"
    );
}
