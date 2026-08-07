//! End-to-end tests for the `clinical/serum-osmolality.adj` library — the calculated (Dorwart and
//! Chalmers) serum osmolality (1.86 × sodium + glucose/18 + BUN/2.8 + 9) and its three exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant
//! as every other formula library: a consumer states NO arithmetic; it imports the grounded library,
//! binds sodium, glucose and BUN with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals — 1.86 = 93/50, 2.8 = 14/5) and rendering the
//! citation and trust tier in the `derived` section (the auditable answer). The four formulas INVERT
//! around the worked case sodium = 145, glucose = 90, BUN = 28: 1.86 × 145 + 90/18 + 28/2.8 + 9 =
//! 269.7 + 5 + 10 + 9 = 293.7 (osmolality), and each inverse back-solves its own input —
//! (293.7 − 5 − 10 − 9)/1.86 = 145 (sodium), (293.7 − 269.7 − 10 − 9)×18 = 90 (glucose),
//! (293.7 − 269.7 − 5 − 9)×2.8 = 28 (BUN). The four asserted values (293.7, 145, 90, 28) are distinct,
//! none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped serum-osmolality library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_osm_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/serum-osmolality.adj")
        .canonicalize()
        .expect("shipped serum-osmolality.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_osm_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_osm_lib()).unwrap();
    std::fs::write(dir.join("serum-osmolality.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// serum_osmolality — the estimate: 1.86 × sodium + glucose/18 + BUN/2.8 + 9.
// ---------------------------------------------------------------------------

#[test]
fn imports_osmolality_library_and_computes_it_with_citation() {
    let dir = scratch("osm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-osmolality.adj\"\n\
         observe sodium(145)\n\
         observe glucose(90)\n\
         observe bun(28)\n\
         ? serum_osmolality(sodium, glucose, bun)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result:
    // 1.86 × 145 + 90/18 + 28/2.8 + 9 = 293.7, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"serum_osmolality\"") && s.contains("\"value\":293.7"),
        "serum_osmolality(145, 90, 28) = 293.7: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// sodium — the same equation solved for the sodium: (osmolality − glucose/18 − BUN/2.8 − 9) / 1.86.
// ---------------------------------------------------------------------------

#[test]
fn computes_sodium_from_osmolality_with_citation() {
    let dir = scratch("na");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-osmolality.adj\"\n\
         observe serum_osmolality(293.7)\n\
         observe glucose(90)\n\
         observe bun(28)\n\
         ? sodium(serum_osmolality, glucose, bun)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (293.7 − 5 − 10 − 9) / 1.86 = 145, computed on the CPU.
    assert!(
        s.contains("\"name\":\"sodium\"") && s.contains("\"value\":145"),
        "sodium(293.7, 90, 28) = 145: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// glucose — the same equation solved for the glucose: (osmolality − 1.86×sodium − BUN/2.8 − 9) × 18.
// ---------------------------------------------------------------------------

#[test]
fn computes_glucose_from_osmolality_with_citation() {
    let dir = scratch("glu");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-osmolality.adj\"\n\
         observe serum_osmolality(293.7)\n\
         observe sodium(145)\n\
         observe bun(28)\n\
         ? glucose(serum_osmolality, sodium, bun)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (293.7 − 269.7 − 10 − 9) × 18 = 90, computed on the CPU.
    assert!(
        s.contains("\"name\":\"glucose\"") && s.contains("\"value\":90"),
        "glucose(293.7, 145, 28) = 90: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "glucose carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// bun — the same equation solved for the BUN: (osmolality − 1.86×sodium − glucose/18 − 9) × 2.8, the
// fourth reading of the one formula.
// ---------------------------------------------------------------------------

#[test]
fn computes_bun_from_osmolality_with_citation() {
    let dir = scratch("bun");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-osmolality.adj\"\n\
         observe serum_osmolality(293.7)\n\
         observe sodium(145)\n\
         observe glucose(90)\n\
         ? bun(serum_osmolality, sodium, glucose)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (293.7 − 269.7 − 5 − 9) × 2.8 = 28, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bun\"") && s.contains("\"value\":28"),
        "bun(293.7, 145, 90) = 28: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bun carries its cited provenance: {s}"
    );
}
