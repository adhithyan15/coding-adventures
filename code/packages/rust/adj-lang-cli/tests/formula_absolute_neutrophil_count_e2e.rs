//! End-to-end tests for the `clinical/absolute-neutrophil-count.adj` library — the absolute neutrophil
//! count (ANC = WBC × (percent PMNs + percent bands) / 100) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the count and the two
//! percentages with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT
//! value (over exact rationals) and rendering the citation and trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT around the worked case WBC = 5000, PMNs = 40, bands = 10:
//! 5000 × (40 + 10) / 100 = 2500 (ANC), 2500 × 100 / (40 + 10) = 5000 (WBC), 2500 × 100 / 5000 − 10 = 40
//! (PMNs). The three asserted values (2500, 5000, 40) are distinct, none a colon-anchored prefix of
//! another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped absolute-neutrophil-count library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_anc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/absolute-neutrophil-count.adj")
        .canonicalize()
        .expect("shipped absolute-neutrophil-count.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_anc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_anc_lib()).unwrap();
    std::fs::write(dir.join("absolute-neutrophil-count.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// anc — the count: WBC × (percent PMNs + percent bands) / 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_anc_library_and_computes_it_with_citation() {
    let dir = scratch("anc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"absolute-neutrophil-count.adj\"\n\
         observe wbc(5000)\n\
         observe pmns(40)\n\
         observe bands(10)\n\
         ? anc(wbc, pmns, bands)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 5000 × (40 + 10) / 100 =
    // 5000 × 50 / 100 = 2500, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"anc\"") && s.contains("\"value\":2500"),
        "anc(5000, 40, 10) = 2500: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// wbc — the same equation solved for the white count: ANC × 100 / (percent PMNs + percent bands).
// ---------------------------------------------------------------------------

#[test]
fn computes_wbc_from_anc_with_citation() {
    let dir = scratch("wbc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"absolute-neutrophil-count.adj\"\n\
         observe anc(2500)\n\
         observe pmns(40)\n\
         observe bands(10)\n\
         ? wbc(anc, pmns, bands)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2500 × 100 / (40 + 10) = 250000 / 50 = 5000, computed on the CPU.
    assert!(
        s.contains("\"name\":\"wbc\"") && s.contains("\"value\":5000"),
        "wbc(2500, 40, 10) = 5000: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "wbc carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// pmns — the same equation solved for the segmented-neutrophil percentage:
// ANC × 100 / WBC − percent bands, the third reading of the one count.
// ---------------------------------------------------------------------------

#[test]
fn computes_pmns_from_anc_with_citation() {
    let dir = scratch("pmns");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"absolute-neutrophil-count.adj\"\n\
         observe anc(2500)\n\
         observe wbc(5000)\n\
         observe bands(10)\n\
         ? pmns(anc, wbc, bands)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2500 × 100 / 5000 − 10 = 250000 / 5000 − 10 = 50 − 10 = 40, computed on the CPU.
    assert!(
        s.contains("\"name\":\"pmns\"") && s.contains("\"value\":40"),
        "pmns(2500, 5000, 10) = 40: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "pmns carries its cited provenance: {s}"
    );
}
