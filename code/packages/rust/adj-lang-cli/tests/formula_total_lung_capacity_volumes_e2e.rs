//! End-to-end tests for the `clinical/total-lung-capacity-volumes.adj` library — the
//! definition of the total lung capacity as the sum of its FOUR primary volumes
//! (TLC = RV + ERV + IRV + TV) and its four exact rearrangements (each volume = the total
//! minus the other three) — driven through the built CLI binary against the SHIPPED stdlib.
//! This is the first four-input (five-formula) n-ary inverter in the clinical track: the same
//! invariant as every other formula library, extended one dimension past the three-input GCS
//! sum. A consumer states NO arithmetic; it imports the grounded library, binds the measured
//! volumes with `observe`, and the engine applies the cited relation on the CPU, computing the
//! EXACT value and rendering the relation's citation and trust tier in the `derived` section
//! (the auditable answer). The five formulas INVERT around the worked case
//! RV = 2, ERV = 3, IRV = 4, TV = 5: 2 + 3 + 4 + 5 = 14, and 14 minus any three volumes
//! recovers the fourth.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped total-lung-capacity-volumes library, resolved from this
/// crate's manifest dir so the test is location-independent.
fn shipped_tlc_volumes_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/total-lung-capacity-volumes.adj")
        .canonicalize()
        .expect("shipped total-lung-capacity-volumes.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_tlcvol_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_tlc_volumes_lib()).unwrap();
    std::fs::write(dir.join("total-lung-capacity-volumes.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// total_lung_capacity — the definition: the sum of the four primary volumes.
// ---------------------------------------------------------------------------

#[test]
fn imports_tlc_volumes_library_and_computes_total_with_citation() {
    let dir = scratch("total");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity-volumes.adj\"\n\
         observe residual_volume(2)\n\
         observe expiratory_reserve_volume(3)\n\
         observe inspiratory_reserve_volume(4)\n\
         observe tidal_volume(5)\n\
         ? total_lung_capacity(residual_volume, expiratory_reserve_volume, inspiratory_reserve_volume, tidal_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2 + 3 + 4 + 5 = 14.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"total_lung_capacity\"") && s.contains("\"value\":14"),
        "total_lung_capacity(2, 3, 4, 5) = 14: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// residual_volume — the same relation solved for RV: TLC − ERV − IRV − TV.
// ---------------------------------------------------------------------------

#[test]
fn computes_residual_volume_from_total_and_others_with_citation() {
    let dir = scratch("rv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity-volumes.adj\"\n\
         observe total_lung_capacity(14)\n\
         observe expiratory_reserve_volume(3)\n\
         observe inspiratory_reserve_volume(4)\n\
         observe tidal_volume(5)\n\
         ? residual_volume(total_lung_capacity, expiratory_reserve_volume, inspiratory_reserve_volume, tidal_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 14 - 3 - 4 - 5 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"residual_volume\"") && s.contains("\"value\":2"),
        "residual_volume(14, 3, 4, 5) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "residual_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// expiratory_reserve_volume — solved for ERV: TLC − RV − IRV − TV.
// ---------------------------------------------------------------------------

#[test]
fn computes_expiratory_reserve_volume_from_total_and_others_with_citation() {
    let dir = scratch("erv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity-volumes.adj\"\n\
         observe total_lung_capacity(14)\n\
         observe residual_volume(2)\n\
         observe inspiratory_reserve_volume(4)\n\
         observe tidal_volume(5)\n\
         ? expiratory_reserve_volume(total_lung_capacity, residual_volume, inspiratory_reserve_volume, tidal_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 14 - 2 - 4 - 5 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"expiratory_reserve_volume\"") && s.contains("\"value\":3"),
        "expiratory_reserve_volume(14, 2, 4, 5) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "expiratory_reserve_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// inspiratory_reserve_volume — solved for IRV: TLC − RV − ERV − TV.
// ---------------------------------------------------------------------------

#[test]
fn computes_inspiratory_reserve_volume_from_total_and_others_with_citation() {
    let dir = scratch("irv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity-volumes.adj\"\n\
         observe total_lung_capacity(14)\n\
         observe residual_volume(2)\n\
         observe expiratory_reserve_volume(3)\n\
         observe tidal_volume(5)\n\
         ? inspiratory_reserve_volume(total_lung_capacity, residual_volume, expiratory_reserve_volume, tidal_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 14 - 2 - 3 - 5 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"inspiratory_reserve_volume\"") && s.contains("\"value\":4"),
        "inspiratory_reserve_volume(14, 2, 3, 5) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "inspiratory_reserve_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — solved for TV: TLC − RV − ERV − IRV, the fifth reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_from_total_and_others_with_citation() {
    let dir = scratch("tv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity-volumes.adj\"\n\
         observe total_lung_capacity(14)\n\
         observe residual_volume(2)\n\
         observe expiratory_reserve_volume(3)\n\
         observe inspiratory_reserve_volume(4)\n\
         ? tidal_volume(total_lung_capacity, residual_volume, expiratory_reserve_volume, inspiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 14 - 2 - 3 - 4 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\"") && s.contains("\"value\":5"),
        "tidal_volume(14, 2, 3, 4) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tidal_volume carries its StatPearls citation: {s}"
    );
}
