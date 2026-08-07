//! End-to-end tests for the `clinical/glomerular-filtration-rate.adj` library — the Starling
//! determination of the glomerular filtration rate (GFR = Kf × [(P_GC − P_BS) − (π_GC − π_BS)])
//! and its five exact rearrangements — driven through the built CLI binary against the SHIPPED
//! stdlib. The same invariant as every other formula library: a consumer states NO arithmetic; it
//! imports the grounded library, binds the measured quantities with `observe`, and the engine
//! applies the cited relation on the CPU, computing the EXACT value and rendering the relation's
//! citation and trust tier in the `derived` section (the auditable answer).
//!
//! This is the first FIVE-input clinical inverter and the first of the PRODUCT-OF-A-DIFFERENCE-
//! OF-TWO-DIFFERENCES shape: an outer × (the filtration coefficient) multiplying an inner
//! subtraction of one pressure-difference from another. The six formulas INVERT around the
//! worked case Kf = 14, P_GC = 60 mmHg, P_BS = 17 mmHg, π_GC = 42 mmHg, π_BS = 12 mmHg, whose net
//! filtration pressure is (60 − 17) − (42 − 12) = 13 mmHg, so GFR = 14 × 13 = 182. Each test
//! fixes five quantities and recovers the sixth exactly. The six asserted values (182, 14, 60,
//! 17, 42, 12) are chosen so none is a substring of another rendered value, so each
//! `"value":N` assertion is unambiguous.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped glomerular-filtration-rate library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_gfr_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/glomerular-filtration-rate.adj")
        .canonicalize()
        .expect("shipped glomerular-filtration-rate.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_gfr_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_gfr_lib()).unwrap();
    std::fs::write(dir.join("glomerular-filtration-rate.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// glomerular_filtration_rate — the determination: Kf × [(P_GC − P_BS) − (π_GC − π_BS)].
// ---------------------------------------------------------------------------

#[test]
fn imports_gfr_library_and_computes_it_with_citation() {
    let dir = scratch("gfr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe filtration_coefficient(14)\n\
         observe glomerular_capillary_hydrostatic_pressure(60)\n\
         observe bowman_space_hydrostatic_pressure(17)\n\
         observe glomerular_capillary_oncotic_pressure(42)\n\
         observe bowman_space_oncotic_pressure(12)\n\
         ? glomerular_filtration_rate(filtration_coefficient, glomerular_capillary_hydrostatic_pressure, bowman_space_hydrostatic_pressure, glomerular_capillary_oncotic_pressure, bowman_space_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result:
    // 14 × [(60 − 17) − (42 − 12)] = 14 × 13 = 182.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"glomerular_filtration_rate\"") && s.contains("\"value\":182"),
        "glomerular_filtration_rate(14, 60, 17, 42, 12) = 182: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// filtration_coefficient — the same relation solved for Kf: GFR / net filtration pressure.
// ---------------------------------------------------------------------------

#[test]
fn computes_filtration_coefficient_from_gfr_and_pressures_with_citation() {
    let dir = scratch("kf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe glomerular_filtration_rate(182)\n\
         observe glomerular_capillary_hydrostatic_pressure(60)\n\
         observe bowman_space_hydrostatic_pressure(17)\n\
         observe glomerular_capillary_oncotic_pressure(42)\n\
         observe bowman_space_oncotic_pressure(12)\n\
         ? filtration_coefficient(glomerular_filtration_rate, glomerular_capillary_hydrostatic_pressure, bowman_space_hydrostatic_pressure, glomerular_capillary_oncotic_pressure, bowman_space_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 182 / [(60 − 17) − (42 − 12)] = 182 / 13 = 14, computed on the CPU.
    assert!(
        s.contains("\"name\":\"filtration_coefficient\"") && s.contains("\"value\":14"),
        "filtration_coefficient(182, 60, 17, 42, 12) = 14: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "filtration_coefficient carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// glomerular_capillary_hydrostatic_pressure — solved for P_GC: GFR/Kf + P_BS + π_GC − π_BS.
// ---------------------------------------------------------------------------

#[test]
fn computes_glomerular_capillary_hydrostatic_pressure_with_citation() {
    let dir = scratch("pgc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe glomerular_filtration_rate(182)\n\
         observe filtration_coefficient(14)\n\
         observe bowman_space_hydrostatic_pressure(17)\n\
         observe glomerular_capillary_oncotic_pressure(42)\n\
         observe bowman_space_oncotic_pressure(12)\n\
         ? glomerular_capillary_hydrostatic_pressure(glomerular_filtration_rate, filtration_coefficient, bowman_space_hydrostatic_pressure, glomerular_capillary_oncotic_pressure, bowman_space_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 182/14 + 17 + 42 − 12 = 13 + 17 + 42 − 12 = 60, computed on the CPU.
    assert!(
        s.contains("\"name\":\"glomerular_capillary_hydrostatic_pressure\"") && s.contains("\"value\":60"),
        "glomerular_capillary_hydrostatic_pressure(182, 14, 17, 42, 12) = 60: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "glomerular_capillary_hydrostatic_pressure carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// bowman_space_hydrostatic_pressure — solved for P_BS: P_GC − π_GC + π_BS − GFR/Kf.
// ---------------------------------------------------------------------------

#[test]
fn computes_bowman_space_hydrostatic_pressure_with_citation() {
    let dir = scratch("pbs");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe glomerular_filtration_rate(182)\n\
         observe filtration_coefficient(14)\n\
         observe glomerular_capillary_hydrostatic_pressure(60)\n\
         observe glomerular_capillary_oncotic_pressure(42)\n\
         observe bowman_space_oncotic_pressure(12)\n\
         ? bowman_space_hydrostatic_pressure(glomerular_filtration_rate, filtration_coefficient, glomerular_capillary_hydrostatic_pressure, glomerular_capillary_oncotic_pressure, bowman_space_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 60 − 42 + 12 − 182/14 = 60 − 42 + 12 − 13 = 17, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bowman_space_hydrostatic_pressure\"") && s.contains("\"value\":17"),
        "bowman_space_hydrostatic_pressure(182, 14, 60, 42, 12) = 17: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bowman_space_hydrostatic_pressure carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// glomerular_capillary_oncotic_pressure — solved for π_GC: P_GC − P_BS + π_BS − GFR/Kf.
// ---------------------------------------------------------------------------

#[test]
fn computes_glomerular_capillary_oncotic_pressure_with_citation() {
    let dir = scratch("pigc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe glomerular_filtration_rate(182)\n\
         observe filtration_coefficient(14)\n\
         observe glomerular_capillary_hydrostatic_pressure(60)\n\
         observe bowman_space_hydrostatic_pressure(17)\n\
         observe bowman_space_oncotic_pressure(12)\n\
         ? glomerular_capillary_oncotic_pressure(glomerular_filtration_rate, filtration_coefficient, glomerular_capillary_hydrostatic_pressure, bowman_space_hydrostatic_pressure, bowman_space_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 60 − 17 + 12 − 182/14 = 60 − 17 + 12 − 13 = 42, computed on the CPU.
    assert!(
        s.contains("\"name\":\"glomerular_capillary_oncotic_pressure\"") && s.contains("\"value\":42"),
        "glomerular_capillary_oncotic_pressure(182, 14, 60, 17, 12) = 42: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "glomerular_capillary_oncotic_pressure carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// bowman_space_oncotic_pressure — solved for π_BS: GFR/Kf − P_GC + P_BS + π_GC, the sixth reading
// of the one sentence.
// ---------------------------------------------------------------------------

#[test]
fn computes_bowman_space_oncotic_pressure_with_citation() {
    let dir = scratch("pibs");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"glomerular-filtration-rate.adj\"\n\
         observe glomerular_filtration_rate(182)\n\
         observe filtration_coefficient(14)\n\
         observe glomerular_capillary_hydrostatic_pressure(60)\n\
         observe bowman_space_hydrostatic_pressure(17)\n\
         observe glomerular_capillary_oncotic_pressure(42)\n\
         ? bowman_space_oncotic_pressure(glomerular_filtration_rate, filtration_coefficient, glomerular_capillary_hydrostatic_pressure, bowman_space_hydrostatic_pressure, glomerular_capillary_oncotic_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 182/14 − 60 + 17 + 42 = 13 − 60 + 17 + 42 = 12, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bowman_space_oncotic_pressure\"") && s.contains("\"value\":12"),
        "bowman_space_oncotic_pressure(182, 14, 60, 17, 42) = 12: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bowman_space_oncotic_pressure carries its StatPearls citation: {s}"
    );
}
