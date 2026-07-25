//! End-to-end tests for the `chemistry/boyles-law.adj` library — Boyle's law
//! (P₁V₁ = P₂V₂) in all FOUR exact readings — driven through the built CLI binary
//! against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library, binds
//! the measured quantities with `observe`, and the engine applies the cited relation on
//! the CPU — computing the EXACT value and rendering the relation's citation + trust tier
//! in the `derived` section (the auditable answer). The four readings all turn on the
//! same conserved product P₁V₁ = P₂V₂ = 12, around P₁=6, V₁=2, P₂=3, V₂=4.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped Boyle's-law library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_boyles_law_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/boyles-law.adj")
        .canonicalize()
        .expect("shipped boyles-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_boyle_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_boyles_law_lib()).unwrap();
    std::fs::write(dir.join("boyles-law.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// final_volume — V₂ = P₁V₁ / P₂: a fixed amount of gas reaches this volume at a new
// pressure. 6*2 / 3 = 4.
// ---------------------------------------------------------------------------

#[test]
fn imports_boyles_law_library_and_computes_final_volume_with_citation() {
    let dir = scratch("v2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"boyles-law.adj\"\n\
         observe initial_pressure(6)\n\
         observe initial_volume(2)\n\
         observe final_pressure(3)\n\
         ? final_volume(initial_pressure, initial_volume, final_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 6*2 / 3 = 4.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"final_volume\"") && s.contains("\"value\":4"),
        "final_volume(6, 2, 3) = 4: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// final_pressure — P₂ = P₁V₁ / V₂: the pressure after squeezing to a new volume.
// 6*2 / 4 = 3.
// ---------------------------------------------------------------------------

#[test]
fn computes_final_pressure_with_citation() {
    let dir = scratch("p2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"boyles-law.adj\"\n\
         observe initial_pressure(6)\n\
         observe initial_volume(2)\n\
         observe final_volume(4)\n\
         ? final_pressure(initial_pressure, initial_volume, final_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6*2 / 4 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"final_pressure\"") && s.contains("\"value\":3"),
        "final_pressure(6, 2, 4) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "final_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_pressure — P₁ = P₂V₂ / V₁: recover the starting pressure. 3*4 / 2 = 6.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_pressure_with_citation() {
    let dir = scratch("p1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"boyles-law.adj\"\n\
         observe final_pressure(3)\n\
         observe final_volume(4)\n\
         observe initial_volume(2)\n\
         ? initial_pressure(final_pressure, final_volume, initial_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3*4 / 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_pressure\"") && s.contains("\"value\":6"),
        "initial_pressure(3, 4, 2) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_volume — V₁ = P₂V₂ / P₁: recover the starting volume, the fourth exact
// reading of the one law. 3*4 / 6 = 2.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_volume_with_citation() {
    let dir = scratch("v1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"boyles-law.adj\"\n\
         observe final_pressure(3)\n\
         observe final_volume(4)\n\
         observe initial_pressure(6)\n\
         ? initial_volume(final_pressure, final_volume, initial_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3*4 / 6 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_volume\"") && s.contains("\"value\":2"),
        "initial_volume(3, 4, 6) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_volume carries its LibreTexts citation: {s}"
    );
}
