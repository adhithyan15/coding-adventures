//! End-to-end test for the metrology SI-DERIVED-UNITS facts library
//! (`adj-facts-stdlib/metrology/si-derived-units.adj`) driven through the built
//! CLI: a native `table` of generic-derived-quantity → unit-symbol resolves a
//! binding-query recall with the NIST citation, and abstains on a quantity the
//! source sentence does not name — 0 model calls, never a fabricated unit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsder_{tag}_{}", std::process::id()));
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

fn with_case(dir: &Path, body: &str) -> PathBuf {
    let src = facts_stdlib().join("metrology/si-derived-units.adj");
    std::fs::copy(&src, dir.join("si-derived-units.adj"))
        .expect("copy shipped si-derived-units.adj");
    let p = dir.join("case.adj");
    std::fs::write(&p, format!("import \"si-derived-units.adj\"\n{body}")).unwrap();
    p
}

#[test]
fn si_derived_unit_recall_binds_symbol_with_citation() {
    let dir = scratch("recall");
    let p = with_case(
        &dir,
        "? si_derived_unit(velocity, $Symbol)\n\
         ? si_derived_unit(acceleration, $Symbol)\n",
    );

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Velocity -> m/s; acceleration -> m/s2 (verbatim symbols from the NIST page).
    assert!(out.contains("m/s"), "velocity binds the m/s symbol: {out}");
    assert!(
        out.contains("m/s2"),
        "acceleration binds the m/s2 symbol: {out}"
    );
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation: {out}"
    );
}

#[test]
fn an_unnamed_quantity_abstains_rather_than_inventing_a_unit() {
    let dir = scratch("abstain");
    // Luminance is a real photometric quantity, but the cited sentence does not
    // name it among the generic derived units — the table must abstain.
    let p = with_case(&dir, "? si_derived_unit(luminance, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a quantity the source doesn't name abstains: {out}"
    );
}
