//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/mitosis-phase-ordinal-position.adj`) driven
//! through the built CLI: a `rule` composing the NEW `mitosis_phase_order`
//! table (`biology/mitosis-phase-order.adj`) with the already-shipped
//! `ordinal_number` table (`mathematics/ordinal-numbers.adj`, a
//! CROSS-DIRECTORY import via `../mathematics/ordinal-numbers.adj`, the
//! same shape `astronomy/planet-ordinal-position.adj` and
//! `astronomy/moon-phase-ordinal-position.adj` already established) to
//! DERIVE `mitosis_phase_ordinal_position($Phase, $Ordinal)` -- the FOURTH
//! cross-directory `rule` composition in this loop's science curriculum
//! sweep, and the FIRST in the biology domain. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mitosisordinal_{tag}_{}", std::process::id()));
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

/// Copy BOTH shipped libraries, preserving their real relative directory
/// structure: `mitosis-phase-ordinal-position.adj` (in `biology/`) imports
/// `mitosis-phase-order.adj` (same dir) and
/// `../mathematics/ordinal-numbers.adj` (cross-directory), so the entry
/// program must sit at a root that contains both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("biology/mitosis-phase-order.adj", "biology/mitosis-phase-order.adj"),
        (
            "biology/mitosis-phase-ordinal-position.adj",
            "biology/mitosis-phase-ordinal-position.adj",
        ),
        (
            "mathematics/ordinal-numbers.adj",
            "mathematics/ordinal-numbers.adj",
        ),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn anaphase_derives_third_with_dual_citations() {
    let dir = scratch("anaphase");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position(anaphase, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"O\":\"third\""),
        "anaphase is the third phase of mitosis: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: NCI
    // SEER (mitosis_phase_order) AND the ordinal-word convention
    // (ordinal_number).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derivation is a rule composing two fact steps: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("ef.edu"),
        "carries citations from BOTH composed libraries (mitosis-phase-order.adj and ordinal-numbers.adj): {out}"
    );
}

#[test]
fn first_reverse_binds_to_prophase() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position($P, first)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"prophase\""),
        "prophase is the first phase of mitosis: {out}"
    );
}

#[test]
fn interphase_abstains_honestly_as_not_one_of_the_four_ordered_phases() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position(interphase, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "interphase is the resting phase BETWEEN divisions, not one of the four ordered mitotic phases -- honest abstention, never invented: {out}"
    );
}
