//! End-to-end tests for the `metrology/temperature.adj` library — the
//! temperature-scale conversion formulas — driven through the built CLI binary
//! against the SHIPPED stdlib. Each proves the same invariant as the other
//! formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds a reading with `observe`, and the engine applies the cited
//! formula on the CPU — computing the EXACT value and rendering the applied
//! formula's NIST citation + trust tier in the `derived` section (the auditable
//! answer).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped temperature library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_temperature_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/metrology/temperature.adj")
        .canonicalize()
        .expect("shipped temperature.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_temp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_temperature_lib()).unwrap();
    std::fs::write(dir.join("temperature.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fahrenheit(celsius) — Celsius → Fahrenheit, NIST's (°C * 1.8) + 32.
// ---------------------------------------------------------------------------

#[test]
fn imports_temperature_library_and_converts_celsius_to_fahrenheit_with_citation() {
    let dir = scratch("c2f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"temperature.adj\"\n\
         observe celsius(100)\n\
         ? fahrenheit(celsius)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: the
    // boiling point of water, 100 °C = 212 °F, computed on the CPU.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fahrenheit\"") && s.contains("\"value\":212"),
        "fahrenheit(100) = 212: {s}"
    );
    // … AND the NIST citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("nist.gov/pml/owm/si-units-temperature"),
        "applied formula carries its cited NIST provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// celsius(fahrenheit) — Fahrenheit → Celsius, NIST's (°F - 32) / 1.8.
// ---------------------------------------------------------------------------

#[test]
fn converts_fahrenheit_to_celsius_with_citation() {
    let dir = scratch("f2c");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"temperature.adj\"\n\
         observe fahrenheit(32)\n\
         ? celsius(fahrenheit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The freezing point of water, 32 °F = 0 °C, computed on the CPU.
    assert!(
        s.contains("\"name\":\"celsius\"") && s.contains("\"value\":0"),
        "celsius(32) = 0: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("nist.gov/pml/owm/si-units-temperature"),
        "conversion carries its NIST citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// kelvin(celsius) — Celsius → Kelvin, NIST's °C + 273.15 (a distinct source
// cell and a non-integer result the engine renders exactly).
// ---------------------------------------------------------------------------

#[test]
fn converts_celsius_to_kelvin_with_citation() {
    let dir = scratch("c2k");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"temperature.adj\"\n\
         observe celsius(0)\n\
         ? kelvin(celsius)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The ice point of water on the Kelvin scale, 0 °C = 273.15 K.
    assert!(
        s.contains("\"name\":\"kelvin\"") && s.contains("\"value\":273.15"),
        "kelvin(0) = 273.15: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("nist.gov/pml/owm/si-units-temperature"),
        "conversion carries its NIST citation: {s}"
    );
}
