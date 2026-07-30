//! End-to-end tests for the `clinical/corrected-phenytoin.adj` library — the albumin-corrected
//! (Sheiner–Tozer) phenytoin concentration (corrected = obtained / (0.275 × albumin + 0.1)) and its
//! two exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the obtained level and albumin with `observe`, and the engine applies the cited
//! formula on the CPU, computing the EXACT value (over exact rationals — 0.275 = 11/40, 0.1 = 1/10) and
//! rendering the citation and trust tier in the `derived` section (the auditable answer). The three
//! formulas INVERT around the worked case obtained = 6.5, albumin = 2: 6.5 / (0.275 × 2 + 0.1) =
//! 6.5 / 0.65 = 10 (corrected), 10 × 0.65 = 6.5 (obtained), (6.5/10 − 0.1)/0.275 = 2 (albumin). The
//! three asserted values (10, 6.5, 2) are distinct, none a colon-anchored prefix of another rendered
//! value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped corrected-phenytoin library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_phenytoin_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/corrected-phenytoin.adj")
        .canonicalize()
        .expect("shipped corrected-phenytoin.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phenytoin_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_phenytoin_lib()).unwrap();
    std::fs::write(dir.join("corrected-phenytoin.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// corrected_phenytoin — the correction: obtained / (0.275 × albumin + 0.1).
// ---------------------------------------------------------------------------

#[test]
fn imports_phenytoin_library_and_computes_it_with_citation() {
    let dir = scratch("corr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-phenytoin.adj\"\n\
         observe obtained_phenytoin(6.5)\n\
         observe albumin(2)\n\
         ? corrected_phenytoin(obtained_phenytoin, albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 6.5 / (0.275 × 2 + 0.1) =
    // 6.5 / 0.65 = 10, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"corrected_phenytoin\"") && s.contains("\"value\":10"),
        "corrected_phenytoin(6.5, 2) = 10: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// obtained_phenytoin — the same equation solved for the obtained level: corrected × (0.275 × albumin +
// 0.1).
// ---------------------------------------------------------------------------

#[test]
fn computes_obtained_phenytoin_from_corrected_with_citation() {
    let dir = scratch("obt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-phenytoin.adj\"\n\
         observe corrected_phenytoin(10)\n\
         observe albumin(2)\n\
         ? obtained_phenytoin(corrected_phenytoin, albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 × (0.275 × 2 + 0.1) = 10 × 0.65 = 6.5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"obtained_phenytoin\"") && s.contains("\"value\":6.5"),
        "obtained_phenytoin(10, 2) = 6.5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "obtained_phenytoin carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// albumin — the same equation solved for the albumin: (obtained / corrected − 0.1) / 0.275, the third
// reading of the one correction.
// ---------------------------------------------------------------------------

#[test]
fn computes_albumin_from_phenytoin_pair_with_citation() {
    let dir = scratch("alb");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"corrected-phenytoin.adj\"\n\
         observe obtained_phenytoin(6.5)\n\
         observe corrected_phenytoin(10)\n\
         ? albumin(corrected_phenytoin, obtained_phenytoin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (6.5 / 10 − 0.1) / 0.275 = (0.65 − 0.1) / 0.275 = 0.55 / 0.275 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"albumin\"") && s.contains("\"value\":2"),
        "albumin(10, 6.5) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "albumin carries its cited provenance: {s}"
    );
}
