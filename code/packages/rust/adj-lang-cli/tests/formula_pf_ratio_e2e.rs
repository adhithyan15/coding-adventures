//! End-to-end tests for the `clinical/pf-ratio.adj` library — the P/F-ratio relation
//! (P/F ratio = arterial oxygen partial pressure ÷ fraction of inspired oxygen) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured oxygenation values with `observe`, and the engine applies the cited
//! definition on the CPU, computing the EXACT value and rendering the definition's citation and trust
//! tier in the `derived` section (the auditable answer). The three formulas INVERT around the worked
//! case PaO2 = 200 mmHg, FiO2 = 0.5: 200 ÷ 0.5 = 400 (P/F ratio), 400 × 0.5 = 200 (PaO2), 200 ÷ 400 =
//! 0.5 (FiO2). The asserted values (400, 200, 0.5) — none is a colon-anchored prefix of another
//! rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped P/F-ratio library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_pf_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/pf-ratio.adj")
        .canonicalize()
        .expect("shipped pf-ratio.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pf_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_pf_lib()).unwrap();
    std::fs::write(dir.join("pf-ratio.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// pf_ratio — the relation: arterial oxygen partial pressure ÷ fraction of inspired oxygen.
// ---------------------------------------------------------------------------

#[test]
fn imports_pf_ratio_library_and_computes_it_with_citation() {
    let dir = scratch("pf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pf-ratio.adj\"\n\
         observe arterial_oxygen_partial_pressure(200)\n\
         observe fraction_of_inspired_oxygen(0.5)\n\
         ? pf_ratio(arterial_oxygen_partial_pressure, fraction_of_inspired_oxygen)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 200 ÷ 0.5 = 400.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"pf_ratio\"") && s.contains("\"value\":400"),
        "pf_ratio(200, 0.5) = 400: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// arterial_oxygen_partial_pressure — the same definition solved for the PaO2: P/F × FiO2.
// ---------------------------------------------------------------------------

#[test]
fn computes_pao2_from_ratio_and_fio2_with_citation() {
    let dir = scratch("pao2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pf-ratio.adj\"\n\
         observe pf_ratio(400)\n\
         observe fraction_of_inspired_oxygen(0.5)\n\
         ? arterial_oxygen_partial_pressure(pf_ratio, fraction_of_inspired_oxygen)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 400 × 0.5 = 200, computed on the CPU.
    assert!(
        s.contains("\"name\":\"arterial_oxygen_partial_pressure\"") && s.contains("\"value\":200"),
        "arterial_oxygen_partial_pressure(400, 0.5) = 200: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "arterial_oxygen_partial_pressure carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// fraction_of_inspired_oxygen — the same definition solved for the FiO2: PaO2 ÷ P/F, the third
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_fio2_from_pao2_and_ratio_with_citation() {
    let dir = scratch("fio2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pf-ratio.adj\"\n\
         observe arterial_oxygen_partial_pressure(200)\n\
         observe pf_ratio(400)\n\
         ? fraction_of_inspired_oxygen(arterial_oxygen_partial_pressure, pf_ratio)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 200 ÷ 400 = 0.5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"fraction_of_inspired_oxygen\"") && s.contains("\"value\":0.5"),
        "fraction_of_inspired_oxygen(200, 400) = 0.5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "fraction_of_inspired_oxygen carries its cited provenance: {s}"
    );
}
