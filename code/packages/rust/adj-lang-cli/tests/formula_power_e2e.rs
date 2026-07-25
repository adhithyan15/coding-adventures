//! End-to-end tests for the `physics/power.adj` library — mechanical power
//! (P = W/t) and its two exact rearrangements (W = P·t, t = W/P) — driven through the
//! built CLI binary against the SHIPPED stdlib. Each proves the same invariant as the
//! other formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured quantities with `observe`, and the engine applies the
//! cited definition on the CPU — computing the EXACT value and rendering the
//! definition's citation + trust tier in the `derived` section (the auditable answer).
//! The three formulas INVERT: 6 / 2 = 3, 3 * 2 = 6, 6 / 3 = 2.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped power library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_power_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/power.adj")
        .canonicalize()
        .expect("shipped power.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_power_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_power_lib()).unwrap();
    std::fs::write(dir.join("power.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// power — the definition: the work done divided by the time taken (P = W/t).
// ---------------------------------------------------------------------------

#[test]
fn imports_power_library_and_computes_definition_with_citation() {
    let dir = scratch("p");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"power.adj\"\n\
         observe work(6)\n\
         observe time(2)\n\
         ? power(work, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 6 / 2 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"power\"") && s.contains("\"value\":3"),
        "power(6, 2) = 3: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// work — the same definition solved for W: the power times the time, which INVERTS
// the power just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_work_as_power_times_time_with_citation() {
    let dir = scratch("w");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"power.adj\"\n\
         observe power(3)\n\
         observe time(2)\n\
         ? work(power, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"work\"") && s.contains("\"value\":6"),
        "work(3, 2) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "work carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// time — the same definition solved for t: the work over the power, the third exact
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_time_as_work_over_power_with_citation() {
    let dir = scratch("t");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"power.adj\"\n\
         observe work(6)\n\
         observe power(3)\n\
         ? time(work, power)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"time\"") && s.contains("\"value\":2"),
        "time(6, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "time carries its LibreTexts citation: {s}"
    );
}
