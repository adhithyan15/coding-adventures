//! End-to-end tests for the `clinical/fractional-excretion-of-magnesium.adj` library — the fractional
//! excretion of magnesium (FEMg = [(UMg × PCr) / (PMg × UCr × 0.7)] × 100) and its four exact rearrangements —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the four measured values
//! with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over exact
//! rationals) and rendering the citation and trust tier in the `derived` section (the auditable answer). The
//! five formulas INVERT around the worked case UMg = 7, PCr = 1, PMg = 1, UCr = 100 (FEMg = 10).
//!
//! The cited "× 100 ÷ 0.7" is written in the library as "× 1000 ÷ 7" (its exact integer form, since
//! 0.7 = 7/10) so the whole computation stays in exact integer arithmetic and every rendered value is an exact
//! integer.
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 1000 and 7, so a bare numeric substring could spuriously
//! match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-magnesium library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_femg_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-magnesium.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-magnesium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_femg_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_femg_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-magnesium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fractional_excretion_magnesium — the FEMg: [(UMg × PCr) / (PMg × UCr × 0.7)] × 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_femg_library_and_computes_it_with_citation() {
    let dir = scratch("femg");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-magnesium.adj\"\n\
         observe urinary_magnesium(7)\n\
         observe serum_creatinine(1)\n\
         observe serum_magnesium(1)\n\
         observe urinary_creatinine(100)\n\
         ? fractional_excretion_magnesium(urinary_magnesium, serum_creatinine, serum_magnesium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1000 × 7 × 1 / (1 × 100 × 7) = 7000 / 700 = 10, computed EXACTLY over rationals.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fractional_excretion_magnesium\",\"value\":10"),
        "FEMg(7, 1, 1, 100) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urinary_magnesium — solved for UMg: FEMg × PMg × UCr × 7 / (1000 × PCr).
// ---------------------------------------------------------------------------

#[test]
fn computes_urinary_magnesium_from_femg_with_citation() {
    let dir = scratch("umg");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-magnesium.adj\"\n\
         observe fractional_excretion_magnesium(10)\n\
         observe serum_creatinine(1)\n\
         observe serum_magnesium(1)\n\
         observe urinary_creatinine(100)\n\
         ? urinary_magnesium(fractional_excretion_magnesium, serum_creatinine, serum_magnesium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 × 1 × 100 × 7 / (1000 × 1) = 7000 / 1000 = 7.
    assert!(
        s.contains("\"name\":\"urinary_magnesium\",\"value\":7"),
        "urinary_magnesium(10, 1, 1, 100) = 7: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urinary_magnesium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_magnesium — solved for PMg: 1000 × UMg × PCr / (FEMg × UCr × 7).
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_magnesium_from_femg_with_citation() {
    let dir = scratch("pmg");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-magnesium.adj\"\n\
         observe fractional_excretion_magnesium(10)\n\
         observe urinary_magnesium(7)\n\
         observe serum_creatinine(1)\n\
         observe urinary_creatinine(100)\n\
         ? serum_magnesium(fractional_excretion_magnesium, urinary_magnesium, serum_creatinine, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1000 × 7 × 1 / (10 × 100 × 7) = 7000 / 7000 = 1.
    assert!(
        s.contains("\"name\":\"serum_magnesium\",\"value\":1"),
        "serum_magnesium(10, 7, 1, 100) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_magnesium carries its cited provenance: {s}"
    );
}
