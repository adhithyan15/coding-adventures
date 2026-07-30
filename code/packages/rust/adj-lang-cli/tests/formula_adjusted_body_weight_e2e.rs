//! End-to-end tests for the `clinical/adjusted-body-weight.adj` library — the obese-dosing adjusted
//! body weight (AdjBW = ideal body weight + 0.4 × (actual body weight − ideal body weight)) and its two
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the ideal and actual body weights with `observe`, and the engine applies the cited
//! formula on the CPU, computing the EXACT value (over exact rationals — 0.4 = 2/5, 0.6 = 3/5) and
//! rendering the citation and trust tier in the `derived` section (the auditable answer). The three
//! formulas INVERT around the worked case ideal body weight = 70, actual body weight = 100:
//! 70 + 0.4 × (100 − 70) = 70 + 12 = 82 (adjusted), (82 − 70)/0.4 + 70 = 100 (actual),
//! (82 − 0.4 × 100)/0.6 = 70 (ideal). The three asserted values (82, 100, 70) are distinct, none a
//! colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped adjusted-body-weight library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_abw_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/adjusted-body-weight.adj")
        .canonicalize()
        .expect("shipped adjusted-body-weight.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_abw_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_abw_lib()).unwrap();
    std::fs::write(dir.join("adjusted-body-weight.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// adjusted_body_weight — the adjustment: IBW + 0.4 × (actual − IBW).
// ---------------------------------------------------------------------------

#[test]
fn imports_abw_library_and_computes_it_with_citation() {
    let dir = scratch("abw");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adjusted-body-weight.adj\"\n\
         observe ideal_body_weight(70)\n\
         observe actual_body_weight(100)\n\
         ? adjusted_body_weight(ideal_body_weight, actual_body_weight)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 70 + 0.4 × (100 − 70) =
    // 70 + 12 = 82, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"adjusted_body_weight\"") && s.contains("\"value\":82"),
        "adjusted_body_weight(70, 100) = 82: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// actual_body_weight — the same equation solved for the actual body weight: (adjusted − IBW)/0.4 + IBW.
// ---------------------------------------------------------------------------

#[test]
fn computes_actual_body_weight_from_adjusted_with_citation() {
    let dir = scratch("actual");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adjusted-body-weight.adj\"\n\
         observe adjusted_body_weight(82)\n\
         observe ideal_body_weight(70)\n\
         ? actual_body_weight(adjusted_body_weight, ideal_body_weight)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (82 − 70) / 0.4 + 70 = 30 + 70 = 100, computed on the CPU.
    assert!(
        s.contains("\"name\":\"actual_body_weight\"") && s.contains("\"value\":100"),
        "actual_body_weight(82, 70) = 100: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "actual_body_weight carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// ideal_body_weight — the same equation solved for the ideal body weight: (adjusted − 0.4 × actual)/0.6,
// the third reading of the one adjustment.
// ---------------------------------------------------------------------------

#[test]
fn computes_ideal_body_weight_from_adjusted_with_citation() {
    let dir = scratch("ideal");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adjusted-body-weight.adj\"\n\
         observe adjusted_body_weight(82)\n\
         observe actual_body_weight(100)\n\
         ? ideal_body_weight(adjusted_body_weight, actual_body_weight)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (82 − 0.4 × 100) / 0.6 = (82 − 40) / 0.6 = 42 / 0.6 = 70, computed on the CPU.
    assert!(
        s.contains("\"name\":\"ideal_body_weight\"") && s.contains("\"value\":70"),
        "ideal_body_weight(82, 100) = 70: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "ideal_body_weight carries its cited provenance: {s}"
    );
}
