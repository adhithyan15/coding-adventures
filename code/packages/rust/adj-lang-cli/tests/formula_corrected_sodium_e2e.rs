//! End-to-end tests for the `clinical/corrected-sodium.adj` library — the hyperglycemia-corrected
//! serum sodium (corrected sodium = measured sodium + 1.6 × (glucose − 100) / 100) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant
//! as every other formula library: a consumer states NO arithmetic; it imports the grounded library,
//! binds the measured sodium and glucose with `observe`, and the engine applies the cited correction
//! on the CPU, computing the EXACT value (over exact rationals — 1.6 = 8/5) and rendering the citation
//! and trust tier in the `derived` section (the auditable answer). The three formulas INVERT around
//! the worked case measured sodium = 126, glucose = 600: 126 + 1.6 × (600 − 100) / 100 = 134
//! (corrected), 134 − 1.6 × (600 − 100) / 100 = 126 (measured), 100 + (134 − 126) × 100 / 1.6 = 600
//! (glucose). The three asserted values (134, 126, 600) are distinct, none a colon-anchored prefix of
//! another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped corrected-sodium library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_cs_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/corrected-sodium.adj")
        .canonicalize()
        .expect("shipped corrected-sodium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cs_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cs_lib()).unwrap();
    std::fs::write(dir.join("corrected-sodium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// corrected_sodium — the correction: measured sodium + 1.6 × (glucose − 100) / 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_corrected_sodium_library_and_computes_it_with_citation() {
    let dir = scratch("cs");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-sodium.adj\"\n\
         observe measured_sodium(126)\n\
         observe glucose(600)\n\
         ? corrected_sodium(measured_sodium, glucose)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied correction's result: 126 + 1.6 × (600 − 100) / 100
    // = 134, computed EXACTLY over rationals (1.6 = 8/5), not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"corrected_sodium\"") && s.contains("\"value\":134"),
        "corrected_sodium(126, 600) = 134: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied correction carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// measured_sodium — the same correction solved for the measured sodium: corrected − 1.6 × (glucose − 100) / 100.
// ---------------------------------------------------------------------------

#[test]
fn computes_measured_sodium_from_corrected_with_citation() {
    let dir = scratch("meas");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-sodium.adj\"\n\
         observe corrected_sodium(134)\n\
         observe glucose(600)\n\
         ? measured_sodium(corrected_sodium, glucose)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 134 − 1.6 × (600 − 100) / 100 = 126, computed on the CPU.
    assert!(
        s.contains("\"name\":\"measured_sodium\"") && s.contains("\"value\":126"),
        "measured_sodium(134, 600) = 126: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "measured_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// glucose — the same correction solved for the glucose: 100 + (corrected − measured) × 100 / 1.6, the
// third reading of the one correction.
// ---------------------------------------------------------------------------

#[test]
fn computes_glucose_from_corrected_and_measured_with_citation() {
    let dir = scratch("gluc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-sodium.adj\"\n\
         observe corrected_sodium(134)\n\
         observe measured_sodium(126)\n\
         ? glucose(corrected_sodium, measured_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 + (134 − 126) × 100 / 1.6 = 100 + 800 / 1.6 = 100 + 500 = 600, computed on the CPU.
    assert!(
        s.contains("\"name\":\"glucose\"") && s.contains("\"value\":600"),
        "glucose(134, 126) = 600: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "glucose carries its cited provenance: {s}"
    );
}
