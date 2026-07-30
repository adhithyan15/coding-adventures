//! End-to-end tests for the `clinical/arm-span-height.adj` library — estimating standing height from arm
//! span (height = span / 1.06) and its one exact rearrangement — driven through the built CLI binary against
//! the SHIPPED stdlib. The same invariant as every other formula library: a consumer states NO arithmetic; it
//! imports the grounded library, binds the measured arm span with `observe`, and the engine applies the cited
//! formula on the CPU, computing the EXACT value (over exact rationals) and rendering the citation and trust
//! tier in the `derived` section (the auditable answer). The two formulas INVERT around the worked case
//! span = 159: 159 / 1.06 = 150 (height), 150 × 1.06 = 159 (span).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the factor 1.06, so a bare numeric substring could spuriously match.
//! The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped arm-span-height library, resolved from this crate's manifest dir so the test
/// is location-independent.
fn shipped_ash_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/arm-span-height.adj")
        .canonicalize()
        .expect("shipped arm-span-height.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ash_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ash_lib()).unwrap();
    std::fs::write(dir.join("arm-span-height.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// estimated_height — the standing-height estimate: span / 1.06.
// ---------------------------------------------------------------------------

#[test]
fn imports_arm_span_height_library_and_computes_it_with_citation() {
    let dir = scratch("ash");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"arm-span-height.adj\"\n\
         observe arm_span(159)\n\
         ? estimated_height(arm_span)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 159 / 1.06 = 150, computed EXACTLY over
    // rationals (1.06 = 53/50). Match the adjacent name/value pair so the 1.06 factor cannot spuriously satisfy
    // a bare "value":150.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"estimated_height\",\"value\":150"),
        "estimated_height(159) = 150: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// arm_span — the same expression solved for the arm span: height × 1.06.
// ---------------------------------------------------------------------------

#[test]
fn computes_arm_span_from_height_with_citation() {
    let dir = scratch("span");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"arm-span-height.adj\"\n\
         observe estimated_height(150)\n\
         ? arm_span(estimated_height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 × 1.06 = 159, computed on the CPU.
    assert!(
        s.contains("\"name\":\"arm_span\",\"value\":159"),
        "arm_span(150) = 159: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "arm_span carries its cited provenance: {s}"
    );
}
