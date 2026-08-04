//! End-to-end tests for the `physics/hookes-law.adj` library — Hooke's law (the
//! magnitude relation F = k·x) and its two exact rearrangements (k = F/x, x = F/k) —
//! driven through the built CLI binary against the SHIPPED stdlib. Each proves the
//! same invariant as the other formula libraries: a consumer states NO arithmetic; it
//! imports the grounded library, binds the measured quantities with `observe`, and the
//! engine applies the cited relation on the CPU — computing the EXACT value and
//! rendering the relation's citation + trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT: 3 * 2 = 6, 6 / 2 = 3, 6 / 3 = 2.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped Hooke's-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_hookes_law_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/hookes-law.adj")
        .canonicalize()
        .expect("shipped hookes-law.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_hooke_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_hookes_law_lib()).unwrap();
    std::fs::write(dir.join("hookes-law.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// spring force — the relation: the force constant times the displacement (F = k·x).
// ---------------------------------------------------------------------------

#[test]
fn imports_hookes_law_library_and_computes_force_with_citation() {
    let dir = scratch("f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hookes-law.adj\"\n\
         observe spring_constant(3)\n\
         observe displacement(2)\n\
         ? spring_force(spring_constant, displacement)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 3 * 2 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"spring_force\"") && s.contains("\"value\":6"),
        "spring_force(3, 2) = 6: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// force constant — the same relation solved for k: the force over the displacement,
// which INVERTS the spring force just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_spring_constant_as_force_over_displacement_with_citation() {
    let dir = scratch("k");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hookes-law.adj\"\n\
         observe spring_force(6)\n\
         observe displacement(2)\n\
         ? spring_constant(spring_force, displacement)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 2 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"spring_constant\"") && s.contains("\"value\":3"),
        "spring_constant(6, 2) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "spring_constant carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// displacement — the same relation solved for x: the force over the force constant,
// the third exact reading of the one relation.
// ---------------------------------------------------------------------------

#[test]
fn computes_displacement_as_force_over_spring_constant_with_citation() {
    let dir = scratch("x");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hookes-law.adj\"\n\
         observe spring_force(6)\n\
         observe spring_constant(3)\n\
         ? displacement(spring_force, spring_constant)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"displacement\"") && s.contains("\"value\":2"),
        "displacement(6, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("phys.libretexts.org"),
        "displacement carries its LibreTexts citation: {s}"
    );
}
