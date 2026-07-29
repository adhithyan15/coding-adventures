//! End-to-end tests for the `clinical/cardiac-index.adj` library — the definition of the
//! cardiac index (cardiac index = cardiac output ÷ body surface area) and its two exact
//! rearrangements (cardiac output = cardiac index × body surface area, body surface area =
//! cardiac output ÷ cardiac index) — driven through the built CLI binary against the SHIPPED
//! stdlib. Each proves the same invariant as the other formula libraries: a consumer states NO
//! arithmetic; it imports the grounded library, binds the measured quantities with `observe`,
//! and the engine applies the cited relation on the CPU, computing the EXACT value and
//! rendering the relation's citation and trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT around the worked case CO = 6 L/min, BSA = 2 m²:
//! 6 ÷ 2 = 3, and both 3 × 2 = 6 and 6 ÷ 3 = 2 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped cardiac-index library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_ci_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/cardiac-index.adj")
        .canonicalize()
        .expect("shipped cardiac-index.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ci_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ci_lib()).unwrap();
    std::fs::write(dir.join("cardiac-index.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// cardiac_index — the definition: the cardiac output over the body surface area.
// ---------------------------------------------------------------------------

#[test]
fn imports_ci_library_and_computes_cardiac_index_with_citation() {
    let dir = scratch("ci");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-index.adj\"\n\
         observe cardiac_output(6)\n\
         observe body_surface_area(2)\n\
         ? cardiac_index(cardiac_output, body_surface_area)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 6 / 2 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"cardiac_index\"") && s.contains("\"value\":3"),
        "cardiac_index(6, 2) = 3: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// cardiac_output — the same relation solved for CO: CI × BSA, which INVERTS the cardiac index
// just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_cardiac_output_from_ci_and_bsa_with_citation() {
    let dir = scratch("co");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-index.adj\"\n\
         observe cardiac_index(3)\n\
         observe body_surface_area(2)\n\
         ? cardiac_output(cardiac_index, body_surface_area)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"cardiac_output\"") && s.contains("\"value\":6"),
        "cardiac_output(3, 2) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "cardiac_output carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// body_surface_area — the same relation solved for BSA: CO ÷ CI, the third exact reading of
// the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_bsa_from_co_and_ci_with_citation() {
    let dir = scratch("bsa");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cardiac-index.adj\"\n\
         observe cardiac_output(6)\n\
         observe cardiac_index(3)\n\
         ? body_surface_area(cardiac_output, cardiac_index)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"body_surface_area\"") && s.contains("\"value\":2"),
        "body_surface_area(6, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "body_surface_area carries its StatPearls citation: {s}"
    );
}
