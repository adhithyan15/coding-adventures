//! End-to-end tests for the `clinical/parkland-formula.adj` library — the Parkland burn resuscitation
//! formula (total 24-hour fluid = 4 × weight × %TBSA) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the weight and
//! burned surface area with `observe`, and the engine applies the cited formula on the CPU, computing
//! the EXACT value (over exact rationals) and rendering the citation and trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT around the worked case weight = 80,
//! TBSA = 30: 4 × 80 × 30 = 9600 (total fluid), 9600 / (4 × 30) = 80 (weight),
//! 9600 / (4 × 80) = 30 (%TBSA). The three asserted values (9600, 80, 30) are distinct, none a
//! colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped parkland-formula library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_parkland_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/parkland-formula.adj")
        .canonicalize()
        .expect("shipped parkland-formula.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_parkland_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_parkland_lib()).unwrap();
    std::fs::write(dir.join("parkland-formula.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// total_fluid — the resuscitation volume: 4 × weight × %TBSA.
// ---------------------------------------------------------------------------

#[test]
fn imports_parkland_library_and_computes_fluid_with_citation() {
    let dir = scratch("fluid");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"parkland-formula.adj\"\n\
         observe weight(80)\n\
         observe tbsa(30)\n\
         ? total_fluid(weight, tbsa)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 4 × 80 × 30 = 9600, computed
    // EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"total_fluid\"") && s.contains("\"value\":9600"),
        "total_fluid(80, 30) = 9600: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// weight — the same equation solved for the weight: total_fluid / (4 × %TBSA).
// ---------------------------------------------------------------------------

#[test]
fn computes_weight_from_parkland_fluid_with_citation() {
    let dir = scratch("weight");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"parkland-formula.adj\"\n\
         observe total_fluid(9600)\n\
         observe tbsa(30)\n\
         ? weight(total_fluid, tbsa)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 9600 / (4 × 30) = 9600 / 120 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"weight\"") && s.contains("\"value\":80"),
        "weight(9600, 30) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "weight carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tbsa — the same equation solved for the burn size: total_fluid / (4 × weight), the third reading of
// the one formula.
// ---------------------------------------------------------------------------

#[test]
fn computes_tbsa_from_parkland_fluid_with_citation() {
    let dir = scratch("tbsa");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"parkland-formula.adj\"\n\
         observe total_fluid(9600)\n\
         observe weight(80)\n\
         ? tbsa(total_fluid, weight)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 9600 / (4 × 80) = 9600 / 320 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tbsa\"") && s.contains("\"value\":30"),
        "tbsa(9600, 80) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tbsa carries its cited provenance: {s}"
    );
}
