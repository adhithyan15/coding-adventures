//! End-to-end tests for the `clinical/respiratory-quotient.adj` library — the respiratory quotient
//! (RQ = VCO2 / VO2) and its two exact rearrangements — driven through the built CLI binary against the
//! SHIPPED stdlib. The same invariant as every other formula library: a consumer states NO arithmetic; it
//! imports the grounded library, binds CO2 production and O2 consumption with `observe`, and the engine
//! applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and rendering the
//! citation and trust tier in the `derived` section (the auditable answer). The three formulas INVERT around
//! the worked case VCO2 = 300, VO2 = 400: 300 / 400 = 0.75 (RQ), 0.75 × 400 = 300 (VCO2),
//! 300 / 0.75 = 400 (VO2). The worked RQ is the DYADIC value 0.75 (= 3/4, exactly representable in f64) so
//! every rendered value is exact.
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: a bare numeric substring could spuriously match another node. The name-anchored adjacent form
//! is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped respiratory-quotient library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_rq_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/respiratory-quotient.adj")
        .canonicalize()
        .expect("shipped respiratory-quotient.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rq_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_rq_lib()).unwrap();
    std::fs::write(dir.join("respiratory-quotient.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// respiratory_quotient — the ratio: VCO2 / VO2.
// ---------------------------------------------------------------------------

#[test]
fn imports_respiratory_quotient_library_and_computes_it_with_citation() {
    let dir = scratch("rq");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-quotient.adj\"\n\
         observe co2_production(300)\n\
         observe o2_consumption(400)\n\
         ? respiratory_quotient(co2_production, o2_consumption)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 300 / 400 = 0.75, computed EXACTLY over
    // rationals (3/4). Match the adjacent name/value pair.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"respiratory_quotient\",\"value\":0.75"),
        "respiratory_quotient(300, 400) = 0.75: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// co2_production — the same expression solved for VCO2: RQ × VO2.
// ---------------------------------------------------------------------------

#[test]
fn computes_co2_production_from_rq_with_citation() {
    let dir = scratch("vco2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-quotient.adj\"\n\
         observe respiratory_quotient(0.75)\n\
         observe o2_consumption(400)\n\
         ? co2_production(respiratory_quotient, o2_consumption)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.75 × 400 = 300, computed on the CPU (0.75 = 3/4 is exact in f64).
    assert!(
        s.contains("\"name\":\"co2_production\",\"value\":300"),
        "co2_production(0.75, 400) = 300: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "co2_production carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// o2_consumption — the same expression solved for VO2: VCO2 / RQ, the third reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_o2_consumption_from_rq_with_citation() {
    let dir = scratch("vo2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-quotient.adj\"\n\
         observe co2_production(300)\n\
         observe respiratory_quotient(0.75)\n\
         ? o2_consumption(co2_production, respiratory_quotient)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 300 / 0.75 = 400, computed on the CPU (0.75 = 3/4 is exact in f64).
    assert!(
        s.contains("\"name\":\"o2_consumption\",\"value\":400"),
        "o2_consumption(300, 0.75) = 400: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "o2_consumption carries its cited provenance: {s}"
    );
}
