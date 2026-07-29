//! End-to-end tests for the `clinical/serum-ascites-albumin-gradient.adj` library — the SAAG
//! relation (SAAG = serum albumin − ascitic albumin) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured albumin concentrations with `observe`, and the engine applies the cited definition on
//! the CPU, computing the EXACT value and rendering the definition's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case serum
//! albumin = 4 g/dL, ascitic albumin = 1 g/dL: 4 − 1 = 3 (SAAG), 3 + 1 = 4 (serum), 4 − 3 = 1
//! (ascitic). The three asserted values (3, 4, 1) are distinct single digits, none a colon-anchored
//! prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped SAAG library, resolved from this crate's manifest dir so the test is
/// location-independent.
fn shipped_saag_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/serum-ascites-albumin-gradient.adj")
        .canonicalize()
        .expect("shipped serum-ascites-albumin-gradient.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_saag_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_saag_lib()).unwrap();
    std::fs::write(dir.join("serum-ascites-albumin-gradient.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// saag — the relation: serum albumin − ascitic albumin.
// ---------------------------------------------------------------------------

#[test]
fn imports_saag_library_and_computes_it_with_citation() {
    let dir = scratch("gap");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-ascites-albumin-gradient.adj\"\n\
         observe serum_albumin(4)\n\
         observe ascites_albumin(1)\n\
         ? saag(serum_albumin, ascites_albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 4 − 1 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"saag\"") && s.contains("\"value\":3"),
        "saag(4, 1) = 3: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_albumin — the same definition solved for the serum value: SAAG + ascitic.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_albumin_from_gradient_and_ascitic_with_citation() {
    let dir = scratch("serum");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-ascites-albumin-gradient.adj\"\n\
         observe saag(3)\n\
         observe ascites_albumin(1)\n\
         ? serum_albumin(saag, ascites_albumin)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 + 1 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_albumin\"") && s.contains("\"value\":4"),
        "serum_albumin(3, 1) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_albumin carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// ascites_albumin — the same definition solved for the ascitic value: serum − SAAG, the third
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_ascites_albumin_from_serum_and_gradient_with_citation() {
    let dir = scratch("ascites");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"serum-ascites-albumin-gradient.adj\"\n\
         observe serum_albumin(4)\n\
         observe saag(3)\n\
         ? ascites_albumin(serum_albumin, saag)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 − 3 = 1, computed on the CPU.
    assert!(
        s.contains("\"name\":\"ascites_albumin\"") && s.contains("\"value\":1"),
        "ascites_albumin(4, 3) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "ascites_albumin carries its cited provenance: {s}"
    );
}
