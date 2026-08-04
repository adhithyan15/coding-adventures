//! End-to-end tests for the `clinical/mentzer-index.adj` library — the Mentzer-index relation
//! (Mentzer index = mean corpuscular volume ÷ red cell count) and its two exact rearrangements —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured red-cell indices with `observe`, and the engine applies the cited definition on the CPU,
//! computing the EXACT value and rendering the definition's citation and trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT around the worked case MCV = 80 fL, red
//! cell count = 4 (million/µL): 80 ÷ 4 = 20 (index), 20 × 4 = 80 (MCV), 80 ÷ 20 = 4 (count). The
//! three asserted values (20, 80, 4) — none is a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped Mentzer-index library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_mentzer_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/mentzer-index.adj")
        .canonicalize()
        .expect("shipped mentzer-index.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mentzer_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_mentzer_lib()).unwrap();
    std::fs::write(dir.join("mentzer-index.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// mentzer_index — the relation: mean corpuscular volume ÷ red cell count.
// ---------------------------------------------------------------------------

#[test]
fn imports_mentzer_library_and_computes_it_with_citation() {
    let dir = scratch("idx");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mentzer-index.adj\"\n\
         observe mean_corpuscular_volume(80)\n\
         observe red_cell_count(4)\n\
         ? mentzer_index(mean_corpuscular_volume, red_cell_count)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 80 ÷ 4 = 20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"mentzer_index\"") && s.contains("\"value\":20"),
        "mentzer_index(80, 4) = 20: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mean_corpuscular_volume — the same definition solved for the MCV: index × red cell count.
// ---------------------------------------------------------------------------

#[test]
fn computes_mcv_from_index_and_count_with_citation() {
    let dir = scratch("mcv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mentzer-index.adj\"\n\
         observe mentzer_index(20)\n\
         observe red_cell_count(4)\n\
         ? mean_corpuscular_volume(mentzer_index, red_cell_count)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 × 4 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mean_corpuscular_volume\"") && s.contains("\"value\":80"),
        "mean_corpuscular_volume(20, 4) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "mean_corpuscular_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// red_cell_count — the same definition solved for the count: MCV ÷ index, the third reading of the
// one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_count_from_mcv_and_index_with_citation() {
    let dir = scratch("rbc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mentzer-index.adj\"\n\
         observe mean_corpuscular_volume(80)\n\
         observe mentzer_index(20)\n\
         ? red_cell_count(mean_corpuscular_volume, mentzer_index)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 80 ÷ 20 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"red_cell_count\"") && s.contains("\"value\":4"),
        "red_cell_count(80, 20) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "red_cell_count carries its cited provenance: {s}"
    );
}
