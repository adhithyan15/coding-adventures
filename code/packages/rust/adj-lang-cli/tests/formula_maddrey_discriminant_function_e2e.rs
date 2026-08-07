//! End-to-end tests for the `clinical/maddrey-discriminant-function.adj` library — the Maddrey
//! discriminant function (DF = 4.6 × (patient PT − control PT) + total bilirubin) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as
//! every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds
//! the two prothrombin times and the bilirubin with `observe`, and the engine applies the cited formula on
//! the CPU, computing the EXACT value (over exact rationals) and rendering the citation and trust tier in
//! the `derived` section (the auditable answer). The three formulas INVERT around the worked case PT = 25,
//! control PT = 15, bilirubin = 8: 4.6 × (25 − 15) + 8 = 54 (DF), (54 − 8) / 4.6 + 15 = 25 (PT),
//! 54 − 4.6 × (25 − 15) = 8 (bilirubin).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree contains the 4.6 constant (rendered `"value":4.6`) and the intermediate
//! 46, so a bare `"value":<short>` could spuriously match a longer number's leading digits. The adjacent
//! form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped maddrey-discriminant-function library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_mdf_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/maddrey-discriminant-function.adj")
        .canonicalize()
        .expect("shipped maddrey-discriminant-function.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mdf_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_mdf_lib()).unwrap();
    std::fs::write(dir.join("maddrey-discriminant-function.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// discriminant_function — the score: 4.6 × (patient PT − control PT) + total bilirubin.
// ---------------------------------------------------------------------------

#[test]
fn imports_mdf_library_and_computes_it_with_citation() {
    let dir = scratch("df");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maddrey-discriminant-function.adj\"\n\
         observe pt(25)\n\
         observe control_pt(15)\n\
         observe tbili(8)\n\
         ? discriminant_function(pt, control_pt, tbili)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 4.6 × (25 − 15) + 8 = 46 + 8 = 54,
    // computed EXACTLY over rationals. Match the adjacent name/value pair so the 4.6 constant and the 46
    // intermediate in the derivation cannot spuriously satisfy a bare "value":54.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"discriminant_function\",\"value\":54"),
        "discriminant_function(25, 15, 8) = 54: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// pt — the same equation solved for the patient prothrombin time:
// (DF − total bilirubin) / 4.6 + control PT.
// ---------------------------------------------------------------------------

#[test]
fn computes_pt_from_mdf_with_citation() {
    let dir = scratch("pt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maddrey-discriminant-function.adj\"\n\
         observe discriminant_function(54)\n\
         observe control_pt(15)\n\
         observe tbili(8)\n\
         ? pt(discriminant_function, control_pt, tbili)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (54 − 8) / 4.6 + 15 = 46 / 4.6 + 15 = 10 + 15 = 25, computed on the CPU.
    assert!(
        s.contains("\"name\":\"pt\",\"value\":25"),
        "pt(54, 15, 8) = 25: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "pt carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// tbili — the same equation solved for the total bilirubin: DF − 4.6 × (patient PT − control PT), the
// third reading of the one score.
// ---------------------------------------------------------------------------

#[test]
fn computes_tbili_from_mdf_with_citation() {
    let dir = scratch("tbili");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maddrey-discriminant-function.adj\"\n\
         observe discriminant_function(54)\n\
         observe pt(25)\n\
         observe control_pt(15)\n\
         ? tbili(discriminant_function, pt, control_pt)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 54 − 4.6 × (25 − 15) = 54 − 46 = 8, computed on the CPU.
    assert!(
        s.contains("\"name\":\"tbili\",\"value\":8"),
        "tbili(54, 25, 15) = 8: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "tbili carries its cited provenance: {s}"
    );
}
