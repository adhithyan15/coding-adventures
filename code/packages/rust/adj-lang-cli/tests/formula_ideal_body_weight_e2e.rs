//! End-to-end tests for the `clinical/ideal-body-weight.adj` library — the Devine ideal body weight
//! (IBW = base + 2.3 × (height in inches − 60), base 50 male / 45.5 female) and the male inverse —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the height
//! with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over
//! exact rationals — 2.3 = 23/10, 45.5 = 91/2) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The formulas COMPOSE and invert around the worked case
//! height = 70 inches: 50 + 2.3 × (70 − 60) = 73 (male IBW), 45.5 + 23 = 68.5 (female IBW),
//! (73 − 50)/2.3 + 60 = 70 (height back from the male IBW). The three asserted values (73, 68.5, 70)
//! are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped ideal-body-weight library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_ibw_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/ideal-body-weight.adj")
        .canonicalize()
        .expect("shipped ideal-body-weight.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ibw_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ibw_lib()).unwrap();
    std::fs::write(dir.join("ideal-body-weight.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// ideal_body_weight_male — 50 + 2.3 × (height − 60).
// ---------------------------------------------------------------------------

#[test]
fn imports_ibw_library_and_computes_male_with_citation() {
    let dir = scratch("male");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ideal-body-weight.adj\"\n\
         observe height(70)\n\
         ? ideal_body_weight_male(height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 50 + 2.3 × (70 − 60) = 73,
    // computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"ideal_body_weight_male\"") && s.contains("\"value\":73"),
        "ideal_body_weight_male(70) = 73: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// ideal_body_weight_female — 45.5 + 2.3 × (height − 60).
// ---------------------------------------------------------------------------

#[test]
fn computes_female_ideal_body_weight_with_citation() {
    let dir = scratch("female");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ideal-body-weight.adj\"\n\
         observe height(70)\n\
         ? ideal_body_weight_female(height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 45.5 + 2.3 × (70 − 60) = 45.5 + 23 = 68.5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"ideal_body_weight_female\"") && s.contains("\"value\":68.5"),
        "ideal_body_weight_female(70) = 68.5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "ideal_body_weight_female carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// height — the male equation solved for the height: (male IBW − 50) / 2.3 + 60.
// ---------------------------------------------------------------------------

#[test]
fn computes_height_from_male_ideal_body_weight_with_citation() {
    let dir = scratch("height");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ideal-body-weight.adj\"\n\
         observe ideal_body_weight_male(73)\n\
         ? height(ideal_body_weight_male)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (73 − 50) / 2.3 + 60 = 10 + 60 = 70, computed on the CPU.
    assert!(
        s.contains("\"name\":\"height\"") && s.contains("\"value\":70"),
        "height(73) = 70: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "height carries its cited provenance: {s}"
    );
}
