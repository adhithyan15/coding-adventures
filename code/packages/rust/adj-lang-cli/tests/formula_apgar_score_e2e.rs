//! End-to-end tests for the `clinical/apgar-score.adj` library — the Apgar-score relation
//! (Apgar = appearance + pulse + grimace + activity + respiration) and two representative
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the five component scores with `observe`, and the engine applies the cited
//! definition on the CPU, computing the EXACT value and rendering the definition's citation and trust
//! tier in the `derived` section (the auditable answer). The formulas INVERT around the worked case
//! appearance = 2, pulse = 2, grimace = 2, activity = 1, respiration = 1: 2 + 2 + 2 + 1 + 1 = 8
//! (Apgar), 8 − 2 − 2 − 2 − 1 = 1 (activity), 8 − 2 − 2 − 1 − 1 = 2 (appearance). The three asserted
//! values (8, 1, 2) are distinct single digits, none a colon-anchored prefix of another rendered
//! value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped Apgar-score library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_apgar_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/apgar-score.adj")
        .canonicalize()
        .expect("shipped apgar-score.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_apgar_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_apgar_lib()).unwrap();
    std::fs::write(dir.join("apgar-score.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// apgar_score — the relation: appearance + pulse + grimace + activity + respiration.
// ---------------------------------------------------------------------------

#[test]
fn imports_apgar_library_and_computes_it_with_citation() {
    let dir = scratch("apgar");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apgar-score.adj\"\n\
         observe appearance(2)\n\
         observe pulse(2)\n\
         observe grimace(2)\n\
         observe activity(1)\n\
         observe respiration(1)\n\
         ? apgar_score(appearance, pulse, grimace, activity, respiration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 2 + 2 + 2 + 1 + 1 = 8.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"apgar_score\"") && s.contains("\"value\":8"),
        "apgar_score(2, 2, 2, 1, 1) = 8: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// activity — the same definition solved for the activity sign: total minus the other four.
// ---------------------------------------------------------------------------

#[test]
fn computes_activity_from_total_and_others_with_citation() {
    let dir = scratch("activity");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apgar-score.adj\"\n\
         observe apgar_score(8)\n\
         observe appearance(2)\n\
         observe pulse(2)\n\
         observe grimace(2)\n\
         observe respiration(1)\n\
         ? activity(apgar_score, appearance, pulse, grimace, respiration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 8 − 2 − 2 − 2 − 1 = 1, computed on the CPU.
    assert!(
        s.contains("\"name\":\"activity\"") && s.contains("\"value\":1"),
        "activity(8, 2, 2, 2, 1) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "activity carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// appearance — the same definition solved for the appearance sign: total minus the other four, the
// third reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_appearance_from_total_and_others_with_citation() {
    let dir = scratch("appearance");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apgar-score.adj\"\n\
         observe apgar_score(8)\n\
         observe pulse(2)\n\
         observe grimace(2)\n\
         observe activity(1)\n\
         observe respiration(1)\n\
         ? appearance(apgar_score, pulse, grimace, activity, respiration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 8 − 2 − 2 − 1 − 1 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"appearance\"") && s.contains("\"value\":2"),
        "appearance(8, 2, 2, 1, 1) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "appearance carries its cited provenance: {s}"
    );
}
