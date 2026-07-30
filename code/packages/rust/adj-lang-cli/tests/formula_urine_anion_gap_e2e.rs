//! End-to-end tests for the `clinical/urine-anion-gap.adj` library — the urine anion gap
//! (urine Na + urine K − urine Cl) and its three exact rearrangements — driven through the built CLI
//! binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the three urine electrolytes with
//! `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over exact
//! rationals) and rendering the citation and trust tier in the `derived` section (the auditable answer).
//! The four formulas INVERT around the worked case urine Na = 40, urine K = 30, urine Cl = 60:
//! 40 + 30 − 60 = 10 (gap), 10 + 60 − 30 = 40 (Na), 10 + 60 − 40 = 30 (K), 40 + 30 − 10 = 60 (Cl). The
//! four asserted values (10, 40, 30, 60) are distinct, none a colon-anchored prefix of another rendered
//! value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped urine-anion-gap library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_urine_anion_gap_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/urine-anion-gap.adj")
        .canonicalize()
        .expect("shipped urine-anion-gap.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_uag_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_urine_anion_gap_lib()).unwrap();
    std::fs::write(dir.join("urine-anion-gap.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// urine_anion_gap — the gap: urine Na + urine K − urine Cl.
// ---------------------------------------------------------------------------

#[test]
fn imports_urine_anion_gap_library_and_computes_it_with_citation() {
    let dir = scratch("gap");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"urine-anion-gap.adj\"\n\
         observe urine_sodium(40)\n\
         observe urine_potassium(30)\n\
         observe urine_chloride(60)\n\
         ? urine_anion_gap(urine_sodium, urine_potassium, urine_chloride)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 40 + 30 − 60 = 10, computed EXACTLY
    // over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"urine_anion_gap\"") && s.contains("\"value\":10"),
        "urine_anion_gap(40, 30, 60) = 10: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_sodium — the same equation solved for the urine sodium: gap + urine Cl − urine K.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_sodium_from_gap_with_citation() {
    let dir = scratch("na");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"urine-anion-gap.adj\"\n\
         observe urine_anion_gap(10)\n\
         observe urine_potassium(30)\n\
         observe urine_chloride(60)\n\
         ? urine_sodium(urine_anion_gap, urine_potassium, urine_chloride)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 + 60 − 30 = 40, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_sodium\"") && s.contains("\"value\":40"),
        "urine_sodium(10, 30, 60) = 40: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_potassium — the same equation solved for the urine potassium: gap + urine Cl − urine Na.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_potassium_from_gap_with_citation() {
    let dir = scratch("k");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"urine-anion-gap.adj\"\n\
         observe urine_anion_gap(10)\n\
         observe urine_sodium(40)\n\
         observe urine_chloride(60)\n\
         ? urine_potassium(urine_anion_gap, urine_sodium, urine_chloride)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 + 60 − 40 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_potassium\"") && s.contains("\"value\":30"),
        "urine_potassium(10, 40, 60) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_potassium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_chloride — the same equation solved for the urine chloride: urine Na + urine K − gap, the fourth
// reading of the one gap.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_chloride_from_gap_with_citation() {
    let dir = scratch("cl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"urine-anion-gap.adj\"\n\
         observe urine_sodium(40)\n\
         observe urine_potassium(30)\n\
         observe urine_anion_gap(10)\n\
         ? urine_chloride(urine_sodium, urine_potassium, urine_anion_gap)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 40 + 30 − 10 = 60, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_chloride\"") && s.contains("\"value\":60"),
        "urine_chloride(40, 30, 10) = 60: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_chloride carries its cited provenance: {s}"
    );
}
