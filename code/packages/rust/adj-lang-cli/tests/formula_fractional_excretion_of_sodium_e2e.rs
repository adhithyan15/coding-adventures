//! End-to-end tests for the `clinical/fractional-excretion-of-sodium.adj` library — the fractional
//! excretion of sodium (FeNa = 100 × urinary sodium × serum creatinine / (serum sodium × urinary
//! creatinine)) and its four exact rearrangements — driven through the built CLI binary against the
//! SHIPPED stdlib. The same invariant as every other formula library: a consumer states NO arithmetic;
//! it imports the grounded library, binds the four laboratory values with `observe`, and the engine
//! applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and rendering
//! the citation and trust tier in the `derived` section (the auditable answer). The five formulas
//! INVERT around the worked case urinary sodium = 20, serum creatinine = 4, serum sodium = 125,
//! urinary creatinine = 80: 100 × 20 × 4 / (125 × 80) = 0.8 (FeNa), and each inverse back-solves its
//! own input — 0.8 × 125 × 80 / (100 × 4) = 20 (urine Na), 0.8 × 125 × 80 / (100 × 20) = 4 (serum Cr),
//! 100 × 20 × 4 / (0.8 × 80) = 125 (serum Na), 100 × 20 × 4 / (0.8 × 125) = 80 (urine Cr). The five
//! asserted values (0.8, 20, 4, 125, 80) are distinct, none a colon-anchored prefix of another.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-sodium library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_fena_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-sodium.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-sodium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fena_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fena_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-sodium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fractional_excretion_sodium — the index: 100 × uNa × sCr / (sNa × uCr).
// ---------------------------------------------------------------------------

#[test]
fn imports_fena_library_and_computes_it_with_citation() {
    let dir = scratch("fena");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-sodium.adj\"\n\
         observe urinary_sodium(20)\n\
         observe serum_creatinine(4)\n\
         observe serum_sodium(125)\n\
         observe urinary_creatinine(80)\n\
         ? fractional_excretion_sodium(urinary_sodium, serum_creatinine, serum_sodium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 100 × 20 × 4 / (125 × 80) = 0.8,
    // computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fractional_excretion_sodium\"") && s.contains("\"value\":0.8"),
        "fractional_excretion_sodium(20, 4, 125, 80) = 0.8: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urinary_sodium — the same equation solved for the urinary sodium: FeNa × sNa × uCr / (100 × sCr).
// ---------------------------------------------------------------------------

#[test]
fn computes_urinary_sodium_from_fena_with_citation() {
    let dir = scratch("una");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-sodium.adj\"\n\
         observe fractional_excretion_sodium(0.8)\n\
         observe serum_creatinine(4)\n\
         observe serum_sodium(125)\n\
         observe urinary_creatinine(80)\n\
         ? urinary_sodium(fractional_excretion_sodium, serum_creatinine, serum_sodium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.8 × 125 × 80 / (100 × 4) = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urinary_sodium\"") && s.contains("\"value\":20"),
        "urinary_sodium(0.8, 4, 125, 80) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urinary_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_creatinine — the same equation solved for the serum creatinine: FeNa × sNa × uCr / (100 × uNa).
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_creatinine_from_fena_with_citation() {
    let dir = scratch("scr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-sodium.adj\"\n\
         observe fractional_excretion_sodium(0.8)\n\
         observe urinary_sodium(20)\n\
         observe serum_sodium(125)\n\
         observe urinary_creatinine(80)\n\
         ? serum_creatinine(fractional_excretion_sodium, urinary_sodium, serum_sodium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.8 × 125 × 80 / (100 × 20) = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_creatinine\"") && s.contains("\"value\":4"),
        "serum_creatinine(0.8, 20, 125, 80) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_creatinine carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_sodium — the same equation solved for the serum sodium: 100 × uNa × sCr / (FeNa × uCr).
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_sodium_from_fena_with_citation() {
    let dir = scratch("sna");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-sodium.adj\"\n\
         observe fractional_excretion_sodium(0.8)\n\
         observe urinary_sodium(20)\n\
         observe serum_creatinine(4)\n\
         observe urinary_creatinine(80)\n\
         ? serum_sodium(fractional_excretion_sodium, urinary_sodium, serum_creatinine, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 × 20 × 4 / (0.8 × 80) = 125, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_sodium\"") && s.contains("\"value\":125"),
        "serum_sodium(0.8, 20, 4, 80) = 125: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urinary_creatinine — the same equation solved for the urinary creatinine: 100 × uNa × sCr / (FeNa ×
// sNa), the fifth reading of the one index.
// ---------------------------------------------------------------------------

#[test]
fn computes_urinary_creatinine_from_fena_with_citation() {
    let dir = scratch("ucr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-sodium.adj\"\n\
         observe fractional_excretion_sodium(0.8)\n\
         observe urinary_sodium(20)\n\
         observe serum_creatinine(4)\n\
         observe serum_sodium(125)\n\
         ? urinary_creatinine(fractional_excretion_sodium, urinary_sodium, serum_creatinine, serum_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 × 20 × 4 / (0.8 × 125) = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urinary_creatinine\"") && s.contains("\"value\":80"),
        "urinary_creatinine(0.8, 20, 4, 125) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urinary_creatinine carries its cited provenance: {s}"
    );
}
