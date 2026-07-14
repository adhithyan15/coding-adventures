//! End-to-end test for the `chemistry/ideal-gas-law.adj` library — the ideal gas
//! law solved for pressure, P = nRT/V — driven through the built CLI binary
//! against the SHIPPED stdlib. It proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library,
//! binds the three state variables with `observe`, and the engine applies the
//! cited law on the CPU — computing the value and rendering the applied formula's
//! NIST CODATA citation + trust tier in the `derived` section (the auditable
//! answer, zero math by the model).
//!
//! Because the molar gas constant R = 8.314462618 J mol⁻¹ K⁻¹ (NIST CODATA,
//! exact) makes the result a real number, the assertion parses the emitted
//! `value` and compares it to the expected pressure within a small epsilon rather
//! than matching an exact decimal string.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped ideal-gas-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_idealgas_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/ideal-gas-law.adj")
        .canonicalize()
        .expect("shipped chemistry/ideal-gas-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_idealgas_{tag}_{}", std::process::id()));
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

/// Pull the numeric `value` for the derived `name` out of the CLI's JSON output.
/// The output shape is `…"name":"pressure","value":101324.8623…,"dim":…`, so we
/// slice between the `"value":` that follows this name and the next comma.
fn derived_value(json: &str, name: &str) -> f64 {
    let anchor = format!("\"name\":\"{name}\",\"value\":");
    let start = json
        .find(&anchor)
        .map(|i| i + anchor.len())
        .unwrap_or_else(|| panic!("derived value for {name} present: {json}"));
    let rest = &json[start..];
    let end = rest.find(',').expect("value terminated by a comma");
    rest[..end]
        .trim()
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("value for {name} parses as a real: {json}"))
}

/// Ideal gas law at STP — "one mole of an ideal gas at 273.15 K occupies the
/// molar volume 0.022414 m³" → the model binds moles, temperature, volume; the
/// engine applies P = nRT/V on the CPU with the NIST CODATA molar gas constant
/// R = 8.314462618 J mol⁻¹ K⁻¹ → ~101325 Pa (one standard atmosphere), carrying
/// the authoritative NIST citation.
#[test]
fn ideal_gas_law_binds_and_computes_pressure_at_stp_with_nist_citation() {
    let dir = scratch("stp");
    let lib = std::fs::read_to_string(shipped_idealgas_lib()).unwrap();
    std::fs::write(dir.join("ideal-gas-law.adj"), lib).unwrap();
    std::fs::write(
        dir.join("case.adj"),
        "import \"ideal-gas-law.adj\"\n\
         observe moles(1)\n\
         observe temperature(273.15)\n\
         observe volume(0.022414)\n\
         ? pressure(moles, temperature, volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"derived\":["), "derived section present: {s}");

    // The pressure is a real (R makes it non-integer): parse it and compare to
    // one standard atmosphere within a small epsilon rather than matching digits.
    let pressure = derived_value(&s, "pressure");
    let expected = 101_325.0_f64; // 1 atm — STP pressure for 1 mol in the molar volume
    assert!(
        (pressure - expected).abs() < 5.0,
        "pressure at STP is ~101325 Pa (got {pressure}): {s}"
    );

    // … AND the applied law carries its NIST CODATA citation + trust tier, so the
    // value of R (and thus the answer) is auditable end to end.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("physics.nist.gov/cgi-bin/cuu/Value?r")
            && s.contains("8.314 462 618"),
        "applied formula carries its verbatim NIST CODATA provenance: {s}"
    );
}
