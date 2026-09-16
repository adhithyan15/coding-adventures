//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/ear-structure-function.adj`) driven through
//! the built CLI: a native `table` naming what each middle-ear ossicle
//! does to the sound signal, decoded from a span already sitting unused
//! inside the SAME NIDCD quotes `ear-parts.adj`'s own header already
//! reproduces -- a sibling to that table. Resolves binding-query recall
//! (both directions, including a 3-answer backward recall) with the
//! source's citation, and abstains on a real, already-tabled ear
//! structure (ear_canal) whose own quote states no action-verb function --
//! 0 model calls.
//!
//! NO SINGLE SENTENCE NAMES A BONE AND WHAT IT DOES (RS-5e, #14986). One
//! sentence says the eardrum sends vibrations "to three tiny bones in the
//! middle ear", the next names them, and a third says "the bones in the
//! middle ear amplify". So each ossicle row `cites` all three, in page
//! order, and the envelope -- now the page's framing sentence, not the
//! amplify sentence -- stays the primary source. Every citation is pinned
//! whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_earstructurefunction_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/ear-structure-function.adj");
    std::fs::copy(&src, dir.join("ear-structure-function.adj"))
        .expect("copy shipped ear-structure-function.adj");
}

const LOCATOR: &str = "https://www.nidcd.nih.gov/health/how-do-we-hear";
const ENVELOPE: &str = "Hearing depends on a series of complex steps that change sound waves in the air into electrical signals.";
const OLD_ENVELOPE: &str = "The bones in the middle ear amplify, or increase, the sound vibrations and send them to the cochlea, a snail-shaped structure filled with fluid, in the inner ear.";
/// The three sentences each ossicle row cites, in page order.
const CHAIN: [&str; 3] = [
    "The eardrum vibrates from the incoming sound waves and sends these vibrations to three tiny bones in the middle ear.",
    "These bones are called the malleus, incus, and stapes.",
    OLD_ENVELOPE,
];
const OSSICLES: [&str; 3] = ["malleus", "incus", "stapes"];

/// The whole citation run every ossicle answer carries.
fn citation() -> String {
    let corr: Vec<String> = CHAIN
        .iter()
        .map(|s| format!("{{\"source\":\"{s}\",\"locator\":\"{LOCATOR}\"}}"))
        .collect();
    format!(
        "\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{}]",
        corr.join(",")
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/ear-structure-function.adj"))
        .expect("read shipped ear-structure-function.adj");
    adj[adj.find("table ear_structure_function").expect("table")..].to_string()
}

#[test]
fn ear_structure_function_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ear-structure-function.adj\"\n\
         ? ear_structure_function(malleus, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ear_structure_function(malleus, amplifies_sound)\""),
        "the malleus amplifies sound: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("nidcd.nih.gov") && contains(trust)` (#15209).
    assert!(out.contains(&citation()), "carries the whole NIDCD citation chain: {out}");
}

#[test]
fn ear_structure_function_recalls_backward_all_three() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ear-structure-function.adj\"\n\
         ? ear_structure_function($Structure, amplifies_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for structure in OSSICLES {
        assert!(
            out.contains(&format!(
                "\"term\":\"ear_structure_function({structure}, amplifies_sound)\""
            )),
            "backward recall should include {structure}: {out}"
        );
    }
}

#[test]
fn every_ossicle_answer_carries_all_three_sentences_in_page_order() {
    for structure in OSSICLES {
        let dir = scratch(&format!("row_{structure}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"ear-structure-function.adj\"\n? ear_structure_function({structure}, $F)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {structure}: {out}");
        assert!(out.contains("\"F\":\"amplifies_sound\""), "{structure} amplifies sound: {out}");
        assert!(
            out.contains(&citation()),
            "{structure}: the framing envelope, then the three chained sentences in page order: {out}"
        );
    }
}

#[test]
fn ear_structure_function_abstains_honestly_on_ear_canal() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ear-structure-function.adj\"\n\
         ? ear_structure_function(ear_canal, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "ear_canal's own quote states no action-verb function -- honest abstention: {out}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    for structure in OSSICLES {
        let mut expected = format!("    row ({structure}, amplifies_sound) {{\n");
        for s in CHAIN {
            expected.push_str(&format!("        cites \"{s}\" locator \"{LOCATOR}\"\n"));
        }
        expected.push_str("    }");
        assert!(body.contains(&expected), "row ({structure}, amplifies_sound) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        cites \"").count(), 9, "three corroborations per ossicle");
    assert!(!body.contains("\n        source "), "no row source: no sentence names a bone and its function");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{OLD_ENVELOPE}\"\n    locator")), "not the amplify sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["malleus", "incus", "stapes", "bone", "amplif", "middle ear"] {
        assert!(!folded.contains(word), "the envelope must name no ossicle or function, but contains {word:?}");
    }
}
