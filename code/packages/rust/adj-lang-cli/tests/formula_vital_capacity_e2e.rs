//! End-to-end tests for the `clinical/vital-capacity.adj` library — the vital-capacity relation
//! (VC = tidal volume + inspiratory reserve volume + expiratory reserve volume) and its three exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured lung volumes with `observe`, and the engine applies the cited
//! definition on the CPU, computing the EXACT value and rendering the definition's citation and trust
//! tier in the `derived` section (the auditable answer). The four formulas INVERT around the worked
//! case tidal volume = 1, inspiratory reserve volume = 3, expiratory reserve volume = 2: 1 + 3 + 2 = 6
//! (VC), 6 − 3 − 2 = 1 (TV), 6 − 1 − 2 = 3 (IRV), 6 − 1 − 3 = 2 (ERV). The four asserted values
//! (6, 1, 3, 2) are distinct single digits, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped vital-capacity library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_vc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/vital-capacity.adj")
        .canonicalize()
        .expect("shipped vital-capacity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_vc_lib()).unwrap();
    std::fs::write(dir.join("vital-capacity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// vital_capacity — the relation: tidal volume + inspiratory reserve + expiratory reserve.
// ---------------------------------------------------------------------------

#[test]
fn imports_vital_capacity_library_and_computes_it_with_citation() {
    let dir = scratch("vc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vital-capacity.adj\"\n\
         observe tidal_volume(1)\n\
         observe inspiratory_reserve_volume(3)\n\
         observe expiratory_reserve_volume(2)\n\
         ? vital_capacity(tidal_volume, inspiratory_reserve_volume, expiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 1 + 3 + 2 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"vital_capacity\"") && s.contains("\"value\":6"),
        "vital_capacity(1, 3, 2) = 6: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — the same definition solved for the tidal volume: VC − IRV − ERV.
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_from_capacity_and_reserves_with_citation() {
    let dir = scratch("tv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vital-capacity.adj\"\n\
         observe vital_capacity(6)\n\
         observe inspiratory_reserve_volume(3)\n\
         observe expiratory_reserve_volume(2)\n\
         ? tidal_volume(vital_capacity, inspiratory_reserve_volume, expiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 − 3 − 2 = 1, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\"") && s.contains("\"value\":1"),
        "tidal_volume(6, 3, 2) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tidal_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// inspiratory_reserve_volume — the same definition solved for the IRV: VC − TV − ERV.
// ---------------------------------------------------------------------------

#[test]
fn computes_irv_from_capacity_tidal_and_erv_with_citation() {
    let dir = scratch("irv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vital-capacity.adj\"\n\
         observe vital_capacity(6)\n\
         observe tidal_volume(1)\n\
         observe expiratory_reserve_volume(2)\n\
         ? inspiratory_reserve_volume(vital_capacity, tidal_volume, expiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 − 1 − 2 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"inspiratory_reserve_volume\"") && s.contains("\"value\":3"),
        "inspiratory_reserve_volume(6, 1, 2) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "inspiratory_reserve_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// expiratory_reserve_volume — the same definition solved for the ERV: VC − TV − IRV, the fourth
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_erv_from_capacity_tidal_and_irv_with_citation() {
    let dir = scratch("erv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vital-capacity.adj\"\n\
         observe vital_capacity(6)\n\
         observe tidal_volume(1)\n\
         observe inspiratory_reserve_volume(3)\n\
         ? expiratory_reserve_volume(vital_capacity, tidal_volume, inspiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 − 1 − 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"expiratory_reserve_volume\"") && s.contains("\"value\":2"),
        "expiratory_reserve_volume(6, 1, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "expiratory_reserve_volume carries its cited provenance: {s}"
    );
}
