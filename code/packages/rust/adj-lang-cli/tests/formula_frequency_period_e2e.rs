//! End-to-end tests for the `physics/frequency-period.adj` library — the reciprocal
//! relation between frequency and period (f = 1/T) and its single exact rearrangement
//! (T = 1/f) — driven through the built CLI binary against the SHIPPED stdlib. Each
//! proves the same invariant as the other formula libraries: a consumer states NO
//! arithmetic; it imports the grounded library, binds the measured quantity with
//! `observe`, and the engine applies the cited definition on the CPU — computing the
//! EXACT reciprocal and rendering the definition's citation + trust tier in the
//! `derived` section (the auditable answer). Unlike the two-input rate libraries this is
//! a one-input reciprocal, so the two formulas form a clean 2-way inverter:
//! 1 / 4 = 0.25 and 1 / 0.5 = 2.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped frequency-period library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_frequency_period_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/frequency-period.adj")
        .canonicalize()
        .expect("shipped frequency-period.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_freqper_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_frequency_period_lib()).unwrap();
    std::fs::write(dir.join("frequency-period.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// frequency — the definition: one divided by the period (f = 1/T).
// ---------------------------------------------------------------------------

#[test]
fn imports_frequency_period_library_and_computes_frequency_with_citation() {
    let dir = scratch("f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"frequency-period.adj\"\n\
         observe period(4)\n\
         ? frequency(period)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 1 / 4 = 0.25.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"frequency\"") && s.contains("\"value\":0.25"),
        "frequency(4) = 0.25: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// period — the same reciprocal relation solved for T: one divided by the frequency,
// which INVERTS the frequency direction.
// ---------------------------------------------------------------------------

#[test]
fn computes_period_as_reciprocal_of_frequency_with_citation() {
    let dir = scratch("t");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"frequency-period.adj\"\n\
         observe frequency(0.5)\n\
         ? period(frequency)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1 / 0.5 = 2, computed on the CPU as an exact rational.
    assert!(
        s.contains("\"name\":\"period\"") && s.contains("\"value\":2"),
        "period(0.5) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "period carries its LibreTexts citation: {s}"
    );
}
