//! End-to-end tests for the `clinical/oxygenation-index.adj` library — the oxygenation index
//! (OI = mean airway pressure × FiO2 × 100 / PaO2) and its two exact rearrangements — driven through the
//! built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the mean airway pressure, the FiO2
//! (as a fraction), and the PaO2 with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case MAP = 10,
//! FiO2 = 0.5, PaO2 = 50: 10 × 0.5 × 100 / 50 = 10 (OI), 10 × 0.5 × 100 / 10 = 50 (PaO2),
//! 10 × 50 / (0.5 × 100) = 10 (MAP).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree carries the constant 100 and the intermediate 500 (= 10 × 0.5 × 100),
//! so bare substrings like `"value":10` (a prefix of `100`) or `"value":50` (a prefix of `500`) could
//! spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped oxygenation-index library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_oi_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/oxygenation-index.adj")
        .canonicalize()
        .expect("shipped oxygenation-index.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oi_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_oi_lib()).unwrap();
    std::fs::write(dir.join("oxygenation-index.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// oxygen_index — the index: MAP × FiO2 × 100 / PaO2.
// ---------------------------------------------------------------------------

#[test]
fn imports_oxygenation_index_library_and_computes_it_with_citation() {
    let dir = scratch("oi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygenation-index.adj\"\n\
         observe mean_airway_pressure(10)\n\
         observe fio2(0.5)\n\
         observe pao2(50)\n\
         ? oxygen_index(mean_airway_pressure, fio2, pao2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 10 × 0.5 × 100 / 50 = 10, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 100 constant and the 500 intermediate
    // in the derivation cannot spuriously satisfy a bare "value":10.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"oxygen_index\",\"value\":10"),
        "oxygen_index(10, 0.5, 50) = 10: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// pao2 — the same equation solved for the arterial oxygen tension: MAP × FiO2 × 100 / OI.
// ---------------------------------------------------------------------------

#[test]
fn computes_pao2_from_oxygenation_index_with_citation() {
    let dir = scratch("pao2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygenation-index.adj\"\n\
         observe oxygen_index(10)\n\
         observe mean_airway_pressure(10)\n\
         observe fio2(0.5)\n\
         ? pao2(oxygen_index, mean_airway_pressure, fio2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 × 0.5 × 100 / 10 = 500 / 10 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"pao2\",\"value\":50"),
        "pao2(10, 10, 0.5) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "pao2 carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mean_airway_pressure — the same equation solved for the support pressure: OI × PaO2 / (FiO2 × 100), the
// third reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_mean_airway_pressure_from_oxygenation_index_with_citation() {
    let dir = scratch("map");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygenation-index.adj\"\n\
         observe oxygen_index(10)\n\
         observe pao2(50)\n\
         observe fio2(0.5)\n\
         ? mean_airway_pressure(oxygen_index, pao2, fio2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 × 50 / (0.5 × 100) = 500 / 50 = 10, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mean_airway_pressure\",\"value\":10"),
        "mean_airway_pressure(10, 50, 0.5) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "mean_airway_pressure carries its cited provenance: {s}"
    );
}
