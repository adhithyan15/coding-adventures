//! End-to-end tests for the `clinical/fractional-excretion-of-potassium.adj` library — the fractional
//! excretion of potassium (FEK = [(urine K / serum K) / (urine Cr / serum Cr)] × 100) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as
//! every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds
//! the four laboratory values with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case urine K = 60,
//! serum K = 5, urine Cr = 120, serum Cr = 1: (60 / 5) / (120 / 1) × 100 = 10 (FEK),
//! 10 × 5 × (120 / 1) / 100 = 60 (urine K), 60 / (120 / 1) / 10 × 100 = 5 (serum K). The three asserted
//! values (10, 60, 5) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-potassium library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_fek_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-potassium.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-potassium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fek_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fek_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-potassium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fek — the fraction: [(urine K / serum K) / (urine Cr / serum Cr)] × 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_fek_library_and_computes_it_with_citation() {
    let dir = scratch("fek");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-potassium.adj\"\n\
         observe urine_k(60)\n\
         observe serum_k(5)\n\
         observe urine_creatinine(120)\n\
         observe serum_creatinine(1)\n\
         ? fek(urine_k, serum_k, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (60 / 5) / (120 / 1) × 100 =
    // 12 / 120 × 100 = 10, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fek\"") && s.contains("\"value\":10"),
        "fek(60, 5, 120, 1) = 10: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_k — the same equation solved for the urine potassium: FEK × serum K × (urine Cr / serum Cr) / 100.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_k_from_fek_with_citation() {
    let dir = scratch("uk");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-potassium.adj\"\n\
         observe fek(10)\n\
         observe serum_k(5)\n\
         observe urine_creatinine(120)\n\
         observe serum_creatinine(1)\n\
         ? urine_k(fek, serum_k, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 × 5 × (120 / 1) / 100 = 10 × 5 × 120 / 100 = 6000 / 100 = 60, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_k\"") && s.contains("\"value\":60"),
        "urine_k(10, 5, 120, 1) = 60: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_k carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_k — the same equation solved for the serum potassium: urine K / (urine Cr / serum Cr) / FEK × 100,
// the third reading of the one ratio.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_k_from_fek_with_citation() {
    let dir = scratch("sk");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-potassium.adj\"\n\
         observe fek(10)\n\
         observe urine_k(60)\n\
         observe urine_creatinine(120)\n\
         observe serum_creatinine(1)\n\
         ? serum_k(fek, urine_k, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 60 / (120 / 1) / 10 × 100 = 60 / 120 / 10 × 100 = 0.5 / 10 × 100 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_k\"") && s.contains("\"value\":5"),
        "serum_k(10, 60, 120, 1) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_k carries its cited provenance: {s}"
    );
}
