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
    // THE WHOLE CITATION, anchored on its JSON key and closed by the
    // terminating quote. This sentence carries a qualifier, so a
    // truncation would silently drop meaning -- the defect issue #13916
    // shipped. Pinning a fragment narrows that hole rather than closing
    // it, because `contains` on a fragment cannot see what precedes or
    // follows it. See issue #13918.
    assert!(
        // The apostrophe here is U+2019, as genome.gov renders it. This pin
        // previously carried an ASCII one, matching a citation absent from its
        // own page. It was correct in FORM -- whole sentence, anchored on the
        // JSON key -- and it faithfully defended a wrong value, which is why
        // repairing the value broke it.
        out.contains("\"source\":\"Mitosis is generally followed by equal division of the cell’s content into two daughter cells that have identical genomes.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
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

const CELL_DIVISION_GENETIC_OUTCOME_PIN: &str = r#""bindings":{"Outcome":"genetically_identical"},"citations":[{"source":"Mitosis is generally followed by equal division of the cell’s content into two daughter cells that have identical genomes.","locator":"https://www.genome.gov/genetics-glossary/Mitosis","trust":"authoritative""#;

#[test]
fn cell_division_genetic_outcome_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-genetic-outcome.adj\"
? cell_division_genetic_outcome(mitosis, $Outcome)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped citation carried an ASCII apostrophe where the page renders
    // U+2019, so it did not appear on its own page -- the whole premise being
    // that a caller can check a citation against its locator.
    assert!(
        out.contains(CELL_DIVISION_GENETIC_OUTCOME_PIN),
        "the cell division genetic outcome citation matches its page: {out}"
    );
}

const CELL_DIVISION_MEIOSIS_PIN: &str = r#""bindings":{"Outcome":"haploid"},"citations":[{"source":"Mitosis is generally followed by equal division of the cell’s content into two daughter cells that have identical genomes.","locator":"https://www.genome.gov/genetics-glossary/Mitosis","trust":"authoritative","corroborations":[{"source":"During meiosis, each diploid cell undergoes two rounds of division to yield four haploid daughter cells — the gametes.","locator":"https://www.genome.gov/genetics-glossary/Meiosis"}"#;

#[test]
fn cell_division_meiosis_citation_keeps_the_pages_em_dash() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-genetic-outcome.adj\"\n? cell_division_genetic_outcome(meiosis, $Outcome)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The value shipped "--" where the page has an em dash (U+2014). A NEW
    // AXIS: every screen in this effort keyed on QUOTE characters, so none
    // could see a dash flattening. Found by hand-diffing a candidate, then
    // confirmed by a punctuation-general screen -- which itself first
    // reported ZERO here, because it mapped each dash CHARACTER to a marker
    // and so read "--" as two marks against the page's one.
    //
    // THE QUERY IS `meiosis`, not the companion's first query (`mitosis`).
    // The repaired value is the meiosis `cites`; pinning mitosis would tie an
    // answer about genetic identity to evidence about gametes.
    //
    // Note for consumers: citations[0] here is the table's mitosis `source`
    // even for a meiosis answer -- the per-row evidence is in corroborations.
    // The pin spans bindings THROUGH that corroboration for exactly that
    // reason. Tracked on #14124.
    assert!(
        out.contains(CELL_DIVISION_MEIOSIS_PIN),
        "the meiosis corroboration matches its page: {out}"
    );
}
