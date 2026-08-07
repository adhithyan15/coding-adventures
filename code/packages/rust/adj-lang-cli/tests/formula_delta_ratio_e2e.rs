//! End-to-end tests for the `clinical/delta-ratio.adj` library — the delta ratio (delta-delta / delta
//! gap ratio, (anion gap − 12) / (24 − bicarbonate)) and its two exact rearrangements — driven through
//! the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library:
//! a consumer states NO arithmetic; it imports the grounded library, binds the anion gap and
//! bicarbonate with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT
//! value (over exact rationals — 18/10 = 9/5) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case anion
//! gap = 30, bicarbonate = 14: (30 − 12)/(24 − 14) = 18/10 = 1.8 (delta ratio),
//! 1.8 × (24 − 14) + 12 = 30 (anion gap), 24 − (30 − 12)/1.8 = 14 (bicarbonate). The three asserted
//! values (1.8, 30, 14) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped delta-ratio library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_delta_ratio_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/delta-ratio.adj")
        .canonicalize()
        .expect("shipped delta-ratio.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_deltaratio_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_delta_ratio_lib()).unwrap();
    std::fs::write(dir.join("delta-ratio.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// delta_ratio — the ratio: (anion gap − 12) / (24 − bicarbonate).
// ---------------------------------------------------------------------------

#[test]
fn imports_delta_ratio_library_and_computes_it_with_citation() {
    let dir = scratch("ratio");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"delta-ratio.adj\"\n\
         observe anion_gap(30)\n\
         observe bicarbonate(14)\n\
         ? delta_ratio(anion_gap, bicarbonate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (30 − 12)/(24 − 14) = 18/10 =
    // 1.8, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"delta_ratio\"") && s.contains("\"value\":1.8"),
        "delta_ratio(30, 14) = 1.8: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// anion_gap — the same equation solved for the anion gap: delta_ratio × (24 − bicarbonate) + 12.
// ---------------------------------------------------------------------------

#[test]
fn computes_anion_gap_from_delta_ratio_with_citation() {
    let dir = scratch("ag");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"delta-ratio.adj\"\n\
         observe delta_ratio(1.8)\n\
         observe bicarbonate(14)\n\
         ? anion_gap(delta_ratio, bicarbonate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1.8 × (24 − 14) + 12 = 18 + 12 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"anion_gap\"") && s.contains("\"value\":30"),
        "anion_gap(1.8, 14) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "anion_gap carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// bicarbonate — the same equation solved for the bicarbonate: 24 − (anion gap − 12) / delta_ratio, the
// third reading of the one ratio.
// ---------------------------------------------------------------------------

#[test]
fn computes_bicarbonate_from_delta_ratio_with_citation() {
    let dir = scratch("hco3");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"delta-ratio.adj\"\n\
         observe delta_ratio(1.8)\n\
         observe anion_gap(30)\n\
         ? bicarbonate(delta_ratio, anion_gap)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 24 − (30 − 12) / 1.8 = 24 − 10 = 14, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bicarbonate\"") && s.contains("\"value\":14"),
        "bicarbonate(1.8, 30) = 14: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bicarbonate carries its cited provenance: {s}"
    );
}
