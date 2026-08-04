//! End-to-end tests for the `clinical/physiologic-dead-space.adj` library — the Bohr physiologic dead space
//! (VD = tidal volume × (PaCO2 − PeCO2) / PaCO2) and its two exact rearrangements — driven through the built
//! CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the tidal volume, the arterial CO2, and the
//! mixed-expired CO2 with `observe`, and the engine applies the cited formula on the CPU, computing the
//! EXACT value (over exact rationals) and rendering the citation and trust tier in the `derived` section
//! (the auditable answer). The three formulas INVERT around the worked case VT = 500, PaCO2 = 40, PeCO2 = 20:
//! 500 × (40 − 20) / 40 = 250 (VD), 250 × 40 / (40 − 20) = 500 (VT), 40 − 250 × 40 / 500 = 20 (PeCO2).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree carries the intermediates 20 (= 40 − 20) and 10000 (= 500 × 20), so a
//! bare numeric substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped physiologic-dead-space library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_vd_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/physiologic-dead-space.adj")
        .canonicalize()
        .expect("shipped physiologic-dead-space.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vd_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_vd_lib()).unwrap();
    std::fs::write(dir.join("physiologic-dead-space.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// dead_space_volume — the dead space: VT × (PaCO2 − PeCO2) / PaCO2.
// ---------------------------------------------------------------------------

#[test]
fn imports_dead_space_library_and_computes_it_with_citation() {
    let dir = scratch("vd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"physiologic-dead-space.adj\"\n\
         observe tidal_volume(500)\n\
         observe paco2(40)\n\
         observe peco2(20)\n\
         ? dead_space_volume(tidal_volume, paco2, peco2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 500 × (40 − 20) / 40 = 250, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 20 and 10000 intermediates cannot
    // spuriously satisfy a bare "value":250.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"dead_space_volume\",\"value\":250"),
        "dead_space_volume(500, 40, 20) = 250: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — the same equation solved for the tidal volume: VD × PaCO2 / (PaCO2 − PeCO2).
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_from_dead_space_with_citation() {
    let dir = scratch("vt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"physiologic-dead-space.adj\"\n\
         observe dead_space_volume(250)\n\
         observe paco2(40)\n\
         observe peco2(20)\n\
         ? tidal_volume(dead_space_volume, paco2, peco2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 250 × 40 / (40 − 20) = 10000 / 20 = 500, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\",\"value\":500"),
        "tidal_volume(250, 40, 20) = 500: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tidal_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// peco2 — the same equation solved for the mixed-expired CO2: PaCO2 − VD × PaCO2 / VT, the third reading of
// the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_peco2_from_dead_space_with_citation() {
    let dir = scratch("peco2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"physiologic-dead-space.adj\"\n\
         observe dead_space_volume(250)\n\
         observe tidal_volume(500)\n\
         observe paco2(40)\n\
         ? peco2(dead_space_volume, tidal_volume, paco2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 40 − 250 × 40 / 500 = 40 − 10000 / 500 = 40 − 20 = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"peco2\",\"value\":20"),
        "peco2(250, 500, 40) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "peco2 carries its cited provenance: {s}"
    );
}
