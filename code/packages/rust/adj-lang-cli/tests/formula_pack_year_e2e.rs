//! End-to-end tests for the `clinical/pack-year.adj` library — the pack-year relation
//! (pack-years = packs smoked per day × years smoked) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! observed daily rate and duration with `observe`, and the engine applies the cited definition on
//! the CPU, computing the EXACT value and rendering the definition's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case packs
//! per day = 2, years smoked = 15: 2 × 15 = 30 (pack-years), 30 ÷ 15 = 2 (rate), 30 ÷ 2 = 15
//! (years). The three asserted values (30, 2, 15) — none is a colon-anchored prefix of another
//! rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped pack-year library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_pack_year_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/pack-year.adj")
        .canonicalize()
        .expect("shipped pack-year.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_packyear_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_pack_year_lib()).unwrap();
    std::fs::write(dir.join("pack-year.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// pack_years — the relation: packs per day × years smoked.
// ---------------------------------------------------------------------------

#[test]
fn imports_pack_year_library_and_computes_it_with_citation() {
    let dir = scratch("py");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pack-year.adj\"\n\
         observe packs_per_day(2)\n\
         observe years_smoked(15)\n\
         ? pack_years(packs_per_day, years_smoked)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 2 × 15 = 30.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"pack_years\"") && s.contains("\"value\":30"),
        "pack_years(2, 15) = 30: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// packs_per_day — the same definition solved for the daily rate: pack-years ÷ years.
// ---------------------------------------------------------------------------

#[test]
fn computes_rate_from_total_and_years_with_citation() {
    let dir = scratch("rate");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pack-year.adj\"\n\
         observe pack_years(30)\n\
         observe years_smoked(15)\n\
         ? packs_per_day(pack_years, years_smoked)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 ÷ 15 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"packs_per_day\"") && s.contains("\"value\":2"),
        "packs_per_day(30, 15) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "packs_per_day carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// years_smoked — the same definition solved for the duration: pack-years ÷ packs/day, the third
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_years_from_total_and_rate_with_citation() {
    let dir = scratch("years");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pack-year.adj\"\n\
         observe pack_years(30)\n\
         observe packs_per_day(2)\n\
         ? years_smoked(pack_years, packs_per_day)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 ÷ 2 = 15, computed on the CPU.
    assert!(
        s.contains("\"name\":\"years_smoked\"") && s.contains("\"value\":15"),
        "years_smoked(30, 2) = 15: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "years_smoked carries its cited provenance: {s}"
    );
}
