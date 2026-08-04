//! End-to-end tests for the `clinical/fractional-excretion-of-urea.adj` library — the fractional excretion
//! of urea (FEUrea = [urine urea / serum urea] / [urine creatinine / serum creatinine] × 100) and its two
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant
//! as every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds
//! the four measured concentrations with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case Uurea = 300,
//! Surea = 30, Ucr = 100, Scr = 2: 300 × 2 × 100 / (30 × 100) = 20 (FEUrea),
//! 20 × 30 × 100 / (2 × 100) = 300 (Uurea), 300 × 2 × 100 / (20 × 100) = 30 (Surea).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree carries the constant 100 and intermediates (600, 3000, 60000), and 30
//! is a leading-digit prefix of 300, so a bare numeric substring could spuriously match. The name-anchored
//! adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-urea library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_feurea_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-urea.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-urea.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_feurea_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_feurea_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-urea.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fractional_excretion_of_urea — the percentage: Uurea × Scr × 100 / (Surea × Ucr).
// ---------------------------------------------------------------------------

#[test]
fn imports_feurea_library_and_computes_it_with_citation() {
    let dir = scratch("feurea");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-urea.adj\"\n\
         observe urine_urea(300)\n\
         observe serum_urea(30)\n\
         observe urine_creatinine(100)\n\
         observe serum_creatinine(2)\n\
         ? fractional_excretion_of_urea(urine_urea, serum_creatinine, serum_urea, urine_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 300 × 2 × 100 / (30 × 100) = 20,
    // computed EXACTLY over rationals. Match the adjacent name/value pair so the 100 constant and the
    // 600/3000/60000 intermediates cannot spuriously satisfy a bare "value":20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fractional_excretion_of_urea\",\"value\":20"),
        "fractional_excretion_of_urea(300, 2, 30, 100) = 20: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_urea — the same equation solved for the urine urea: FEUrea × Surea × Ucr / (Scr × 100).
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_urea_from_feurea_with_citation() {
    let dir = scratch("uu");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-urea.adj\"\n\
         observe fractional_excretion_of_urea(20)\n\
         observe serum_urea(30)\n\
         observe urine_creatinine(100)\n\
         observe serum_creatinine(2)\n\
         ? urine_urea(fractional_excretion_of_urea, serum_urea, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 × 30 × 100 / (2 × 100) = 60000 / 200 = 300, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_urea\",\"value\":300"),
        "urine_urea(20, 30, 100, 2) = 300: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_urea carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_urea — the same equation solved for the serum urea: Uurea × Scr × 100 / (FEUrea × Ucr), the third
// reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_urea_from_feurea_with_citation() {
    let dir = scratch("su");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-urea.adj\"\n\
         observe fractional_excretion_of_urea(20)\n\
         observe urine_urea(300)\n\
         observe serum_creatinine(2)\n\
         observe urine_creatinine(100)\n\
         ? serum_urea(fractional_excretion_of_urea, urine_urea, serum_creatinine, urine_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 300 × 2 × 100 / (20 × 100) = 60000 / 2000 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_urea\",\"value\":30"),
        "serum_urea(20, 300, 2, 100) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_urea carries its cited provenance: {s}"
    );
}
