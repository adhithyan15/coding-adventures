//! End-to-end tests for the `clinical/elimination-half-life.adj` library — a drug's elimination half-life
//! (t1/2 = 0.693 × Vd / CL) and its two exact rearrangements — driven through the built CLI binary against the
//! SHIPPED stdlib. The same invariant as every other formula library: a consumer states NO arithmetic; it
//! imports the grounded library, binds the volume of distribution and the clearance with `observe`, and the
//! engine applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and rendering
//! the citation and trust tier in the `derived` section (the auditable answer). The three formulas INVERT
//! around the worked case Vd = 2000, CL = 693: 0.693 × 2000 / 693 = 2 (half-life),
//! 2 × 693 / 0.693 = 2000 (Vd), 0.693 × 2000 / 2 = 693 (CL).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the factor 0.693 and the intermediate 1386, so a bare numeric substring
//! could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped elimination-half-life library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_ehl_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/elimination-half-life.adj")
        .canonicalize()
        .expect("shipped elimination-half-life.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ehl_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ehl_lib()).unwrap();
    std::fs::write(dir.join("elimination-half-life.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// half_life — the elimination half-life: 0.693 × Vd / CL.
// ---------------------------------------------------------------------------

#[test]
fn imports_elimination_half_life_library_and_computes_it_with_citation() {
    let dir = scratch("ehl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elimination-half-life.adj\"\n\
         observe volume_of_distribution(2000)\n\
         observe clearance(693)\n\
         ? half_life(volume_of_distribution, clearance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 0.693 × 2000 / 693 = 1386 / 693 = 2,
    // computed EXACTLY over rationals (0.693 = 693/1000). Match the adjacent name/value pair so the 0.693
    // factor and the 1386 intermediate cannot spuriously satisfy a bare "value":2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"half_life\",\"value\":2"),
        "half_life(2000, 693) = 2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// volume_of_distribution — the same equation solved for Vd: half_life × CL / 0.693.
// ---------------------------------------------------------------------------

#[test]
fn computes_volume_of_distribution_from_half_life_with_citation() {
    let dir = scratch("vd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elimination-half-life.adj\"\n\
         observe half_life(2)\n\
         observe clearance(693)\n\
         ? volume_of_distribution(half_life, clearance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 693 / 0.693 = 1386 / 0.693 = 2000, computed on the CPU.
    assert!(
        s.contains("\"name\":\"volume_of_distribution\",\"value\":2000"),
        "volume_of_distribution(2, 693) = 2000: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "volume_of_distribution carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// clearance — the same equation solved for CL: 0.693 × Vd / half_life, the third reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_clearance_from_half_life_with_citation() {
    let dir = scratch("cl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"elimination-half-life.adj\"\n\
         observe half_life(2)\n\
         observe volume_of_distribution(2000)\n\
         ? clearance(half_life, volume_of_distribution)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.693 × 2000 / 2 = 1386 / 2 = 693, computed on the CPU.
    assert!(
        s.contains("\"name\":\"clearance\",\"value\":693"),
        "clearance(2, 2000) = 693: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "clearance carries its cited provenance: {s}"
    );
}
