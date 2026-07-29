//! End-to-end tests for the `clinical/anion-gap.adj` library — the definition of the
//! serum anion gap (AG = (Na + K) - (Cl + HCO3)) and its two exact rearrangements
//! (Na = AG + Cl + HCO3 - K, HCO3 = Na + K - Cl - AG) — driven through the built CLI
//! binary against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library, binds the
//! electrolytes with `observe`, and the engine applies the cited definition on the CPU,
//! computing the EXACT value and rendering the definition's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked
//! case Na = 140, K = 5, Cl = 100, HCO3 = 25 mEq/L: (140 + 5) - (100 + 25) = 20, and both
//! 20 + 100 + 25 - 5 = 140 and 140 + 5 - 100 - 20 = 25 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped anion-gap library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_anion_gap_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/anion-gap.adj")
        .canonicalize()
        .expect("shipped anion-gap.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ag_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_anion_gap_lib()).unwrap();
    std::fs::write(dir.join("anion-gap.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// anion_gap — the definition: the measured cations (Na + K) minus the measured anions
// (Cl + HCO3).
// ---------------------------------------------------------------------------

#[test]
fn imports_anion_gap_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"anion-gap.adj\"\n\
         observe sodium(140)\n\
         observe potassium(5)\n\
         observe chloride(100)\n\
         observe bicarbonate(25)\n\
         ? anion_gap(sodium, potassium, chloride, bicarbonate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result:
    // (140 + 5) - (100 + 25) = 20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"anion_gap\"") && s.contains("\"value\":20"),
        "anion_gap(140, 5, 100, 25) = 20: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is
    // auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// sodium — the same definition solved for Na: AG + Cl + HCO3 - K, which INVERTS the anion
// gap just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_sodium_from_anion_gap_and_the_other_electrolytes_with_citation() {
    let dir = scratch("na");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"anion-gap.adj\"\n\
         observe anion_gap(20)\n\
         observe potassium(5)\n\
         observe chloride(100)\n\
         observe bicarbonate(25)\n\
         ? sodium(anion_gap, potassium, chloride, bicarbonate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 + 100 + 25 - 5 = 140, computed on the CPU.
    assert!(
        s.contains("\"name\":\"sodium\"") && s.contains("\"value\":140"),
        "sodium(20, 5, 100, 25) = 140: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "sodium carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// bicarbonate — the same definition solved for HCO3: Na + K - Cl - AG, the third exact
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_bicarbonate_from_anion_gap_and_the_other_electrolytes_with_citation() {
    let dir = scratch("hco3");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"anion-gap.adj\"\n\
         observe sodium(140)\n\
         observe potassium(5)\n\
         observe chloride(100)\n\
         observe anion_gap(20)\n\
         ? bicarbonate(sodium, potassium, chloride, anion_gap)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 140 + 5 - 100 - 20 = 25, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bicarbonate\"") && s.contains("\"value\":25"),
        "bicarbonate(140, 5, 100, 20) = 25: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bicarbonate carries its StatPearls citation: {s}"
    );
}
