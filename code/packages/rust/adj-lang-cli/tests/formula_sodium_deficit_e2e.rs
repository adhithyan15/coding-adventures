//! End-to-end tests for the `clinical/sodium-deficit.adj` library — the hyponatraemia sodium deficit
//! (deficit = total body water × (desired sodium − current sodium)) and its three exact rearrangements
//! — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the total
//! body water and the desired and current sodium with `observe`, and the engine applies the cited
//! formula on the CPU, computing the EXACT value (over exact rationals) and rendering the citation and
//! trust tier in the `derived` section (the auditable answer). The four formulas INVERT around the
//! worked case total body water = 42, desired sodium = 140, current sodium = 120:
//! 42 × (140 − 120) = 840 (deficit), 840 / (140 − 120) = 42 (TBW), 840/42 + 120 = 140 (desired),
//! 140 − 840/42 = 120 (current). The four asserted values (840, 42, 140, 120) are distinct, none a
//! colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped sodium-deficit library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_sodium_deficit_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/sodium-deficit.adj")
        .canonicalize()
        .expect("shipped sodium-deficit.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_nadeficit_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_sodium_deficit_lib()).unwrap();
    std::fs::write(dir.join("sodium-deficit.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// sodium_deficit — the deficit: total body water × (desired − current sodium).
// ---------------------------------------------------------------------------

#[test]
fn imports_sodium_deficit_library_and_computes_it_with_citation() {
    let dir = scratch("deficit");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sodium-deficit.adj\"\n\
         observe total_body_water(42)\n\
         observe desired_sodium(140)\n\
         observe current_sodium(120)\n\
         ? sodium_deficit(total_body_water, desired_sodium, current_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 42 × (140 − 120) = 42 × 20 =
    // 840, computed EXACTLY over rationals, not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"sodium_deficit\"") && s.contains("\"value\":840"),
        "sodium_deficit(42, 140, 120) = 840: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// total_body_water — the same equation solved for the TBW: deficit / (desired − current sodium).
// ---------------------------------------------------------------------------

#[test]
fn computes_total_body_water_from_sodium_deficit_with_citation() {
    let dir = scratch("tbw");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sodium-deficit.adj\"\n\
         observe sodium_deficit(840)\n\
         observe desired_sodium(140)\n\
         observe current_sodium(120)\n\
         ? total_body_water(sodium_deficit, desired_sodium, current_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 840 / (140 − 120) = 840 / 20 = 42, computed on the CPU.
    assert!(
        s.contains("\"name\":\"total_body_water\"") && s.contains("\"value\":42"),
        "total_body_water(840, 140, 120) = 42: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "total_body_water carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// desired_sodium — the same equation solved for the desired sodium: deficit / TBW + current sodium.
// ---------------------------------------------------------------------------

#[test]
fn computes_desired_sodium_from_sodium_deficit_with_citation() {
    let dir = scratch("desired");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sodium-deficit.adj\"\n\
         observe sodium_deficit(840)\n\
         observe total_body_water(42)\n\
         observe current_sodium(120)\n\
         ? desired_sodium(sodium_deficit, total_body_water, current_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 840 / 42 + 120 = 20 + 120 = 140, computed on the CPU.
    assert!(
        s.contains("\"name\":\"desired_sodium\"") && s.contains("\"value\":140"),
        "desired_sodium(840, 42, 120) = 140: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "desired_sodium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// current_sodium — the same equation solved for the current sodium: desired sodium − deficit / TBW, the
// fourth reading of the one deficit.
// ---------------------------------------------------------------------------

#[test]
fn computes_current_sodium_from_sodium_deficit_with_citation() {
    let dir = scratch("current");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sodium-deficit.adj\"\n\
         observe sodium_deficit(840)\n\
         observe total_body_water(42)\n\
         observe desired_sodium(140)\n\
         ? current_sodium(sodium_deficit, total_body_water, desired_sodium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 140 − 840 / 42 = 140 − 20 = 120, computed on the CPU.
    assert!(
        s.contains("\"name\":\"current_sodium\"") && s.contains("\"value\":120"),
        "current_sodium(840, 42, 140) = 120: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "current_sodium carries its cited provenance: {s}"
    );
}
