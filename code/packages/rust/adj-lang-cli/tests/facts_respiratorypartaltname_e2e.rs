//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/respiratory-part-alt-name.adj`) driven through
//! the built CLI: a native `table` naming the everyday alternate name for
//! two named respiratory parts, decoded from clauses already sitting unused
//! inside `respiratory-parts.adj`'s own already-quoted NCI SEER source
//! sentences -- a sibling to that table. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a real,
//! already-tabled part (larynx) whose own quote states only its function,
//! never an everyday alternate name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_respiratorypartaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/respiratory-part-alt-name.adj");
    std::fs::copy(&src, dir.join("respiratory-part-alt-name.adj"))
        .expect("copy shipped respiratory-part-alt-name.adj");
}

#[test]
fn respiratory_part_alt_name_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name(trachea, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"respiratory_part_alt_name(trachea, windpipe)\""),
        "the trachea is commonly called the windpipe: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn respiratory_part_alt_name_recalls_backward_to_alveoli() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name($Part, air_sacs)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"respiratory_part_alt_name(alveoli, air_sacs)\""),
        "air_sacs recalls the alveoli: {out}"
    );
}

#[test]
fn respiratory_part_alt_name_abstains_honestly_on_larynx() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-part-alt-name.adj\"\n\
         ? respiratory_part_alt_name(larynx, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "larynx is a real, already-tabled respiratory part but its own quote states only its function, never an everyday alternate name -- honest abstention: {out}"
    );
}
