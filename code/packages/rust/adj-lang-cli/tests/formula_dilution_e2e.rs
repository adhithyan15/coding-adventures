//! End-to-end tests for the `chemistry/dilution.adj` library — the dilution
//! equation (M₁V₁ = M₂V₂) and its four exact rearrangements (solve for the final
//! concentration, the final volume, the initial concentration, or the initial
//! volume) — driven through the built CLI binary against the SHIPPED stdlib. Each
//! proves the same invariant as the other formula libraries: a consumer states NO
//! arithmetic; it imports the grounded library, binds the measured quantities with
//! `observe`, and the engine applies the cited conservation law on the CPU —
//! computing the EXACT value and rendering the relation's citation + trust tier in
//! the `derived` section (the auditable answer). The four readings invert around one
//! conserved product M₁V₁ = M₂V₂ = 12: with M₁=4, V₁=3, M₂=2, V₂=6, each of the four
//! recovers the fourth quantity from the other three.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped dilution library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_dilution_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/dilution.adj")
        .canonicalize()
        .expect("shipped dilution.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_dilut_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_dilution_lib()).unwrap();
    std::fs::write(dir.join("dilution.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// final concentration — M₂ = M₁V₁ / V₂: the conserved product over the new volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_dilution_library_and_computes_final_concentration_with_citation() {
    let dir = scratch("m2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dilution.adj\"\n\
         observe initial_concentration(4)\n\
         observe initial_volume(3)\n\
         observe final_volume(6)\n\
         ? final_concentration(initial_concentration, initial_volume, final_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 4 * 3 / 6 = 2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"final_concentration\"") && s.contains("\"value\":2"),
        "final_concentration(4, 3, 6) = 2: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// final volume — V₂ = M₁V₁ / M₂: the volume needed to hit a target concentration.
// ---------------------------------------------------------------------------

#[test]
fn computes_final_volume_with_citation() {
    let dir = scratch("v2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dilution.adj\"\n\
         observe initial_concentration(4)\n\
         observe initial_volume(3)\n\
         observe final_concentration(2)\n\
         ? final_volume(initial_concentration, initial_volume, final_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 * 3 / 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"final_volume\"") && s.contains("\"value\":6"),
        "final_volume(4, 3, 2) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "final_volume carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial concentration — M₁ = M₂V₂ / V₁: the stock a known dilution came from.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_concentration_with_citation() {
    let dir = scratch("m1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dilution.adj\"\n\
         observe final_concentration(2)\n\
         observe final_volume(6)\n\
         observe initial_volume(3)\n\
         ? initial_concentration(final_concentration, final_volume, initial_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 * 6 / 3 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_concentration\"") && s.contains("\"value\":4"),
        "initial_concentration(2, 6, 3) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_concentration carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial volume — V₁ = M₂V₂ / M₁: the amount of stock a target dilution needs.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_volume_with_citation() {
    let dir = scratch("v1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dilution.adj\"\n\
         observe final_concentration(2)\n\
         observe final_volume(6)\n\
         observe initial_concentration(4)\n\
         ? initial_volume(final_concentration, final_volume, initial_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 * 6 / 4 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_volume\"") && s.contains("\"value\":3"),
        "initial_volume(2, 6, 4) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_volume carries its LibreTexts citation: {s}"
    );
}
