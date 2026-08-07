//! End-to-end tests for the `clinical/minute-ventilation.adj` library — the definition
//! of minute ventilation (minute volume MV = tidal volume TV × respiratory frequency
//! Rf) and its two exact rearrangements (TV = MV/Rf, Rf = MV/TV) — driven through the
//! built CLI binary against the SHIPPED stdlib.
//! Each proves the same invariant as the other formula libraries: a consumer states
//! NO arithmetic; it imports the grounded library, binds the respiratory quantities
//! with `observe`, and the engine applies the cited definition on the CPU, computing
//! the EXACT value and rendering the definition's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case TV = 500 mL, Rf = 12 breaths/min: 500 * 12 = 6000, and both
//! 6000 / 12 = 500 and 6000 / 500 = 12 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped minute-ventilation library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_minute_ventilation_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/minute-ventilation.adj")
        .canonicalize()
        .expect("shipped minute-ventilation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mv_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_minute_ventilation_lib()).unwrap();
    std::fs::write(dir.join("minute-ventilation.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// minute_volume — the definition: the tidal volume times the respiratory frequency.
// ---------------------------------------------------------------------------

#[test]
fn imports_minute_ventilation_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"minute-ventilation.adj\"\n\
         observe tidal_volume(500)\n\
         observe respiratory_frequency(12)\n\
         ? minute_volume(tidal_volume, respiratory_frequency)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 500 * 12 = 6000.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"minute_volume\"") && s.contains("\"value\":6000"),
        "minute_volume(500, 12) = 6000: {s}"
    );
    // … AND the LibreTexts citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — the same definition solved for TV: the minute volume divided by the
// respiratory frequency, which INVERTS the minute volume just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_as_minute_volume_over_frequency_with_citation() {
    let dir = scratch("tv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"minute-ventilation.adj\"\n\
         observe minute_volume(6000)\n\
         observe respiratory_frequency(12)\n\
         ? tidal_volume(minute_volume, respiratory_frequency)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6000 / 12 = 500, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\"") && s.contains("\"value\":500"),
        "tidal_volume(6000, 12) = 500: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "tidal_volume carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// respiratory_frequency — the same definition solved for Rf: the minute volume divided
// by the tidal volume, the third exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_frequency_as_minute_volume_over_tidal_volume_with_citation() {
    let dir = scratch("rf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"minute-ventilation.adj\"\n\
         observe minute_volume(6000)\n\
         observe tidal_volume(500)\n\
         ? respiratory_frequency(minute_volume, tidal_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6000 / 500 = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"respiratory_frequency\"") && s.contains("\"value\":12"),
        "respiratory_frequency(6000, 500) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("med.libretexts.org"),
        "respiratory_frequency carries its LibreTexts citation: {s}"
    );
}
