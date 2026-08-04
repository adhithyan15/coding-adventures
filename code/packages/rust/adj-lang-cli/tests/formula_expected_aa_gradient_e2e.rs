//! End-to-end tests for the `clinical/expected-aa-gradient.adj` library — the age-predicted NORMAL
//! alveolar–arterial oxygen gradient (expected A-a gradient = (age + 10) / 4) and its one exact
//! rearrangement — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every
//! other formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! patient's age with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT
//! value (over exact rationals) and rendering the citation and trust tier in the `derived` section (the
//! auditable answer). The two formulas INVERT around the worked case age = 50:
//! (50 + 10) / 4 = 15 (expected gradient), 15 × 4 − 10 = 50 (age).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 10 and 4 and the intermediate 60, so a bare numeric
//! substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped expected-aa-gradient library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_eaag_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/expected-aa-gradient.adj")
        .canonicalize()
        .expect("shipped expected-aa-gradient.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_eaag_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_eaag_lib()).unwrap();
    std::fs::write(dir.join("expected-aa-gradient.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// expected_aa_gradient — the age-predicted upper limit of normal: (age + 10) / 4.
// ---------------------------------------------------------------------------

#[test]
fn imports_expected_aa_gradient_library_and_computes_it_with_citation() {
    let dir = scratch("eaag");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"expected-aa-gradient.adj\"\n\
         observe patient_age(50)\n\
         ? expected_aa_gradient(patient_age)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (50 + 10) / 4 = 60 / 4 = 15, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 10/4 constants and the 60 intermediate
    // cannot spuriously satisfy a bare "value":15.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"expected_aa_gradient\",\"value\":15"),
        "expected_aa_gradient(50) = 15: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// patient_age — the same equation solved for the age: expected × 4 − 10.
// ---------------------------------------------------------------------------

#[test]
fn computes_patient_age_from_expected_gradient_with_citation() {
    let dir = scratch("age");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"expected-aa-gradient.adj\"\n\
         observe expected_aa_gradient(15)\n\
         ? patient_age(expected_aa_gradient)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 × 4 − 10 = 60 − 10 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"patient_age\",\"value\":50"),
        "patient_age(15) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "patient_age carries its cited provenance: {s}"
    );
}
