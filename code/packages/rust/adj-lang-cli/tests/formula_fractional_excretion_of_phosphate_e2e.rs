//! End-to-end tests for the `clinical/fractional-excretion-of-phosphate.adj` library — the fractional
//! excretion of phosphate (FEPO4 = (urine PO4 × serum Cr × 100) / (serum PO4 × urine Cr)) and its two
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the four laboratory values with `observe`, and the engine applies the cited formula on
//! the CPU, computing the EXACT value (over exact rationals) and rendering the citation and trust tier in
//! the `derived` section (the auditable answer). The three formulas INVERT around the worked case urine
//! PO4 = 30, serum PO4 = 4, urine Cr = 50, serum Cr = 1: (30 × 1 × 100) / (4 × 50) = 15 (FEPO4),
//! 15 × 4 × 50 / (100 × 1) = 30 (urine PO4), 30 × 1 × 100 / (15 × 50) = 4 (serum PO4). The three asserted
//! values (15, 30, 4) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-phosphate library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_fep_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-phosphate.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-phosphate.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fep_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fep_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-phosphate.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fep — the fraction: (urine PO4 × serum Cr × 100) / (serum PO4 × urine Cr).
// ---------------------------------------------------------------------------

#[test]
fn imports_fep_library_and_computes_it_with_citation() {
    let dir = scratch("fep");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-phosphate.adj\"\n\
         observe urine_phosphate(30)\n\
         observe serum_phosphate(4)\n\
         observe urine_creatinine(50)\n\
         observe serum_creatinine(1)\n\
         ? fep(urine_phosphate, serum_phosphate, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (30 × 1 × 100) / (4 × 50) =
    // 3000 / 200 = 15, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fep\"") && s.contains("\"value\":15"),
        "fep(30, 4, 50, 1) = 15: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_phosphate — the same equation solved for the urine phosphate:
// FEPO4 × serum PO4 × urine Cr / (100 × serum Cr).
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_phosphate_from_fep_with_citation() {
    let dir = scratch("upo4");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-phosphate.adj\"\n\
         observe fep(15)\n\
         observe serum_phosphate(4)\n\
         observe urine_creatinine(50)\n\
         observe serum_creatinine(1)\n\
         ? urine_phosphate(fep, serum_phosphate, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 × 4 × 50 / (100 × 1) = 3000 / 100 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_phosphate\"") && s.contains("\"value\":30"),
        "urine_phosphate(15, 4, 50, 1) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_phosphate carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_phosphate — the same equation solved for the serum phosphate:
// urine PO4 × serum Cr × 100 / (FEPO4 × urine Cr), the third reading of the one ratio.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_phosphate_from_fep_with_citation() {
    let dir = scratch("ppo4");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-phosphate.adj\"\n\
         observe fep(15)\n\
         observe urine_phosphate(30)\n\
         observe urine_creatinine(50)\n\
         observe serum_creatinine(1)\n\
         ? serum_phosphate(fep, urine_phosphate, urine_creatinine, serum_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 × 1 × 100 / (15 × 50) = 3000 / 750 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_phosphate\"") && s.contains("\"value\":4"),
        "serum_phosphate(15, 30, 50, 1) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_phosphate carries its cited provenance: {s}"
    );
}
