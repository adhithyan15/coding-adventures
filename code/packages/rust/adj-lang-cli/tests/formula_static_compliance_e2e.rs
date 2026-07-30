//! End-to-end tests for the `clinical/static-compliance.adj` library — static respiratory-system
//! compliance (Cstat = tidal volume / (plateau pressure − PEEP)) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the tidal volume, the
//! plateau pressure, and the PEEP with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case V = 500,
//! Pplat = 20, PEEP = 10: 500 / (20 − 10) = 50 (Cstat), 50 × (20 − 10) = 500 (V), 500 / 50 + 10 = 20 (Pplat).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the compliance result 50 is a leading-digit prefix of the tidal-volume 500, and the
//! derivation tree also carries the driving-pressure intermediate 10, so a bare `"value":50` substring could
//! spuriously match `"value":500`. The adjacent, name-anchored form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped static-compliance library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_cstat_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/static-compliance.adj")
        .canonicalize()
        .expect("shipped static-compliance.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cstat_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cstat_lib()).unwrap();
    std::fs::write(dir.join("static-compliance.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// static_compliance — the compliance: tidal volume / (plateau − PEEP).
// ---------------------------------------------------------------------------

#[test]
fn imports_static_compliance_library_and_computes_it_with_citation() {
    let dir = scratch("cstat");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"static-compliance.adj\"\n\
         observe tidal_volume(500)\n\
         observe plateau_pressure(20)\n\
         observe peep(10)\n\
         ? static_compliance(tidal_volume, plateau_pressure, peep)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 500 / (20 − 10) = 50, computed EXACTLY
    // over rationals. Match the adjacent name/value pair so the tidal-volume 500 and the 10 intermediate in
    // the derivation cannot spuriously satisfy a bare "value":50.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"static_compliance\",\"value\":50"),
        "static_compliance(500, 20, 10) = 50: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tidal_volume — the same equation solved for the volume: Cstat × (plateau − PEEP).
// ---------------------------------------------------------------------------

#[test]
fn computes_tidal_volume_from_compliance_with_citation() {
    let dir = scratch("vt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"static-compliance.adj\"\n\
         observe static_compliance(50)\n\
         observe plateau_pressure(20)\n\
         observe peep(10)\n\
         ? tidal_volume(static_compliance, plateau_pressure, peep)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 50 × (20 − 10) = 50 × 10 = 500, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tidal_volume\",\"value\":500"),
        "tidal_volume(50, 20, 10) = 500: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tidal_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plateau_pressure — the same equation solved for the plateau pressure: V / Cstat + PEEP, the third reading
// of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_plateau_pressure_from_compliance_with_citation() {
    let dir = scratch("pplat");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"static-compliance.adj\"\n\
         observe static_compliance(50)\n\
         observe tidal_volume(500)\n\
         observe peep(10)\n\
         ? plateau_pressure(static_compliance, tidal_volume, peep)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 500 / 50 + 10 = 10 + 10 = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plateau_pressure\",\"value\":20"),
        "plateau_pressure(50, 500, 10) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plateau_pressure carries its cited provenance: {s}"
    );
}
