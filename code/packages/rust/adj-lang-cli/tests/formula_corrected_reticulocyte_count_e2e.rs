//! End-to-end tests for the `clinical/corrected-reticulocyte-count.adj` library — the corrected
//! reticulocyte count (corrected retic = reticulocyte % × (patient Hct / normal Hct)) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as
//! every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! reticulocyte percentage, the patient haematocrit, and the normal haematocrit with `observe`, and the
//! engine applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and
//! rendering the citation and trust tier in the `derived` section (the auditable answer). The three formulas
//! INVERT around the worked case retic = 4, patient Hct = 20, normal Hct = 40: 4 × (20 / 40) = 2 (corrected),
//! 2 × 40 / 20 = 4 (retic %), 2 × 40 / 4 = 20 (patient Hct).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the result 2 is a leading-digit prefix of the patient-Hct 20 and the normal-Hct 40, so a
//! bare `"value":2` substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped corrected-reticulocyte-count library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_crc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/corrected-reticulocyte-count.adj")
        .canonicalize()
        .expect("shipped corrected-reticulocyte-count.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_crc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_crc_lib()).unwrap();
    std::fs::write(dir.join("corrected-reticulocyte-count.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// corrected_reticulocyte_count — the corrected percentage: retic × (patient Hct / normal Hct).
// ---------------------------------------------------------------------------

#[test]
fn imports_corrected_reticulocyte_library_and_computes_it_with_citation() {
    let dir = scratch("crc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-reticulocyte-count.adj\"\n\
         observe reticulocyte_percent(4)\n\
         observe patient_hematocrit(20)\n\
         observe normal_hematocrit(40)\n\
         ? corrected_reticulocyte_count(reticulocyte_percent, patient_hematocrit, normal_hematocrit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 4 × (20 / 40) = 2, computed EXACTLY
    // over rationals. Match the adjacent name/value pair so the 20/40 inputs cannot spuriously satisfy a
    // bare "value":2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"corrected_reticulocyte_count\",\"value\":2"),
        "corrected_reticulocyte_count(4, 20, 40) = 2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// reticulocyte_percent — the same equation solved for the raw retic %: corrected × normal Hct / patient Hct.
// ---------------------------------------------------------------------------

#[test]
fn computes_reticulocyte_percent_from_corrected_with_citation() {
    let dir = scratch("retic");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-reticulocyte-count.adj\"\n\
         observe corrected_reticulocyte_count(2)\n\
         observe patient_hematocrit(20)\n\
         observe normal_hematocrit(40)\n\
         ? reticulocyte_percent(corrected_reticulocyte_count, patient_hematocrit, normal_hematocrit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 40 / 20 = 80 / 20 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"reticulocyte_percent\",\"value\":4"),
        "reticulocyte_percent(2, 20, 40) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "reticulocyte_percent carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// patient_hematocrit — the same equation solved for the patient Hct: corrected × normal Hct / retic %, the
// third reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_patient_hematocrit_from_corrected_with_citation() {
    let dir = scratch("hct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-reticulocyte-count.adj\"\n\
         observe corrected_reticulocyte_count(2)\n\
         observe reticulocyte_percent(4)\n\
         observe normal_hematocrit(40)\n\
         ? patient_hematocrit(corrected_reticulocyte_count, reticulocyte_percent, normal_hematocrit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 40 / 4 = 80 / 4 = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"patient_hematocrit\",\"value\":20"),
        "patient_hematocrit(2, 4, 40) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "patient_hematocrit carries its cited provenance: {s}"
    );
}
