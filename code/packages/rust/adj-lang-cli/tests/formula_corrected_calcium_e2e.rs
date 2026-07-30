//! End-to-end tests for the `clinical/corrected-calcium.adj` library — the albumin-corrected-calcium
//! correction (corrected calcium = measured calcium + 0.8 × (4 − albumin)) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant
//! as every other formula library: a consumer states NO arithmetic; it imports the grounded library,
//! binds the measured calcium and albumin with `observe`, and the engine applies the cited correction
//! on the CPU, computing the EXACT value (over exact rationals — 0.8 = 4/5, so 9.6 is exact, not a
//! rounded float) and rendering the citation and trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT around the worked case measured calcium = 8, albumin = 2:
//! 8 + 0.8 × (4 − 2) = 9.6 (corrected), 9.6 − 0.8 × (4 − 2) = 8 (measured), 4 − (9.6 − 8) / 0.8 = 2
//! (albumin). The three asserted values (9.6, 8, 2) are distinct, none a colon-anchored prefix of
//! another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped corrected-calcium library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_cc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/corrected-calcium.adj")
        .canonicalize()
        .expect("shipped corrected-calcium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cc_lib()).unwrap();
    std::fs::write(dir.join("corrected-calcium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// corrected_calcium — the correction: measured calcium + 0.8 × (4 − albumin).
// ---------------------------------------------------------------------------

#[test]
fn imports_corrected_calcium_library_and_computes_it_with_citation() {
    let dir = scratch("cc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-calcium.adj\"\n\
         observe measured_calcium(8)\n\
         observe albumin(2)\n\
         ? corrected_calcium(measured_calcium, albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied correction's result: 8 + 0.8 × (4 − 2) = 9.6,
    // computed EXACTLY over rationals (0.8 = 4/5), not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"corrected_calcium\"") && s.contains("\"value\":9.6"),
        "corrected_calcium(8, 2) = 9.6: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// measured_calcium — the same correction solved for the measured calcium: corrected − 0.8 × (4 − albumin).
// ---------------------------------------------------------------------------

#[test]
fn computes_measured_calcium_from_corrected_with_citation() {
    let dir = scratch("meas");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-calcium.adj\"\n\
         observe corrected_calcium(9.6)\n\
         observe albumin(2)\n\
         ? measured_calcium(corrected_calcium, albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 9.6 − 0.8 × (4 − 2) = 8, computed on the CPU.
    assert!(
        s.contains("\"name\":\"measured_calcium\"") && s.contains("\"value\":8"),
        "measured_calcium(9.6, 2) = 8: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "measured_calcium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// albumin — the same correction solved for the albumin: 4 − (corrected − measured) / 0.8, the third
// reading of the one correction.
// ---------------------------------------------------------------------------

#[test]
fn computes_albumin_from_corrected_and_measured_with_citation() {
    let dir = scratch("alb");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-calcium.adj\"\n\
         observe corrected_calcium(9.6)\n\
         observe measured_calcium(8)\n\
         ? albumin(corrected_calcium, measured_calcium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 − (9.6 − 8) / 0.8 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"albumin\"") && s.contains("\"value\":2"),
        "albumin(9.6, 8) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "albumin carries its cited provenance: {s}"
    );
}
