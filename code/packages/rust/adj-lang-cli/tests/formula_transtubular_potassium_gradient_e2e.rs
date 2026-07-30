//! End-to-end tests for the `clinical/transtubular-potassium-gradient.adj` library — the transtubular
//! potassium gradient (TTKG = (urine K / serum K) / (urine osmolality / serum osmolality)) and its four
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the four laboratory values with `observe`, and the engine applies the cited formula
//! on the CPU, computing the EXACT value (over exact rationals — the gradient carries no constant) and
//! rendering the citation and trust tier in the `derived` section (the auditable answer). The five
//! formulas INVERT around the worked case urine K = 63, serum K = 7, urine osmolality = 610, serum
//! osmolality = 305: (63/7)/(610/305) = 9/2 = 4.5 (TTKG), and each inverse back-solves its own input —
//! 4.5 × 7 × 610 / 305 = 63 (urine K), 63 × 305 / (4.5 × 610) = 7 (serum K),
//! 63 × 305 / (4.5 × 7) = 610 (urine osm), 4.5 × 7 × 610 / 63 = 305 (serum osm). The five asserted
//! values (4.5, 63, 7, 610, 305) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped transtubular-potassium-gradient library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_ttkg_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/transtubular-potassium-gradient.adj")
        .canonicalize()
        .expect("shipped transtubular-potassium-gradient.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ttkg_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ttkg_lib()).unwrap();
    std::fs::write(dir.join("transtubular-potassium-gradient.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// transtubular_potassium_gradient — the gradient: (uK/sK) / (uOsm/sOsm).
// ---------------------------------------------------------------------------

#[test]
fn imports_ttkg_library_and_computes_it_with_citation() {
    let dir = scratch("ttkg");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transtubular-potassium-gradient.adj\"\n\
         observe urine_potassium(63)\n\
         observe serum_potassium(7)\n\
         observe urine_osmolality(610)\n\
         observe serum_osmolality(305)\n\
         ? transtubular_potassium_gradient(urine_potassium, serum_potassium, urine_osmolality, serum_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (63/7)/(610/305) = 9/2 = 4.5,
    // computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"transtubular_potassium_gradient\"") && s.contains("\"value\":4.5"),
        "transtubular_potassium_gradient(63, 7, 610, 305) = 4.5: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_potassium — the same equation solved for the urine potassium: TTKG × sK × uOsm / sOsm.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_potassium_from_ttkg_with_citation() {
    let dir = scratch("uk");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transtubular-potassium-gradient.adj\"\n\
         observe transtubular_potassium_gradient(4.5)\n\
         observe serum_potassium(7)\n\
         observe urine_osmolality(610)\n\
         observe serum_osmolality(305)\n\
         ? urine_potassium(transtubular_potassium_gradient, serum_potassium, urine_osmolality, serum_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4.5 × 7 × 610 / 305 = 63, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_potassium\"") && s.contains("\"value\":63"),
        "urine_potassium(4.5, 7, 610, 305) = 63: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_potassium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_potassium — the same equation solved for the serum potassium: uK × sOsm / (TTKG × uOsm).
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_potassium_from_ttkg_with_citation() {
    let dir = scratch("sk");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transtubular-potassium-gradient.adj\"\n\
         observe transtubular_potassium_gradient(4.5)\n\
         observe urine_potassium(63)\n\
         observe urine_osmolality(610)\n\
         observe serum_osmolality(305)\n\
         ? serum_potassium(transtubular_potassium_gradient, urine_potassium, urine_osmolality, serum_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 63 × 305 / (4.5 × 610) = 7, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_potassium\"") && s.contains("\"value\":7"),
        "serum_potassium(4.5, 63, 610, 305) = 7: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_potassium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_osmolality — the same equation solved for the urine osmolality: uK × sOsm / (TTKG × sK).
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_osmolality_from_ttkg_with_citation() {
    let dir = scratch("uosm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transtubular-potassium-gradient.adj\"\n\
         observe transtubular_potassium_gradient(4.5)\n\
         observe urine_potassium(63)\n\
         observe serum_potassium(7)\n\
         observe serum_osmolality(305)\n\
         ? urine_osmolality(transtubular_potassium_gradient, urine_potassium, serum_potassium, serum_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 63 × 305 / (4.5 × 7) = 610, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_osmolality\"") && s.contains("\"value\":610"),
        "urine_osmolality(4.5, 63, 7, 305) = 610: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_osmolality carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_osmolality — the same equation solved for the serum osmolality: TTKG × sK × uOsm / uK, the
// fifth reading of the one gradient.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_osmolality_from_ttkg_with_citation() {
    let dir = scratch("sosm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transtubular-potassium-gradient.adj\"\n\
         observe transtubular_potassium_gradient(4.5)\n\
         observe urine_potassium(63)\n\
         observe serum_potassium(7)\n\
         observe urine_osmolality(610)\n\
         ? serum_osmolality(transtubular_potassium_gradient, urine_potassium, serum_potassium, urine_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4.5 × 7 × 610 / 63 = 305, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_osmolality\"") && s.contains("\"value\":305"),
        "serum_osmolality(4.5, 63, 7, 610) = 305: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_osmolality carries its cited provenance: {s}"
    );
}
