//! End-to-end tests for the `clinical/homa-ir.adj` library — the homeostatic model assessment of insulin
//! resistance (HOMA-IR = fasting insulin × fasting glucose / 22.5) and its two exact rearrangements —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the two
//! fasting values with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT
//! value (over exact rationals) and rendering the citation and trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT around the worked case insulin = 9, glucose = 5:
//! 9 × 5 / 22.5 = 2 (HOMA-IR), 2 × 22.5 / 5 = 9 (insulin), 2 × 22.5 / 9 = 5 (glucose).
//!
//! Note the assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a
//! bare `"value":N`: the 22.5 constant renders as `"value":22.5` in the derivation tree, so a bare
//! `"value":2` would spuriously match the leading `2` of `22.5`. The adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped homa-ir library, resolved from this crate's manifest dir so the test is
/// location-independent.
fn shipped_homa_ir_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/homa-ir.adj")
        .canonicalize()
        .expect("shipped homa-ir.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_homair_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_homa_ir_lib()).unwrap();
    std::fs::write(dir.join("homa-ir.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// homa_ir — the index: fasting insulin × fasting glucose / 22.5.
// ---------------------------------------------------------------------------

#[test]
fn imports_homa_ir_library_and_computes_it_with_citation() {
    let dir = scratch("index");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homa-ir.adj\"\n\
         observe fasting_insulin(9)\n\
         observe fasting_glucose(5)\n\
         ? homa_ir(fasting_insulin, fasting_glucose)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 9 × 5 / 22.5 = 45 / 22.5 = 2,
    // computed EXACTLY over rationals, not as a rounded float. Match the adjacent name/value pair so the
    // 22.5 literal in the derivation cannot spuriously satisfy a bare "value":2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"homa_ir\",\"value\":2"),
        "homa_ir(9, 5) = 2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// fasting_insulin — the same equation solved for the insulin: HOMA-IR × 22.5 / glucose.
// ---------------------------------------------------------------------------

#[test]
fn computes_fasting_insulin_from_homa_ir_with_citation() {
    let dir = scratch("insulin");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homa-ir.adj\"\n\
         observe homa_ir(2)\n\
         observe fasting_glucose(5)\n\
         ? fasting_insulin(homa_ir, fasting_glucose)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 22.5 / 5 = 45 / 5 = 9, computed on the CPU.
    assert!(
        s.contains("\"name\":\"fasting_insulin\",\"value\":9"),
        "fasting_insulin(2, 5) = 9: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "fasting_insulin carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// fasting_glucose — the same equation solved for the glucose: HOMA-IR × 22.5 / insulin, the third reading
// of the one index.
// ---------------------------------------------------------------------------

#[test]
fn computes_fasting_glucose_from_homa_ir_with_citation() {
    let dir = scratch("glucose");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homa-ir.adj\"\n\
         observe homa_ir(2)\n\
         observe fasting_insulin(9)\n\
         ? fasting_glucose(homa_ir, fasting_insulin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 22.5 / 9 = 45 / 9 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"fasting_glucose\",\"value\":5"),
        "fasting_glucose(2, 9) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "fasting_glucose carries its cited provenance: {s}"
    );
}
