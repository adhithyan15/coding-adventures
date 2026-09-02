//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/mixture-types.adj`) driven through the built
//! CLI: a native `table` of mixture kind → the everyday example the source names
//! resolves binding-query recalls (forward and backward) with the source's
//! LibreTexts citation, and abstains on a kind not in the table (alloy) — 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmix_{tag}_{}", std::process::id()));
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

#[test]
fn chemistry_mixture_example_recall_binds_example_with_citation() {
    let dir = scratch("mixturetypes");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/mixture-types.adj");
    std::fs::copy(&src, dir.join("mixture-types.adj")).expect("copy shipped mixture-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n\
         ? mixture_example(colloid, $Example)\n\
         ? mixture_example(suspension, $Example)\n\
         ? mixture_example($Kind, vegetable_soup)\n\
         ? mixture_example(alloy, $Example)\n",
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
    // THIS PIN USED TO ASSERT A FIVE-CLAUSE JOINED VALUE, AND IT WAS RIGHT TO.
    // Its reasoning -- anchor on the JSON key, close on the terminating quote,
    // never pin a fragment -- is the discipline used throughout this effort,
    // and it caught a bad repair that replaced the join with a single block
    // grounding NONE of the five rows.
    //
    // But what it was faithfully defending was a CONSTRUCTED SPAN: five real
    // sentences joined with " … ", a string no page displays. An anchored pin
    // defends whatever it is pointed at, including a defect. The join is now
    // split into one `source` plus four `cites`, so each row has its own
    // verbatim span, and the pin is repointed at the envelope.
    assert!(
        out.contains("\"source\":\"A homogeneous mixture is a mixture in which the composition is uniform throughout the mixture. The salt water described above is homogeneous because the dissolved salt is evenly distributed throughout the entire salt water sample.\""),
        "the envelope is one verbatim block, not a join: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A colloid's everyday example is milk; a suspension's is salad dressing —
    // the recalled example values (forward binds).
    assert!(out.contains("\"Example\":\"milk\""), "colloid -> milk: {out}");
    assert!(
        out.contains("\"Example\":\"salad_dressing\""),
        "suspension -> salad_dressing: {out}"
    );
    // The relation runs BACKWARD: bind the example vegetable_soup, recall the
    // kind the source classifies it as — heterogeneous.
    assert!(
        out.contains("\"Kind\":\"heterogeneous\""),
        "vegetable_soup -> heterogeneous (reverse recall): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at consensus trust
    // (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "alloy" is not in the table — honest abstention, never a fabricated example.
    assert!(out.contains("\"abstained\":true"), "alloy abstains: {out}");
}

const MT_SUSP_PIN: &str = r#""bindings":{"Example":"salad_dressing"},"citations":[{"source":"A homogeneous mixture is a mixture in which the composition is uniform throughout the mixture. The salt water described above is homogeneous because the dissolved salt is evenly distributed throughout the entire salt water sample.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures","trust":"consensus","corroborations":[{"source":"When the salt is thoroughly mixed into the water in this glass, it will form a solution.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures"},{"source":"A heterogeneous mixture is a mixture in which the composition is not uniform throughout the mixture. Vegetable soup is a heterogeneous mixture.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures"},{"source":"The salad dressing in this bottle is a suspension.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures""#;

const MT_ALL_PIN: &str = r#""bindings":{"Kind":"heterogeneous"},"citations":[{"source":"A homogeneous mixture is a mixture in which the composition is uniform throughout the mixture. The salt water described above is homogeneous because the dissolved salt is evenly distributed throughout the entire salt water sample.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures","trust":"consensus","corroborations":[{"source":"When the salt is thoroughly mixed into the water in this glass, it will form a solution.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures"},{"source":"A heterogeneous mixture is a mixture in which the composition is not uniform throughout the mixture. Vegetable soup is a heterogeneous mixture.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures"},{"source":"The salad dressing in this bottle is a suspension.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures"},{"source":"Homogenized milk is a colloid.","locator":"https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures""#;

#[test]
fn mixture_types_suspension_answer_carries_its_own_corroboration() {
    let dir = scratch("cite_susp");
    std::fs::copy(
        facts_stdlib().join("chemistry/mixture-types.adj"),
        dir.join("mixture-types.adj"),
    )
    .expect("copy shipped mixture-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n? mixture_example(suspension, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // suspension was the row whose evidence was hardest to find, and the one
    // most likely to be grounded wrongly: the page's FIRST block mentioning
    // salad dressing calls it a "liquid mixture" and never says suspension.
    // The sentence that does is four blocks later. Reading every occurrence
    // rather than the first match is what separated them.
    //
    // The pin runs through suspension's OWN corroboration (index 2), not
    // corroborations[0] -- a prefix pin would bind the `solution` sentence
    // while claiming to check this row.
    assert!(
        out.contains(MT_SUSP_PIN),
        "suspension's answer carries the sentence that names it: {out}"
    );
}

#[test]
fn mixture_types_reverse_answer_carries_all_four_cites_in_order() {
    let dir = scratch("cite_mtall");
    std::fs::copy(
        facts_stdlib().join("chemistry/mixture-types.adj"),
        dir.join("mixture-types.adj"),
    )
    .expect("copy shipped mixture-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n? mixture_example($Kind, vegetable_soup)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Spans the whole four-cite list. The five spans replaced ONE joined
    // value, so a reorder or a dropped middle entry must fail here even
    // though every sentence is still present somewhere in the blob.
    assert!(
        out.contains(MT_ALL_PIN),
        "the answer carries all four corroborations in order: {out}"
    );
}
