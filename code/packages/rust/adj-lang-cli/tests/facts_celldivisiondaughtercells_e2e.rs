//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/cell-division-daughter-cells.adj`) driven
//! through the built CLI: a native NUMERIC-cell `table` naming the two
//! eukaryotic cell-division processes and how many daughter cells each one
//! produces (mitosis -> 2, meiosis -> 4), quoted verbatim from two NIH
//! National Human Genome Research Institute "Genetics Glossary" pages. A
//! genuinely new library, not an extension of the already-shipped
//! `mitosis-phases.adj` family -- meiosis is a distinct biological process.
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_celldivision_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/cell-division-daughter-cells.adj");
    std::fs::copy(&src, dir.join("cell-division-daughter-cells.adj"))
        .expect("copy shipped cell-division-daughter-cells.adj");
}

#[test]
fn cell_division_daughter_cells_recall_binds_the_count_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-daughter-cells.adj\"\n\
         ? cell_division_daughter_cells(mitosis, $Count)\n",
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
        out.contains("\"Count\":\"2\""),
        "mitosis yields two daughter cells: {out}"
    );
    assert!(
        out.contains("genome.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIH NHGRI citation: {out}"
    );
}

#[test]
fn cell_division_daughter_cells_reverse_binds_the_process() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-daughter-cells.adj\"\n\
         ? cell_division_daughter_cells($Process, 4)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Process\":\"meiosis\""),
        "meiosis is the process that yields four daughter cells: {out}"
    );
}

#[test]
fn cell_division_daughter_cells_abstains_honestly_on_binary_fission() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-daughter-cells.adj\"\n\
         ? cell_division_daughter_cells(binary_fission, $Count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "binary fission is the prokaryotic division process, not one of the two tabled here -- honest abstention, never invented: {out}"
    );
}

const CELL_DIVISION_DAUGHTER_CELLS_PIN: &str = r#""bindings":{"Count":"2"},"citations":[{"source":"Mitosis is generally followed by equal division of the cell’s content into two daughter cells that have identical genomes.","locator":"https://www.genome.gov/genetics-glossary/Mitosis","trust":"authoritative""#;

#[test]
fn cell_division_daughter_cells_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-division-daughter-cells.adj\"
? cell_division_daughter_cells(mitosis, $Count)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped citation carried an ASCII apostrophe where the page renders
    // U+2019, so it did not appear on its own page -- the whole premise being
    // that a caller can check a citation against its locator.
    assert!(
        out.contains(CELL_DIVISION_DAUGHTER_CELLS_PIN),
        "the cell division daughter cells citation matches its page: {out}"
    );
}
