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
    assert!(
        out.contains("nidcd.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIDCD citation: {out}"
    );
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
    for structure in ["malleus", "incus", "stapes"] {
        assert!(
            out.contains(&format!(
                "\"term\":\"ear_structure_function({structure}, amplifies_sound)\""
            )),
            "backward recall should include {structure}: {out}"
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
