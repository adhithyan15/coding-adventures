//! End-to-end tests for the `clinical/free-water-deficit.adj` library — the hypernatraemia free water
//! deficit (deficit = total body water × (plasma sodium / 140 − 1)) and its two exact rearrangements —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the total
//! body water and plasma sodium with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals — 154/140 = 11/10) and rendering the citation and
//! trust tier in the `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case total body water = 42, plasma sodium = 154: 42 × (154/140 − 1) = 42 × 0.1 = 4.2
//! (deficit), 4.2 / 0.1 = 42 (total body water), 140 × (4.2/42 + 1) = 140 × 1.1 = 154 (plasma sodium).
//! The three asserted values (4.2, 42, 154) are distinct, none a colon-anchored prefix of another
//! rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped free-water-deficit library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_fwd_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/free-water-deficit.adj")
        .canonicalize()
        .expect("shipped free-water-deficit.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fwd_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fwd_lib()).unwrap();
    std::fs::write(dir.join("free-water-deficit.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// free_water_deficit — the deficit: total body water × (plasma sodium / 140 − 1).
// ---------------------------------------------------------------------------

#[test]
fn imports_fwd_library_and_computes_it_with_citation() {
    let dir = scratch("fwd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"free-water-deficit.adj\"\n\
         observe total_body_water(42)\n\
         observe plasma_sodium(154)\n\
         ? free_water_deficit(total_body_water, plasma_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 42 × (154/140 − 1) =
    // 42 × 0.1 = 4.2, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"free_water_deficit\"") && s.contains("\"value\":4.2"),
        "free_water_deficit(42, 154) = 4.2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// total_body_water — the same equation solved for the total body water: deficit / (plasma sodium / 140
// − 1).
// ---------------------------------------------------------------------------

#[test]
fn computes_total_body_water_from_deficit_with_citation() {
    let dir = scratch("tbw");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"free-water-deficit.adj\"\n\
         observe free_water_deficit(4.2)\n\
         observe plasma_sodium(154)\n\
         ? total_body_water(free_water_deficit, plasma_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4.2 / (154/140 − 1) = 4.2 / 0.1 = 42, computed on the CPU.
    assert!(
        s.contains("\"name\":\"total_body_water\"") && s.contains("\"value\":42"),
        "total_body_water(4.2, 154) = 42: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "total_body_water carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plasma_sodium — the same equation solved for the plasma sodium: 140 × (deficit / total body water +
// 1), the third reading of the one deficit.
// ---------------------------------------------------------------------------

#[test]
fn computes_plasma_sodium_from_deficit_with_citation() {
    let dir = scratch("na");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"free-water-deficit.adj\"\n\
         observe free_water_deficit(4.2)\n\
         observe total_body_water(42)\n\
         ? plasma_sodium(free_water_deficit, total_body_water)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 140 × (4.2 / 42 + 1) = 140 × 1.1 = 154, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plasma_sodium\"") && s.contains("\"value\":154"),
        "plasma_sodium(4.2, 42) = 154: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plasma_sodium carries its cited provenance: {s}"
    );
}
