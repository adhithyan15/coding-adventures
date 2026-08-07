//! End-to-end tests for the `clinical/ejection-fraction.adj` library — the definition of the
//! (left-ventricular) ejection fraction (ejection fraction = stroke volume / end-diastolic
//! volume) and its two exact rearrangements (stroke volume = EF × EDV, end-diastolic
//! volume = SV / EF) — driven through the built CLI binary against the SHIPPED stdlib. Each
//! proves the same invariant as the other formula libraries: a consumer states NO arithmetic;
//! it imports the grounded library, binds the measured quantities with `observe`, and the
//! engine applies the cited relation on the CPU, computing the EXACT value and rendering the
//! relation's citation and trust tier in the `derived` section (the auditable answer). The
//! three formulas INVERT around the worked case SV = 3 mL, EDV = 6 mL: 3 / 6 = 0.5, and both
//! 0.5 × 6 = 3 and 3 / 0.5 = 6 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped ejection-fraction library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_ef_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/ejection-fraction.adj")
        .canonicalize()
        .expect("shipped ejection-fraction.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ef_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ef_lib()).unwrap();
    std::fs::write(dir.join("ejection-fraction.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// ejection_fraction — the definition: the stroke volume as a fraction of the end-diastolic
// volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_ef_library_and_computes_ejection_fraction_with_citation() {
    let dir = scratch("ef");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ejection-fraction.adj\"\n\
         observe stroke_volume(3)\n\
         observe end_diastolic_volume(6)\n\
         ? ejection_fraction(stroke_volume, end_diastolic_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 3 / 6 = 0.5.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"ejection_fraction\"") && s.contains("\"value\":0.5"),
        "ejection_fraction(3, 6) = 0.5: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// stroke_volume — the same relation solved for SV: EF × EDV, which INVERTS the ejection
// fraction just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_stroke_volume_from_ef_and_edv_with_citation() {
    let dir = scratch("sv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ejection-fraction.adj\"\n\
         observe ejection_fraction(0.5)\n\
         observe end_diastolic_volume(6)\n\
         ? stroke_volume(ejection_fraction, end_diastolic_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.5 * 6 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"stroke_volume\"") && s.contains("\"value\":3"),
        "stroke_volume(0.5, 6) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "stroke_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// end_diastolic_volume — the same relation solved for EDV: SV / EF, the third exact reading
// of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_end_diastolic_volume_from_sv_and_ef_with_citation() {
    let dir = scratch("edv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ejection-fraction.adj\"\n\
         observe stroke_volume(3)\n\
         observe ejection_fraction(0.5)\n\
         ? end_diastolic_volume(stroke_volume, ejection_fraction)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 / 0.5 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"end_diastolic_volume\"") && s.contains("\"value\":6"),
        "end_diastolic_volume(3, 0.5) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "end_diastolic_volume carries its StatPearls citation: {s}"
    );
}
