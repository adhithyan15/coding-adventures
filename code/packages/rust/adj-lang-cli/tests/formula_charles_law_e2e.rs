//! End-to-end tests for the `chemistry/charles-law.adj` library — Charles's law
//! (V₁/T₁ = V₂/T₂, volume directly proportional to absolute temperature at constant
//! pressure) in all FOUR exact readings — driven through the built CLI binary against
//! the SHIPPED stdlib. Each proves the same invariant as the other formula libraries: a
//! consumer states NO arithmetic; it imports the grounded library, binds the measured
//! quantities with `observe`, and the engine applies the cited relation on the CPU —
//! computing the EXACT value and rendering the relation's citation + trust tier in the
//! `derived` section (the auditable answer). The four readings all turn on the same
//! conserved cross-product V₁T₂ = V₂T₁ = 12, around V₁=2, T₁=4, V₂=3, T₂=6.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped Charles's-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_charles_law_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/charles-law.adj")
        .canonicalize()
        .expect("shipped charles-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_charles_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_charles_law_lib()).unwrap();
    std::fs::write(dir.join("charles-law.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// final_volume — V₂ = V₁T₂ / T₁: a fixed amount of gas reaches this volume at a new
// absolute temperature. 2*6 / 4 = 3.
// ---------------------------------------------------------------------------

#[test]
fn imports_charles_law_library_and_computes_final_volume_with_citation() {
    let dir = scratch("v2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"charles-law.adj\"\n\
         observe initial_volume(2)\n\
         observe initial_temperature(4)\n\
         observe final_temperature(6)\n\
         ? final_volume(initial_volume, initial_temperature, final_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2*6 / 4 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"final_volume\"") && s.contains("\"value\":3"),
        "final_volume(2, 4, 6) = 3: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// final_temperature — T₂ = V₂T₁ / V₁: the absolute temperature after expanding to a new
// volume. 3*4 / 2 = 6.
// ---------------------------------------------------------------------------

#[test]
fn computes_final_temperature_with_citation() {
    let dir = scratch("t2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"charles-law.adj\"\n\
         observe final_volume(3)\n\
         observe initial_volume(2)\n\
         observe initial_temperature(4)\n\
         ? final_temperature(final_volume, initial_volume, initial_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3*4 / 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"final_temperature\"") && s.contains("\"value\":6"),
        "final_temperature(3, 2, 4) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "final_temperature carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_volume — V₁ = V₂T₁ / T₂: recover the starting volume. 3*4 / 6 = 2.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_volume_with_citation() {
    let dir = scratch("v1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"charles-law.adj\"\n\
         observe final_volume(3)\n\
         observe final_temperature(6)\n\
         observe initial_temperature(4)\n\
         ? initial_volume(final_volume, final_temperature, initial_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3*4 / 6 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_volume\"") && s.contains("\"value\":2"),
        "initial_volume(3, 6, 4) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_volume carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_temperature — T₁ = V₁T₂ / V₂: recover the starting absolute temperature, the
// fourth exact reading of the one law. 2*6 / 3 = 4.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_temperature_with_citation() {
    let dir = scratch("t1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"charles-law.adj\"\n\
         observe initial_volume(2)\n\
         observe final_temperature(6)\n\
         observe final_volume(3)\n\
         ? initial_temperature(initial_volume, final_temperature, final_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2*6 / 3 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_temperature\"") && s.contains("\"value\":4"),
        "initial_temperature(2, 6, 3) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_temperature carries its LibreTexts citation: {s}"
    );
}
