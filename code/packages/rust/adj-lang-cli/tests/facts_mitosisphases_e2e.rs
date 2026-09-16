//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/mitosis-phases.adj`) driven through the built
//! CLI: a native `table` of mitosis phase → defining event resolves a
//! binding-query recall with the source's citation, runs the relation backward
//! (event → phase), and abstains on `interphase` (a stage BETWEEN divisions,
//! deliberately not a mitotic phase in this table) — 0 model calls.
//!
//! Each row CITES its own line from the NCI SEER "Cell Cycle" page. `cites`,
//! not `source`: each line is a list item nested under its phase item and
//! never names the phase (#13934's held question). The envelope used to be
//! prophase's own line, and so was the primary source of the other three
//! answers; it is now the page's statement of what cell division is.

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

const LOCATOR: &str = "https://training.seer.cancer.gov/disease/cancer/biology/cycle.html";
const ENVELOPE: &str = "Cell division is the process by which cells reproduce (mitosis).";
const OLD_ENVELOPE: &str = "Chromatin is transformed into chromosomes composed of pairs of filaments called chromatids (each is a complete genetic copy of its chromosome).";

/// (phase, event, the page's own line) -- generated from the converter's
/// page-derived spans, not retyped.
const ROWS: [(&str, &str, &str); 4] = [
    ("prophase", "chromatin_forms_chromosomes", "Chromatin is transformed into chromosomes composed of pairs of filaments called chromatids (each is a complete genetic copy of its chromosome)."),
    ("metaphase", "chromosomes_line_up", "Paired chromosomes become lined up between the centrioles."),
    ("anaphase", "chromatids_separate", "Chromatids are pulled toward the centrioles. One chromatid from each pair goes to each daughter cell."),
    ("telophase", "nuclear_membrane_forms", "A nuclear membrane forms around each set of chromosomes forming a new nucleus with a nucleolus."),
];

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/mitosis-phases.adj"))
        .expect("read shipped mitosis-phases.adj");
    adj[adj.find("table mitosis_phase").expect("table")..].to_string()
}

/// The whole primary citation, as the serializer emits it.
fn primary() -> String {
    format!("\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\"")
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/mitosis-phases.adj"),
        dir.join("mitosis-phases.adj"),
    )
    .expect("copy shipped mitosis-phases.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"mitosis-phases.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
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
    assert!(
        out.contains("\"E\":\"chromosomes_line_up\""),
        "metaphase → chromosomes_line_up: {out}"
    );
    assert!(
        out.contains("\"E\":\"chromatids_separate\""),
        "anaphase → chromatids_separate: {out}"
    );
    assert!(
        out.contains("\"P\":\"metaphase\""),
        "chromosomes_line_up → metaphase (reverse recall): {out}"
    );
    // ONE CONTIGUOUS RUN. This used to pin the OLD envelope with an empty
    // corroboration list and then re-check the trust tier as a loose second
    // needle satisfiable by any other part of the output (#15209).
    assert!(out.contains(&primary()), "carries the NCI SEER citation, whole: {out}");
    // (b) "interphase" is the resting stage BETWEEN divisions, not a phase OF
    // mitosis — honest abstention, never a fabricated event.
    assert!(
        out.contains("\"abstained\":true"),
        "non-mitotic-phase key abstains: {out}"
    );
}

#[test]
fn every_phase_is_corroborated_by_its_own_line() {
    // WHAT THIS CHANGE ADDED. Before it, every answer's only span was the
    // prophase line.
    for (phase, event, span) in ROWS {
        let out = ask(&format!("corr_{phase}"), &format!("mitosis_phase({phase}, $E)"));
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {phase}: {out}"
        );
        assert!(out.contains(&format!("\"E\":\"{event}\"")), "{phase} binds {event}: {out}");
        assert!(
            out.contains(&format!(
                "{},\"corroborations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\"}}]",
                primary()
            )),
            "{phase}: the envelope is primary and its own line corroborates: {out}"
        );
        for (other, _, other_span) in ROWS {
            if other != phase {
                assert!(
                    !out.contains(other_span),
                    "the {other} line must not reach the {phase} answer: {out}"
                );
            }
        }
    }
}

#[test]
fn no_row_carries_a_source_because_no_line_names_its_phase() {
    // THE ZERO IS THE INSTRUMENT: a row `source` would assert a warrant none
    // of these lines carries.
    let body = shipped_table();
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("row (") && l.trim_end().ends_with('{'))
            .count(),
        4,
        "every row opens its own block"
    );
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("cites \"")).count(),
        4,
        "each row carries exactly one corroboration"
    );
    let source_lines: Vec<&str> = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect();
    assert_eq!(
        source_lines,
        vec![format!("    source \"{ENVELOPE}\"").as_str()],
        "exactly one `source` line in the table -- the envelope's"
    );
    let out = ask("nosource", "mitosis_phase(telophase, $E)");
    assert!(out.contains(&primary()), "the envelope is the PRIMARY source: {out}");
}

#[test]
fn the_envelope_is_cell_division_and_names_no_phase() {
    let body = shipped_table();
    assert!(
        !body.contains(&format!("source \"{OLD_ENVELOPE}\"")),
        "the envelope must not be the prophase line again"
    );
    assert!(body.contains(&format!("    source \"{ENVELOPE}\"")), "the envelope is shipped");
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            keys += 1;
            assert!(!folded.contains(key), "the envelope must name no phase, but names {key:?}");
        }
    }
    assert_eq!(keys, 4, "all four keys were actually checked");
    for (phase, event, span) in ROWS {
        assert!(
            body.contains(&format!("    row ({phase}, {event}) {{\n")),
            "row ({phase}, {event}) is shipped"
        );
        assert!(
            body.contains(&format!("cites \"{span}\" locator \"{LOCATOR}\"")),
            "{phase}'s shipped cites line is the page line"
        );
    }
}
