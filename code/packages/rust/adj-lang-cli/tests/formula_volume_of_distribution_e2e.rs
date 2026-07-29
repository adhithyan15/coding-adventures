//! End-to-end tests for the `clinical/volume-of-distribution.adj` library — the definition of the
//! (apparent) volume of distribution (Vd = amount of drug in the body ÷ plasma concentration) and
//! its two exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib.
//! The same invariant as every other formula library: a consumer states NO arithmetic; it imports
//! the grounded library, binds the measured quantities with `observe`, and the engine applies the
//! cited relation on the CPU, computing the EXACT value and rendering the relation's citation and
//! trust tier in the `derived` section (the auditable answer). The three formulas INVERT around
//! the worked case amount = 200 mg, Cp = 5 mg/L: 200 ÷ 5 = 40, 40 × 5 = 200, and 200 ÷ 40 = 5. The
//! three asserted values (40, 200, 5) are chosen so none is a colon-anchored prefix of another
//! rendered value. This is the first pharmacokinetics library, a dosing cousin of the shipped
//! cockcroft_gault.adj — a ratio-DEFINED quantity (unlike the product-defined rate-pressure-product).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped volume-of-distribution library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_vd_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/volume-of-distribution.adj")
        .canonicalize()
        .expect("shipped volume-of-distribution.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vd_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_vd_lib()).unwrap();
    std::fs::write(dir.join("volume-of-distribution.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// volume_of_distribution — the definition: amount of drug divided by plasma concentration.
// ---------------------------------------------------------------------------

#[test]
fn imports_volume_of_distribution_library_and_computes_it_with_citation() {
    let dir = scratch("vd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volume-of-distribution.adj\"\n\
         observe amount_of_drug_in_body(200)\n\
         observe plasma_concentration(5)\n\
         ? volume_of_distribution(amount_of_drug_in_body, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 200 ÷ 5 = 40.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"volume_of_distribution\"") && s.contains("\"value\":40"),
        "volume_of_distribution(200, 5) = 40: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// amount_of_drug_in_body — the same relation solved for the amount: Vd × Cp.
// ---------------------------------------------------------------------------

#[test]
fn computes_amount_from_vd_and_concentration_with_citation() {
    let dir = scratch("amount");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volume-of-distribution.adj\"\n\
         observe volume_of_distribution(40)\n\
         observe plasma_concentration(5)\n\
         ? amount_of_drug_in_body(volume_of_distribution, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 40 × 5 = 200, computed on the CPU.
    assert!(
        s.contains("\"name\":\"amount_of_drug_in_body\"") && s.contains("\"value\":200"),
        "amount_of_drug_in_body(40, 5) = 200: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "amount_of_drug_in_body carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plasma_concentration — the same relation solved for the concentration: amount ÷ Vd, the third
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_concentration_from_amount_and_vd_with_citation() {
    let dir = scratch("cp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volume-of-distribution.adj\"\n\
         observe amount_of_drug_in_body(200)\n\
         observe volume_of_distribution(40)\n\
         ? plasma_concentration(amount_of_drug_in_body, volume_of_distribution)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 200 ÷ 40 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plasma_concentration\"") && s.contains("\"value\":5"),
        "plasma_concentration(200, 40) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plasma_concentration carries its cited provenance: {s}"
    );
}
