//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/lung-lobe-names.adj`) driven through the built
//! CLI: a native `table` of individual named lung lobe -> owning lung
//! resolves a binding-query recall with the source's NIH/NCBI Bookshelf
//! citation, runs the relation backward with a genuine one-to-many reverse
//! recall (lung -> every lobe it has), and abstains on a non-lobe (the
//! trachea) -- 0 model calls.
//!
//! Each row carries its OWN lung's sentence as its `source`. Unlike the
//! heading-shaped tables converted alongside it, each sentence names both the
//! lung and its lobes, so it warrants its rows. The envelope used to be the
//! right-lung sentence, which made it the primary source of the two LEFT-lung
//! answers although it names no left lobe.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsanat_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK470197/";
const ENVELOPE: &str = "The right and left lungs' structural organization is similar, though asymmetrical.";
const RIGHT: &str = "The right lung is comprised of the right upper (RUL), middle (RML), and lower (RLL) lobes.";
const LEFT: &str = "The left lung consists of the left upper (LUL) and lower (LLL) lobes.";

/// (lobe, lung, that row's own `source`) -- generated from the converter's
/// page-verified spans, not retyped.
const ROWS: [(&str, &str, &str); 5] = [
    ("right_upper_lobe", "right_lung", "The right lung is comprised of the right upper (RUL), middle (RML), and lower (RLL) lobes."),
    ("right_middle_lobe", "right_lung", "The right lung is comprised of the right upper (RUL), middle (RML), and lower (RLL) lobes."),
    ("right_lower_lobe", "right_lung", "The right lung is comprised of the right upper (RUL), middle (RML), and lower (RLL) lobes."),
    ("left_upper_lobe", "left_lung", "The left lung consists of the left upper (LUL) and lower (LLL) lobes."),
    ("left_lower_lobe", "left_lung", "The left lung consists of the left upper (LUL) and lower (LLL) lobes."),
];

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/lung-lobe-names.adj"))
        .expect("read shipped lung-lobe-names.adj");
    adj[adj.find("table lung_lobe_name").expect("table")..].to_string()
}

/// The whole citation a row's answer must carry: its own sentence as the
/// PRIMARY source, the envelope's locator and trust, and no corroboration.
fn row_citation(sentence: &str) -> String {
    format!(
        "\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]"
    )
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(facts_stdlib().join("anatomy/lung-lobe-names.adj"), dir.join("lung-lobe-names.adj"))
        .expect("copy shipped lung-lobe-names.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"lung-lobe-names.adj\"\n? {query}\n")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn anatomy_lung_lobe_names_recall_binds_lung_with_citation() {
    let dir = scratch("lunglobenames");
    let src = facts_stdlib().join("anatomy/lung-lobe-names.adj");
    std::fs::copy(&src, dir.join("lung-lobe-names.adj")).expect("copy shipped lung-lobe-names.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-lobe-names.adj\"\n\
         ? lung_lobe_name(right_middle_lobe, $Lung)\n\
         ? lung_lobe_name(left_upper_lobe, $Lung)\n\
         ? lung_lobe_name($Lobe, right_lung)\n\
         ? lung_lobe_name(trachea, $Lung)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"Lung\":\"right_lung\""), "right_middle_lobe -> right_lung: {out}");
    assert!(out.contains("\"Lung\":\"left_lung\""), "left_upper_lobe -> left_lung: {out}");
    for lobe in ["right_upper_lobe", "right_middle_lobe", "right_lower_lobe"] {
        assert!(
            out.contains(&format!("lung_lobe_name({lobe}, right_lung)")),
            "right_lung recalls {lobe}: {out}"
        );
    }
    // WHOLE CONTIGUOUS RUNS, not `contains("ncbi.nlm.nih.gov") && contains(trust)`
    // (#15209's two-loose-needles shape): both lungs' sentences appear, each as
    // a primary source.
    assert!(out.contains(&row_citation(RIGHT)), "the right-lung sentence is a primary source: {out}");
    assert!(out.contains(&row_citation(LEFT)), "the left-lung sentence is a primary source: {out}");
    assert!(out.contains("\"abstained\":true"), "trachea abstains: {out}");
}

#[test]
fn every_lobe_is_warranted_by_its_own_lungs_sentence() {
    for (lobe, lung, sentence) in ROWS {
        // Lobe keys are distinct, so binding the lung yields one answer. (A
        // fully ground query is ranked as a hypothesis with no citations.)
        let out = ask(&format!("row_{lobe}"), &format!("lung_lobe_name({lobe}, $Lung)"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {lobe}: {out}");
        assert!(out.contains(&format!("\"Lung\":\"{lung}\"")), "{lobe} -> {lung}: {out}");
        assert!(out.contains(&row_citation(sentence)), "{lobe}: its own lung's sentence is the primary source: {out}");
        // NEGATIVE ARMS: the other lung's sentence and the framing envelope
        // reach no row's answer.
        let other = if sentence == RIGHT { LEFT } else { RIGHT };
        assert!(!out.contains(other), "the other lung's sentence must not reach {lobe}: {out}");
        assert!(!out.contains(ENVELOPE), "the framing envelope warrants no row, including {lobe}: {out}");
    }
}

#[test]
fn the_left_lobes_are_no_longer_warranted_by_the_right_lung_sentence() {
    // THE DEFECT THIS CHANGE FIXES, asserted on real answers: the envelope used
    // to be the right-lung sentence, which names no left lobe.
    for lobe in ["left_upper_lobe", "left_lower_lobe"] {
        let out = ask(&format!("left_{lobe}"), &format!("lung_lobe_name({lobe}, $Lung)"));
        assert!(out.contains(&row_citation(LEFT)), "{lobe} is warranted by the left-lung sentence: {out}");
        assert!(!out.contains(RIGHT), "{lobe} must not carry the right-lung sentence: {out}");
    }
}

#[test]
fn the_table_shape_is_five_row_sources_one_framing_envelope() {
    let body = shipped_table();
    let row_sources: Vec<&str> = body.lines().filter(|l| l.starts_with("        source \"")).collect();
    assert_eq!(row_sources.len(), 5, "one `source` per row: {row_sources:?}");
    let all_sources = body.lines().filter(|l| l.trim_start().starts_with("source \"")).count();
    assert_eq!(all_sources, 6, "five row sources and one envelope, at any indentation");
    assert!(body.contains(&format!("\n    source \"{ENVELOPE}\"\n")), "the envelope is the framing sentence");
    assert!(
        !body.lines().any(|l| l.starts_with("        locator ") || l.starts_with("        trust ")),
        "no row restates the envelope's locator or trust (ADJ-TABLES §4)"
    );
    assert!(!body.lines().any(|l| l.trim_start().starts_with("cites ")), "no corroboration anywhere");
    // The envelope names no lobe -- checked against keys READ FROM THE TABLE.
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            keys += 1;
            assert!(!folded.contains(&key.replace('_', " ")), "the envelope must name no lobe, but names {key:?}");
        }
    }
    assert_eq!(keys, 5, "all five keys were actually checked");
    for (lobe, lung, sentence) in ROWS {
        assert!(
            body.contains(&format!("    row ({lobe}, {lung}) {{\n        source \"{sentence}\"\n    }}")),
            "row ({lobe}, {lung}) is shipped with its own lung's sentence"
        );
    }
}
