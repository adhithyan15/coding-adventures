//! End-to-end tests for the `clinical/oxygen-delivery.adj` library — global oxygen delivery
//! (DO2 = cardiac output × arterial oxygen content × 10) and its two exact rearrangements — driven through
//! the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the cardiac output and the
//! arterial oxygen content with `observe`, and the engine applies the cited formula on the CPU, computing
//! the EXACT value (over exact rationals) and rendering the citation and trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT around the worked case CO = 5, CaO2 = 20:
//! 5 × 20 × 10 = 1000 (DO2), 1000 / (20 × 10) = 5 (CO), 1000 / (5 × 10) = 20 (CaO2).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree contains the 10 constant and the intermediate 200, so a bare
//! `"value":20` could spuriously match the leading digits of `200`. The adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped oxygen-delivery library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_do2_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/oxygen-delivery.adj")
        .canonicalize()
        .expect("shipped oxygen-delivery.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_do2_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_do2_lib()).unwrap();
    std::fs::write(dir.join("oxygen-delivery.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// do2 — the delivery: cardiac output × arterial oxygen content × 10.
// ---------------------------------------------------------------------------

#[test]
fn imports_oxygen_delivery_library_and_computes_it_with_citation() {
    let dir = scratch("do2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygen-delivery.adj\"\n\
         observe cardiac_output(5)\n\
         observe arterial_oxygen_content(20)\n\
         ? do2(cardiac_output, arterial_oxygen_content)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 5 × 20 × 10 = 1000, computed EXACTLY
    // over rationals. Match the adjacent name/value pair so the 10 constant and the 200 intermediate in the
    // derivation cannot spuriously satisfy a bare "value":1000.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"do2\",\"value\":1000"),
        "do2(5, 20) = 1000: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// cardiac_output — the same equation solved for the cardiac output: DO2 / (CaO2 × 10).
// ---------------------------------------------------------------------------

#[test]
fn computes_cardiac_output_from_do2_with_citation() {
    let dir = scratch("co");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygen-delivery.adj\"\n\
         observe do2(1000)\n\
         observe arterial_oxygen_content(20)\n\
         ? cardiac_output(do2, arterial_oxygen_content)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1000 / (20 × 10) = 1000 / 200 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"cardiac_output\",\"value\":5"),
        "cardiac_output(1000, 20) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "cardiac_output carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// arterial_oxygen_content — the same equation solved for the content: DO2 / (CO × 10), the third reading
// of the one delivery.
// ---------------------------------------------------------------------------

#[test]
fn computes_arterial_oxygen_content_from_do2_with_citation() {
    let dir = scratch("cao2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"oxygen-delivery.adj\"\n\
         observe do2(1000)\n\
         observe cardiac_output(5)\n\
         ? arterial_oxygen_content(do2, cardiac_output)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1000 / (5 × 10) = 1000 / 50 = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"arterial_oxygen_content\",\"value\":20"),
        "arterial_oxygen_content(1000, 5) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "arterial_oxygen_content carries its cited provenance: {s}"
    );
}
