//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/mitosis-phases.adj`) driven through the built
//! CLI: a native `table` of mitosis phase → defining event resolves a
//! binding-query recall with the source's citation, runs the relation backward
//! (event → phase), and abstains on `interphase` (a stage BETWEEN divisions,
//! deliberately not a mitotic phase in this table) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsbio_{tag}_{}", std::process::id()));
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
fn biology_mitosis_phase_recall_binds_event_with_citation_and_abstains_on_interphase() {
    let dir = scratch("mitosisphases");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/mitosis-phases.adj");
    std::fs::copy(&src, dir.join("mitosis-phases.adj"))
        .expect("copy shipped mitosis-phases.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mitosis-phases.adj\"\n\
         ? mitosis_phase(metaphase, $E)\n\
         ? mitosis_phase(anaphase, $E)\n\
         ? mitosis_phase($P, chromosomes_line_up)\n\
         ? mitosis_phase(interphase, $E)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // (a) The metaphase phase binds the source's metaphase event atom: the
    // chromosomes line up between the centrioles.
    assert!(
        out.contains("\"E\":\"chromosomes_line_up\""),
        "metaphase → chromosomes_line_up: {out}"
    );
    // The anaphase event atom (a second forward bind) — chromatids separate.
    assert!(
        out.contains("\"E\":\"chromatids_separate\""),
        "anaphase → chromatids_separate: {out}"
    );
    // The relation runs backward: the event chromosomes_line_up recalls metaphase.
    assert!(
        out.contains("\"P\":\"metaphase\""),
        "chromosomes_line_up → metaphase (reverse recall): {out}"
    );
    // The answer carries the NCI SEER citation and the authoritative trust tier
    // as its proof (locator + trust).
    assert!(
        out.contains("training.seer.cancer.gov/disease/cancer/biology/cycle.html")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // (b) "interphase" is the resting stage BETWEEN divisions, not a phase OF
    // mitosis — honest abstention, never a fabricated event.
    assert!(
        out.contains("\"abstained\":true"),
        "non-mitotic-phase key abstains: {out}"
    );
}
