//! End-to-end tests for the `clinical/transferrin-saturation.adj` library — the transferrin saturation
//! (TSAT = [serum iron / transferrin] × 70.9) and its two exact rearrangements — driven through the built
//! CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the serum iron and the transferrin with
//! `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value (over exact
//! rationals) and rendering the citation and trust tier in the `derived` section (the auditable answer). The
//! three formulas INVERT around the worked case serum iron = 100, transferrin = 200: 100 / 200 × 70.9 =
//! 35.45 (TSAT), 35.45 × 200 / 70.9 = 100 (serum iron), 100 × 70.9 / 35.45 = 200 (transferrin).
//!
//! The forward value 35.45 is the engine's EXACT rational (709/20) exported once to f64 — it renders cleanly
//! (no accumulated-error noise) because the engine computes over exact rationals and converts a single time;
//! the two inversions land on integers. The assertions match the ADJACENT `"name":...,"value":...` pair the
//! engine renders, rather than a bare `"value":N`, so a substring cannot spuriously match.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped transferrin-saturation library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_tsat_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/transferrin-saturation.adj")
        .canonicalize()
        .expect("shipped transferrin-saturation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_tsat_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_tsat_lib()).unwrap();
    std::fs::write(dir.join("transferrin-saturation.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// transferrin_saturation — the saturation: [serum iron / transferrin] × 70.9.
// ---------------------------------------------------------------------------

#[test]
fn imports_transferrin_saturation_library_and_computes_it_with_citation() {
    let dir = scratch("tsat");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transferrin-saturation.adj\"\n\
         observe serum_iron(100)\n\
         observe transferrin(200)\n\
         ? transferrin_saturation(serum_iron, transferrin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 100 / 200 × 70.9 = 35.45, computed
    // EXACTLY over rationals (709/20) and exported once to f64. Match the adjacent name/value pair.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"transferrin_saturation\",\"value\":35.45"),
        "transferrin_saturation(100, 200) = 35.45: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_iron — the same equation solved for the serum iron: TSAT × transferrin / 70.9.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_iron_from_saturation_with_citation() {
    let dir = scratch("fe");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transferrin-saturation.adj\"\n\
         observe transferrin_saturation(35.45)\n\
         observe transferrin(200)\n\
         ? serum_iron(transferrin_saturation, transferrin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 35.45 × 200 / 70.9 = 7090 / 70.9 = 100, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_iron\",\"value\":100"),
        "serum_iron(35.45, 200) = 100: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_iron carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// transferrin — the same equation solved for the transferrin: serum iron × 70.9 / TSAT, the third reading
// of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_transferrin_from_saturation_with_citation() {
    let dir = scratch("tf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"transferrin-saturation.adj\"\n\
         observe transferrin_saturation(35.45)\n\
         observe serum_iron(100)\n\
         ? transferrin(transferrin_saturation, serum_iron)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 × 70.9 / 35.45 = 7090 / 35.45 = 200, computed on the CPU.
    assert!(
        s.contains("\"name\":\"transferrin\",\"value\":200"),
        "transferrin(35.45, 100) = 200: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "transferrin carries its cited provenance: {s}"
    );
}
