//! End-to-end tests for the `clinical/alveolar-ventilation.adj` library — the definition of
//! alveolar ventilation (VA = (tidal volume − dead space) × respiratory rate) and its three
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. This
//! is the first clinical inverter of the PRODUCT-OF-A-DIFFERENCE shape: a subtraction inside, a
//! multiplication (or its inverse division) outside, evaluated exactly over rationals. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the
//! grounded library, binds the measured quantities with `observe`, and the engine applies the
//! cited relation on the CPU, computing the EXACT value and rendering the relation's citation
//! and trust tier in the `derived` section. The four formulas INVERT around the worked case
//! VT = 500, VD = 150, RR = 12: (500 − 150) × 12 = 4200; 4200 / (500 − 150) = 12;
//! 4200/12 + 150 = 500; 500 − 4200/12 = 150.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped alveolar-ventilation library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_alveolar_ventilation_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/alveolar-ventilation.adj")
        .canonicalize()
        .expect("shipped alveolar-ventilation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_alvvent_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_alveolar_ventilation_lib()).unwrap();
    std::fs::write(dir.join("alveolar-ventilation.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// alveolar_ventilation — the definition: (tidal volume − dead space) × respiratory rate.
// ---------------------------------------------------------------------------

#[test]
fn imports_alveolar_ventilation_library_and_computes_it_with_citation() {
    let dir = scratch("va");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-ventilation.adj\"\n\
         observe tidal_volume(500)\n\
         observe dead_space(150)\n\
         observe respiratory_rate(12)\n\
         ? alveolar_ventilation(tidal_volume, dead_space, respiratory_rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied equation's result: (500 − 150) × 12 = 4200.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"alveolar_ventilation\"") && s.contains("\"value\":4200"),
        "alveolar_ventilation(500, 150, 12) = 4200: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied equation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// respiratory_rate — the same equation solved for RR: VA / (VT − VD).
// ---------------------------------------------------------------------------

#[test]
fn computes_respiratory_rate_from_va_vt_vd_with_citation() {
    let dir = scratch("rr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-ventilation.adj\"\n\
         observe alveolar_ventilation(4200)\n\
         observe tidal_volume(500)\n\
         observe dead_space(150)\n\
         ? respiratory_rate(alveolar_ventilation, tidal_volume, dead_space)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4200 / (500 − 150) = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"respiratory_rate\"") && s.contains("\"value\":12"),
        "respiratory_rate(4200, 500, 150) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "respiratory_rate carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — the same equation solved for VT: VA/RR + VD.
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_from_va_rr_vd_with_citation() {
    let dir = scratch("vt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-ventilation.adj\"\n\
         observe alveolar_ventilation(4200)\n\
         observe respiratory_rate(12)\n\
         observe dead_space(150)\n\
         ? tidal_volume(alveolar_ventilation, respiratory_rate, dead_space)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4200/12 + 150 = 350 + 150 = 500, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\"") && s.contains("\"value\":500"),
        "tidal_volume(4200, 12, 150) = 500: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tidal_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// dead_space — the same equation solved for VD: VT − VA/RR, the fourth reading of the one
// definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_dead_space_from_vt_va_rr_with_citation() {
    let dir = scratch("vd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-ventilation.adj\"\n\
         observe tidal_volume(500)\n\
         observe alveolar_ventilation(4200)\n\
         observe respiratory_rate(12)\n\
         ? dead_space(tidal_volume, alveolar_ventilation, respiratory_rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 500 − 4200/12 = 500 − 350 = 150, computed on the CPU.
    assert!(
        s.contains("\"name\":\"dead_space\"") && s.contains("\"value\":150"),
        "dead_space(500, 4200, 12) = 150: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "dead_space carries its StatPearls citation: {s}"
    );
}
