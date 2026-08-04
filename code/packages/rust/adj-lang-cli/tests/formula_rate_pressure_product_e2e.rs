//! End-to-end tests for the `clinical/rate-pressure-product.adj` library — the definition of the
//! rate pressure product (RPP = heart rate × systolic blood pressure) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the
//! grounded library, binds the measured vitals with `observe`, and the engine applies the cited
//! relation on the CPU, computing the EXACT value and rendering the relation's citation and trust
//! tier in the `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case HR = 80 /min, SBP = 120 mmHg: 80 × 120 = 9600, 9600 ÷ 120 = 80, and 9600 ÷ 80 =
//! 120. The three asserted values (9600, 80, 120) are chosen so none is a substring of another
//! rendered value. This is the cardiology product-cousin of the shipped shock-index.adj (SI = HR
//! ÷ SBP), pairing the same two vitals the other way.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped rate-pressure-product library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_rpp_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/rate-pressure-product.adj")
        .canonicalize()
        .expect("shipped rate-pressure-product.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rpp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_rpp_lib()).unwrap();
    std::fs::write(dir.join("rate-pressure-product.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// rate_pressure_product — the definition: heart rate times systolic blood pressure.
// ---------------------------------------------------------------------------

#[test]
fn imports_rate_pressure_product_library_and_computes_it_with_citation() {
    let dir = scratch("rpp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rate-pressure-product.adj\"\n\
         observe heart_rate(80)\n\
         observe systolic_blood_pressure(120)\n\
         ? rate_pressure_product(heart_rate, systolic_blood_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 80 × 120 = 9600.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"rate_pressure_product\"") && s.contains("\"value\":9600"),
        "rate_pressure_product(80, 120) = 9600: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// heart_rate — the same relation solved for HR: RPP ÷ SBP.
// ---------------------------------------------------------------------------

#[test]
fn computes_heart_rate_from_rpp_and_sbp_with_citation() {
    let dir = scratch("hr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rate-pressure-product.adj\"\n\
         observe rate_pressure_product(9600)\n\
         observe systolic_blood_pressure(120)\n\
         ? heart_rate(rate_pressure_product, systolic_blood_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 9600 ÷ 120 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"heart_rate\"") && s.contains("\"value\":80"),
        "heart_rate(9600, 120) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "heart_rate carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// systolic_blood_pressure — the same relation solved for SBP: RPP ÷ HR, the third reading of the
// one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_sbp_from_rpp_and_heart_rate_with_citation() {
    let dir = scratch("sbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rate-pressure-product.adj\"\n\
         observe rate_pressure_product(9600)\n\
         observe heart_rate(80)\n\
         ? systolic_blood_pressure(rate_pressure_product, heart_rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 9600 ÷ 80 = 120, computed on the CPU.
    assert!(
        s.contains("\"name\":\"systolic_blood_pressure\"") && s.contains("\"value\":120"),
        "systolic_blood_pressure(9600, 80) = 120: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "systolic_blood_pressure carries its cited provenance: {s}"
    );
}
