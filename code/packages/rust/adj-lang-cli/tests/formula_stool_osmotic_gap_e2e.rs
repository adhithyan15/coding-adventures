//! End-to-end tests for the `clinical/stool-osmotic-gap.adj` library — the stool (fecal) osmotic gap
//! (gap = 290 − 2 × (stool sodium + stool potassium)) and its two exact rearrangements — driven through the
//! built CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the stool sodium and the stool
//! potassium with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT value
//! (over exact rationals) and rendering the citation and trust tier in the `derived` section (the auditable
//! answer). The three formulas INVERT around the worked case Na = 30, K = 20: 290 − 2 × (30 + 20) = 190
//! (gap), (290 − 190) / 2 − 20 = 30 (Na), (290 − 190) / 2 − 30 = 20 (K).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 290 and 2 and the intermediates 50/100, so a bare
//! numeric substring could spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped stool-osmotic-gap library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_sog_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/stool-osmotic-gap.adj")
        .canonicalize()
        .expect("shipped stool-osmotic-gap.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_sog_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_sog_lib()).unwrap();
    std::fs::write(dir.join("stool-osmotic-gap.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// stool_osmotic_gap — the gap: 290 − 2 × (Na + K).
// ---------------------------------------------------------------------------

#[test]
fn imports_stool_osmotic_gap_library_and_computes_it_with_citation() {
    let dir = scratch("sog");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stool-osmotic-gap.adj\"\n\
         observe stool_sodium(30)\n\
         observe stool_potassium(20)\n\
         ? stool_osmotic_gap(stool_sodium, stool_potassium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 290 − 2 × (30 + 20) = 190, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 290/2 constants and the 50/100
    // intermediates cannot spuriously satisfy a bare "value":190.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"stool_osmotic_gap\",\"value\":190"),
        "stool_osmotic_gap(30, 20) = 190: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// stool_sodium — the same expression solved for the stool sodium: (290 − gap) / 2 − K.
// ---------------------------------------------------------------------------

#[test]
fn computes_stool_sodium_from_gap_with_citation() {
    let dir = scratch("na");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stool-osmotic-gap.adj\"\n\
         observe stool_osmotic_gap(190)\n\
         observe stool_potassium(20)\n\
         ? stool_sodium(stool_osmotic_gap, stool_potassium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (290 − 190) / 2 − 20 = 100 / 2 − 20 = 50 − 20 = 30, computed on the CPU.
    assert!(
        s.contains("\"name\":\"stool_sodium\",\"value\":30"),
        "stool_sodium(190, 20) = 30: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "stool_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// stool_potassium — the same expression solved for the stool potassium: (290 − gap) / 2 − Na, the third
// reading of the one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_stool_potassium_from_gap_with_citation() {
    let dir = scratch("k");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"stool-osmotic-gap.adj\"\n\
         observe stool_osmotic_gap(190)\n\
         observe stool_sodium(30)\n\
         ? stool_potassium(stool_osmotic_gap, stool_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (290 − 190) / 2 − 30 = 100 / 2 − 30 = 50 − 30 = 20, computed on the CPU.
    assert!(
        s.contains("\"name\":\"stool_potassium\",\"value\":20"),
        "stool_potassium(190, 30) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "stool_potassium carries its cited provenance: {s}"
    );
}
