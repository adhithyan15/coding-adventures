//! End-to-end tests for the `clinical/alveolar-gas-equation.adj` library — the alveolar gas equation
//! (PAO2 = FiO2 × (Patm − PH2O) − PaCO2 / RQ) and its two exact rearrangements — driven through the built
//! CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the inspired-gas and blood-gas quantities
//! with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over
//! exact rationals) and rendering the citation and trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT around the worked case FiO2 = 1, Patm = 760, PH2O = 47, PaCO2 = 40,
//! RQ = 0.8: 1 × (760 − 47) − 40 / 0.8 = 663 (alveolar PO2), (1 × (760 − 47) − 663) × 0.8 = 40 (PaCO2),
//! (663 + 40 / 0.8) / 1 + 47 = 760 (Patm). The three asserted values (663, 40, 760) are distinct, none a
//! colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped alveolar-gas-equation library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_alveolar_gas_equation_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/alveolar-gas-equation.adj")
        .canonicalize()
        .expect("shipped alveolar-gas-equation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_alvgas_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_alveolar_gas_equation_lib()).unwrap();
    std::fs::write(dir.join("alveolar-gas-equation.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// alveolar_po2 — the alveolar oxygen: FiO2 × (Patm − PH2O) − PaCO2 / RQ.
// ---------------------------------------------------------------------------

#[test]
fn imports_alveolar_gas_equation_library_and_computes_pao2_with_citation() {
    let dir = scratch("pao2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-gas-equation.adj\"\n\
         observe fio2(1)\n\
         observe patm(760)\n\
         observe ph2o(47)\n\
         observe paco2(40)\n\
         observe rq(0.8)\n\
         ? alveolar_po2(fio2, patm, ph2o, paco2, rq)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 1 × (760 − 47) − 40 / 0.8 =
    // 713 − 50 = 663, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"alveolar_po2\"") && s.contains("\"value\":663"),
        "alveolar_po2(1, 760, 47, 40, 0.8) = 663: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// paco2 — the same equation solved for the arterial CO2: (FiO2 × (Patm − PH2O) − PAO2) × RQ.
// ---------------------------------------------------------------------------

#[test]
fn computes_paco2_from_alveolar_gas_equation_with_citation() {
    let dir = scratch("paco2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-gas-equation.adj\"\n\
         observe fio2(1)\n\
         observe patm(760)\n\
         observe ph2o(47)\n\
         observe alveolar_po2(663)\n\
         observe rq(0.8)\n\
         ? paco2(fio2, patm, ph2o, alveolar_po2, rq)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (1 × (760 − 47) − 663) × 0.8 = (713 − 663) × 0.8 = 50 × 0.8 = 40, computed on the CPU.
    assert!(
        s.contains("\"name\":\"paco2\"") && s.contains("\"value\":40"),
        "paco2(1, 760, 47, 663, 0.8) = 40: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "paco2 carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// patm — the same equation solved for the atmospheric pressure: (PAO2 + PaCO2 / RQ) / FiO2 + PH2O, the
// third reading of the one equation.
// ---------------------------------------------------------------------------

#[test]
fn computes_patm_from_alveolar_gas_equation_with_citation() {
    let dir = scratch("patm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-gas-equation.adj\"\n\
         observe fio2(1)\n\
         observe ph2o(47)\n\
         observe paco2(40)\n\
         observe alveolar_po2(663)\n\
         observe rq(0.8)\n\
         ? patm(fio2, ph2o, paco2, alveolar_po2, rq)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (663 + 40 / 0.8) / 1 + 47 = (663 + 50) + 47 = 760, computed on the CPU.
    assert!(
        s.contains("\"name\":\"patm\"") && s.contains("\"value\":760"),
        "patm(1, 47, 40, 663, 0.8) = 760: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "patm carries its cited provenance: {s}"
    );
}
