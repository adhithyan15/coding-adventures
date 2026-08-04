//! End-to-end tests for the `clinical/mid-parental-height.adj` library — the mid-parental (target) height
//! for a boy (MPH = (father's height + mother's height + 13) / 2) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the two parental heights
//! with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over exact
//! rationals) and rendering the citation and trust tier in the `derived` section (the auditable answer). The
//! three formulas INVERT around the worked case father = 180, mother = 165: (180 + 165 + 13) / 2 = 179 (MPH),
//! 179 × 2 − 165 − 13 = 180 (father), 179 × 2 − 180 − 13 = 165 (mother).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 13 and 2 and the intermediate 358, so a bare numeric
//! substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped mid-parental-height library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_mph_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/mid-parental-height.adj")
        .canonicalize()
        .expect("shipped mid-parental-height.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mph_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_mph_lib()).unwrap();
    std::fs::write(dir.join("mid-parental-height.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// mid_parental_height — the boy target height: (father + mother + 13) / 2.
// ---------------------------------------------------------------------------

#[test]
fn imports_mid_parental_height_library_and_computes_it_with_citation() {
    let dir = scratch("mph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mid-parental-height.adj\"\n\
         observe father_height(180)\n\
         observe mother_height(165)\n\
         ? mid_parental_height(father_height, mother_height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: (180 + 165 + 13) / 2 = 179, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 13/2 constants and the 358
    // intermediate cannot spuriously satisfy a bare "value":179.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"mid_parental_height\",\"value\":179"),
        "mid_parental_height(180, 165) = 179: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// father_height — the same expression solved for the father's height: MPH × 2 − mother − 13.
// ---------------------------------------------------------------------------

#[test]
fn computes_father_height_from_mph_with_citation() {
    let dir = scratch("father");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mid-parental-height.adj\"\n\
         observe mid_parental_height(179)\n\
         observe mother_height(165)\n\
         ? father_height(mid_parental_height, mother_height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 179 × 2 − 165 − 13 = 358 − 178 = 180, computed on the CPU.
    assert!(
        s.contains("\"name\":\"father_height\",\"value\":180"),
        "father_height(179, 165) = 180: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "father_height carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mother_height — the same expression solved for the mother's height: MPH × 2 − father − 13, the third
// reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_mother_height_from_mph_with_citation() {
    let dir = scratch("mother");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mid-parental-height.adj\"\n\
         observe mid_parental_height(179)\n\
         observe father_height(180)\n\
         ? mother_height(mid_parental_height, father_height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 179 × 2 − 180 − 13 = 358 − 193 = 165, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mother_height\",\"value\":165"),
        "mother_height(179, 180) = 165: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "mother_height carries its cited provenance: {s}"
    );
}
