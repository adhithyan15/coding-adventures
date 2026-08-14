//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/cell-division-genetic-outcome.adj`) driven
//! through the built CLI: a native `table` recording the genetic outcome
//! of mitosis vs. meiosis -- a sibling to the already-shipped
//! `cell-division-daughter-cells.adj` (which only carries the daughter-cell
//! COUNT for each process), decoding the "have identical genomes" /
//! "haploid" clause already sitting unused inside that table's own header
//! quotes. Resolves forward and backward recall queries with the source's
//! citation -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_celldivisiongeneticoutcome_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("biology/cell-division-genetic-outcome.adj");
    std::fs::copy(&src, dir.join("cell-division-genetic-outcome.adj"))
        .expect("copy shipped cell-division-genetic-outcome.adj");
}

#[test]
fn cell_division_genetic_outcome_recalls_mitosis_as_genetically_identical_with_citation() {
    let dir = scratch("mitosis");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-genetic-outcome.adj\"\n\
         ? cell_division_genetic_outcome(mitosis, $Outcome)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"cell_division_genetic_outcome(mitosis, genetically_identical)\""),
        "mitosis should recall as genetically identical: {out}"
    );
    assert!(
        out.contains("genome.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NHGRI citation: {out}"
    );
}

#[test]
fn cell_division_genetic_outcome_backward_recalls_meiosis_for_haploid() {
    let dir = scratch("haploid");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-genetic-outcome.adj\"\n\
         ? cell_division_genetic_outcome($Process, haploid)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"cell_division_genetic_outcome(meiosis, haploid)\""),
        "meiosis should be the only recalled haploid outcome: {out}"
    );
    assert!(
        !out.contains("cell_division_genetic_outcome(mitosis, haploid)"),
        "mitosis yields genetically identical cells, not haploid: {out}"
    );
}

#[test]
fn cell_division_genetic_outcome_abstains_on_binary_fission() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-genetic-outcome.adj\"\n\
         ? cell_division_genetic_outcome(binary_fission, $Outcome)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "binary_fission is the prokaryotic process, not one of these two eukaryotic ones -- honest abstention expected: {out}"
    );
}
