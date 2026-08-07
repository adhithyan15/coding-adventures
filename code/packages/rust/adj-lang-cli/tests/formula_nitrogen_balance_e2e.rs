//! End-to-end tests for the `clinical/nitrogen-balance.adj` library — the nitrogen balance
//! (balance = protein intake / 6.25 − (urine urea nitrogen + 4)) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the protein intake and
//! the urine urea nitrogen with `observe`, and the engine applies the cited formula on the CPU, computing the
//! EXACT value (over exact rationals) and rendering the citation and trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT around the worked case protein = 125, UUN = 12:
//! 125 / 6.25 − (12 + 4) = 4 (balance), (4 + 12 + 4) × 6.25 = 125 (protein), 125 / 6.25 − 4 − 4 = 12 (UUN).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 6.25 and 4 and the intermediates 20/16, so a bare
//! numeric substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped nitrogen-balance library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_nb_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/nitrogen-balance.adj")
        .canonicalize()
        .expect("shipped nitrogen-balance.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_nb_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_nb_lib()).unwrap();
    std::fs::write(dir.join("nitrogen-balance.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// nitrogen_balance — the balance: protein / 6.25 − (UUN + 4).
// ---------------------------------------------------------------------------

#[test]
fn imports_nitrogen_balance_library_and_computes_it_with_citation() {
    let dir = scratch("nb");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"nitrogen-balance.adj\"\n\
         observe protein_intake(125)\n\
         observe urine_urea_nitrogen(12)\n\
         ? nitrogen_balance(protein_intake, urine_urea_nitrogen)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 125 / 6.25 − (12 + 4) = 20 − 16 = 4,
    // computed EXACTLY over rationals. Match the adjacent name/value pair so the 6.25/4 constants and the
    // 20/16 intermediates cannot spuriously satisfy a bare "value":4.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"nitrogen_balance\",\"value\":4"),
        "nitrogen_balance(125, 12) = 4: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// protein_intake — the same equation solved for the protein intake: (balance + UUN + 4) × 6.25.
// ---------------------------------------------------------------------------

#[test]
fn computes_protein_intake_from_balance_with_citation() {
    let dir = scratch("protein");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"nitrogen-balance.adj\"\n\
         observe nitrogen_balance(4)\n\
         observe urine_urea_nitrogen(12)\n\
         ? protein_intake(nitrogen_balance, urine_urea_nitrogen)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (4 + 12 + 4) × 6.25 = 20 × 6.25 = 125, computed on the CPU.
    assert!(
        s.contains("\"name\":\"protein_intake\",\"value\":125"),
        "protein_intake(4, 12) = 125: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "protein_intake carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_urea_nitrogen — the same equation solved for the UUN: protein / 6.25 − 4 − balance, the third
// reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_urea_nitrogen_from_balance_with_citation() {
    let dir = scratch("uun");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"nitrogen-balance.adj\"\n\
         observe nitrogen_balance(4)\n\
         observe protein_intake(125)\n\
         ? urine_urea_nitrogen(nitrogen_balance, protein_intake)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 125 / 6.25 − 4 − 4 = 20 − 4 − 4 = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_urea_nitrogen\",\"value\":12"),
        "urine_urea_nitrogen(4, 125) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_urea_nitrogen carries its cited provenance: {s}"
    );
}
