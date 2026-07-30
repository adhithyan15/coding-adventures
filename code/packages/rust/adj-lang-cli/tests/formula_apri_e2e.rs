//! End-to-end tests for the `clinical/apri.adj` library — the AST-to-platelet ratio index
//! (APRI = AST / ULN / platelet count × 100) and its two exact rearrangements — driven through the built
//! CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the AST, its upper limit of normal, and
//! the platelet count with `observe`, and the engine applies the cited formula on the CPU, computing the
//! EXACT value (over exact rationals) and rendering the citation and trust tier in the `derived` section
//! (the auditable answer). The three formulas INVERT around the worked case AST = 80, ULN = 40, platelets
//! = 100: 80 / 40 / 100 × 100 = 2 (APRI), 2 × 40 × 100 / 100 = 80 (AST), 80 / 40 × 100 / 2 = 100 (platelets).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree contains the 100 constant and the intermediates 200 and 8000, so a
//! bare `"value":2` / `"value":80` / `"value":100` could spuriously match `200` / `8000` / the `100`
//! literal. The adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped apri library, resolved from this crate's manifest dir so the test is
/// location-independent.
fn shipped_apri_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/apri.adj")
        .canonicalize()
        .expect("shipped apri.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_apri_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_apri_lib()).unwrap();
    std::fs::write(dir.join("apri.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// apri — the index: AST / ULN / platelet count × 100.
// ---------------------------------------------------------------------------

#[test]
fn imports_apri_library_and_computes_it_with_citation() {
    let dir = scratch("index");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apri.adj\"\n\
         observe ast(80)\n\
         observe ast_uln(40)\n\
         observe platelet_count(100)\n\
         ? apri(ast, ast_uln, platelet_count)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 80 / 40 / 100 × 100 = 2, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 100 literal and the 200/8000
    // intermediates in the derivation cannot spuriously satisfy a bare "value":2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"apri\",\"value\":2"),
        "apri(80, 40, 100) = 2: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// ast — the same equation solved for the AST: APRI × ULN × platelet count / 100.
// ---------------------------------------------------------------------------

#[test]
fn computes_ast_from_apri_with_citation() {
    let dir = scratch("ast");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apri.adj\"\n\
         observe apri(2)\n\
         observe ast_uln(40)\n\
         observe platelet_count(100)\n\
         ? ast(apri, ast_uln, platelet_count)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 40 × 100 / 100 = 8000 / 100 = 80, computed on the CPU.
    assert!(
        s.contains("\"name\":\"ast\",\"value\":80"),
        "ast(2, 40, 100) = 80: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "ast carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// platelet_count — the same equation solved for the platelet count: AST / ULN × 100 / APRI, the third
// reading of the one index.
// ---------------------------------------------------------------------------

#[test]
fn computes_platelet_count_from_apri_with_citation() {
    let dir = scratch("plt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"apri.adj\"\n\
         observe apri(2)\n\
         observe ast(80)\n\
         observe ast_uln(40)\n\
         ? platelet_count(apri, ast, ast_uln)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 80 / 40 × 100 / 2 = 2 × 100 / 2 = 200 / 2 = 100, computed on the CPU.
    assert!(
        s.contains("\"name\":\"platelet_count\",\"value\":100"),
        "platelet_count(2, 80, 40) = 100: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "platelet_count carries its cited provenance: {s}"
    );
}
