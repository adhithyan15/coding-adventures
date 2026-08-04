//! End-to-end tests for the `clinical/fractional-shortening.adj` library — left ventricular fractional
//! shortening (FS = (LVIDd − LVIDs) / LVIDd × 100) and its two exact rearrangements — driven through the
//! built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the two diameters with `observe`,
//! and the engine applies the cited formula on the CPU, computing the EXACT value (over exact rationals)
//! and rendering the citation and trust tier in the `derived` section (the auditable answer). The three
//! formulas INVERT around the worked case LVIDd = 50, LVIDs = 32: (50 − 32) / 50 × 100 = 36 (FS),
//! 50 − 50 × 36 / 100 = 32 (LVIDs), 100 × 32 / (100 − 36) = 50 (LVIDd). The three asserted values (36, 32,
//! 50) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-shortening library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_fractional_shortening_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-shortening.adj")
        .canonicalize()
        .expect("shipped fractional-shortening.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fs_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fractional_shortening_lib()).unwrap();
    std::fs::write(dir.join("fractional-shortening.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fractional_shortening — the percentage: (LVIDd − LVIDs) / LVIDd × 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_fractional_shortening_library_and_computes_it_with_citation() {
    let dir = scratch("fs");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-shortening.adj\"\n\
         observe lvidd(50)\n\
         observe lvids(32)\n\
         ? fractional_shortening(lvidd, lvids)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (50 − 32) / 50 × 100 = 36, computed
    // EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fractional_shortening\"") && s.contains("\"value\":36"),
        "fractional_shortening(50, 32) = 36: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// lvids — the same equation solved for the end-systolic diameter: LVIDd − LVIDd × FS / 100.
// ---------------------------------------------------------------------------

#[test]
fn computes_lvids_from_fractional_shortening_with_citation() {
    let dir = scratch("lvids");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-shortening.adj\"\n\
         observe lvidd(50)\n\
         observe fractional_shortening(36)\n\
         ? lvids(lvidd, fractional_shortening)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 50 − 50 × 36 / 100 = 50 − 18 = 32, computed on the CPU.
    assert!(
        s.contains("\"name\":\"lvids\"") && s.contains("\"value\":32"),
        "lvids(50, 36) = 32: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "lvids carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// lvidd — the same equation solved for the end-diastolic diameter: 100 × LVIDs / (100 − FS), the third
// reading of the one ratio.
// ---------------------------------------------------------------------------

#[test]
fn computes_lvidd_from_fractional_shortening_with_citation() {
    let dir = scratch("lvidd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-shortening.adj\"\n\
         observe lvids(32)\n\
         observe fractional_shortening(36)\n\
         ? lvidd(lvids, fractional_shortening)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 × 32 / (100 − 36) = 3200 / 64 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"lvidd\"") && s.contains("\"value\":50"),
        "lvidd(32, 36) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "lvidd carries its cited provenance: {s}"
    );
}
