//! End-to-end tests for the `physics/wave-speed.adj` library — the wave relation
//! (v = f·λ) and its two exact rearrangements (f = v/λ, λ = v/f) — driven through
//! the built CLI binary against the SHIPPED stdlib. Each proves the same invariant
//! as the other formula libraries: a consumer states NO arithmetic; it imports the
//! grounded library, binds the measured quantities with `observe`, and the engine
//! applies the cited relation on the CPU — computing the EXACT value and rendering
//! the relation's citation + trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT: 2 * 3 = 6, 6 / 3 = 2, 6 / 2 = 3.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped wave-speed library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_wave_speed_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/wave-speed.adj")
        .canonicalize()
        .expect("shipped wave-speed.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_wave_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_wave_speed_lib()).unwrap();
    std::fs::write(dir.join("wave-speed.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// wave speed — the relation: the frequency times the wavelength (v = f·λ).
// ---------------------------------------------------------------------------

#[test]
fn imports_wave_speed_library_and_computes_relation_with_citation() {
    let dir = scratch("v");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-speed.adj\"\n\
         observe frequency(2)\n\
         observe wavelength(3)\n\
         ? wave_speed(frequency, wavelength)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2 * 3 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"wave_speed\"") && s.contains("\"value\":6"),
        "wave_speed(2, 3) = 6: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// frequency — the same relation solved for f: the speed over the wavelength, which
// INVERTS the wave speed just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_frequency_as_speed_over_wavelength_with_citation() {
    let dir = scratch("f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-speed.adj\"\n\
         observe wave_speed(6)\n\
         observe wavelength(3)\n\
         ? frequency(wave_speed, wavelength)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"frequency\"") && s.contains("\"value\":2"),
        "frequency(6, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "frequency carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// wavelength — the same relation solved for λ: the speed over the frequency, the
// third exact reading of the one relation.
// ---------------------------------------------------------------------------

#[test]
fn computes_wavelength_as_speed_over_frequency_with_citation() {
    let dir = scratch("lambda");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-speed.adj\"\n\
         observe wave_speed(6)\n\
         observe frequency(2)\n\
         ? wavelength(wave_speed, frequency)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 2 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"wavelength\"") && s.contains("\"value\":3"),
        "wavelength(6, 2) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "wavelength carries its LibreTexts citation: {s}"
    );
}
