//! End-to-end tests for the `clinical/reticulocyte-production-index.adj` library — the reticulocyte
//! production index (RPI = reticulocyte% × hematocrit / 45) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the reticulocyte
//! percentage and hematocrit with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case
//! reticulocyte% = 6, hematocrit = 15: 6 × 15 / 45 = 2 (RPI), 2 × 45 / 15 = 6 (reticulocyte%),
//! 2 × 45 / 6 = 15 (hematocrit). The three asserted values (2, 6, 15) are distinct, none a
//! colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped reticulocyte-production-index library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_rpi_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/reticulocyte-production-index.adj")
        .canonicalize()
        .expect("shipped reticulocyte-production-index.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rpi_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_rpi_lib()).unwrap();
    std::fs::write(dir.join("reticulocyte-production-index.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// reticulocyte_production_index — the index: reticulocyte% × hematocrit / 45.
// ---------------------------------------------------------------------------

#[test]
fn imports_rpi_library_and_computes_it_with_citation() {
    let dir = scratch("rpi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reticulocyte-production-index.adj\"\n\
         observe reticulocyte_percent(6)\n\
         observe hematocrit(15)\n\
         ? reticulocyte_production_index(reticulocyte_percent, hematocrit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 6 × 15 / 45 = 2, computed
    // EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"reticulocyte_production_index\"") && s.contains("\"value\":2"),
        "reticulocyte_production_index(6, 15) = 2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// reticulocyte_percent — the same equation solved for the reticulocyte percentage: RPI × 45 / hematocrit.
// ---------------------------------------------------------------------------

#[test]
fn computes_reticulocyte_percent_from_rpi_with_citation() {
    let dir = scratch("retic");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reticulocyte-production-index.adj\"\n\
         observe reticulocyte_production_index(2)\n\
         observe hematocrit(15)\n\
         ? reticulocyte_percent(reticulocyte_production_index, hematocrit)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 45 / 15 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"reticulocyte_percent\"") && s.contains("\"value\":6"),
        "reticulocyte_percent(2, 15) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "reticulocyte_percent carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// hematocrit — the same equation solved for the hematocrit: RPI × 45 / reticulocyte%, the third reading
// of the one index.
// ---------------------------------------------------------------------------

#[test]
fn computes_hematocrit_from_rpi_and_reticulocyte_percent_with_citation() {
    let dir = scratch("hct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reticulocyte-production-index.adj\"\n\
         observe reticulocyte_production_index(2)\n\
         observe reticulocyte_percent(6)\n\
         ? hematocrit(reticulocyte_production_index, reticulocyte_percent)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 45 / 6 = 15, computed on the CPU.
    assert!(
        s.contains("\"name\":\"hematocrit\"") && s.contains("\"value\":15"),
        "hematocrit(2, 6) = 15: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "hematocrit carries its cited provenance: {s}"
    );
}
