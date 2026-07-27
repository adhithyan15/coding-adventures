//! End-to-end tests for the `chemistry/gay-lussac-law.adj` library — Gay-Lussac's law
//! (P₁/T₁ = P₂/T₂, pressure directly proportional to absolute temperature at constant
//! volume) and its FOUR exact readings (solve for any one of the initial pressure,
//! initial temperature, final pressure or final temperature) — driven through the built
//! CLI binary against the SHIPPED stdlib. Each proves the same invariant as the other
//! formula libraries: a consumer states NO arithmetic; it imports the grounded library,
//! binds the measured quantities with `observe`, and the engine applies the cited
//! relation on the CPU — computing the EXACT value and rendering the relation's citation
//! and trust tier in the `derived` section (the auditable answer). The four readings
//! INVERT around the worked case P₁ = 2, T₁ = 4, P₂ = 3, T₂ = 6 (P₁/T₁ = 2/4 = 3/6 =
//! P₂/T₂): 2*6/4 = 3, 3*4/2 = 6, 3*4/6 = 2, 2*6/3 = 4.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped gay-lussac-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_gay_lussac_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/gay-lussac-law.adj")
        .canonicalize()
        .expect("shipped gay-lussac-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_gaylussac_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_gay_lussac_lib()).unwrap();
    std::fs::write(dir.join("gay-lussac-law.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// final_pressure — the pressure a fixed amount of gas reaches at a new absolute
// temperature in a rigid vessel (P₂ = P₁T₂/T₁).
// ---------------------------------------------------------------------------

#[test]
fn imports_gay_lussac_library_and_computes_final_pressure_with_citation() {
    let dir = scratch("p2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"gay-lussac-law.adj\"\n\
         observe initial_pressure(2)\n\
         observe initial_temperature(4)\n\
         observe final_temperature(6)\n\
         ? final_pressure(initial_pressure, initial_temperature, final_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2 * 6 / 4 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"final_pressure\"") && s.contains("\"value\":3"),
        "final_pressure(2, 4, 6) = 3: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// final_temperature — the absolute temperature a fixed amount of gas reaches at a new
// pressure (T₂ = P₂T₁/P₁), which INVERTS the final pressure just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_final_temperature_with_citation() {
    let dir = scratch("t2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"gay-lussac-law.adj\"\n\
         observe final_pressure(3)\n\
         observe initial_pressure(2)\n\
         observe initial_temperature(4)\n\
         ? final_temperature(final_pressure, initial_pressure, initial_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 4 / 2 = 6, computed on the CPU.
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
// initial_pressure — the pressure a fixed amount of gas started at (P₁ = P₂T₁/T₂), the
// third exact reading of the one proportionality law.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_pressure_with_citation() {
    let dir = scratch("p1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"gay-lussac-law.adj\"\n\
         observe final_pressure(3)\n\
         observe final_temperature(6)\n\
         observe initial_temperature(4)\n\
         ? initial_pressure(final_pressure, final_temperature, initial_temperature)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 4 / 6 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_pressure\"") && s.contains("\"value\":2"),
        "initial_pressure(3, 6, 4) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_pressure carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// initial_temperature — the absolute temperature a fixed amount of gas started at
// (T₁ = P₁T₂/P₂), the fourth exact reading of the one proportionality law.
// ---------------------------------------------------------------------------

#[test]
fn computes_initial_temperature_with_citation() {
    let dir = scratch("t1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"gay-lussac-law.adj\"\n\
         observe initial_pressure(2)\n\
         observe final_temperature(6)\n\
         observe final_pressure(3)\n\
         ? initial_temperature(initial_pressure, final_temperature, final_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 * 6 / 3 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"initial_temperature\"") && s.contains("\"value\":4"),
        "initial_temperature(2, 6, 3) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "initial_temperature carries its LibreTexts citation: {s}"
    );
}
