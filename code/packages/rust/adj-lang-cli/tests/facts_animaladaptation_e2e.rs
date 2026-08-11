//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-adaptation.adj`) driven through the
//! built CLI: a native `table` naming three animals and the one survival
//! adaptation each is known for, quoted verbatim from three different
//! nationalgeographic.com animal-facts pages. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_animaladaptation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/animal-adaptation.adj");
    std::fs::copy(&src, dir.join("animal-adaptation.adj"))
        .expect("copy shipped animal-adaptation.adj");
}

#[test]
fn animal_adaptation_recall_binds_the_adaptation_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-adaptation.adj\"\n\
         ? animal_adaptation(arctic_fox, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"A\":\"camouflage\""),
        "the arctic fox is known for camouflage: {out}"
    );
    assert!(
        out.contains("nationalgeographic.com") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic citation: {out}"
    );
}

#[test]
fn animal_adaptation_reverse_binds_the_animal_for_that_adaptation() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-adaptation.adj\"\n\
         ? animal_adaptation($X, hibernation)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"X\":\"groundhog\""),
        "the shipped hibernation example is groundhog: {out}"
    );
}

#[test]
fn animal_adaptation_abstains_honestly_on_an_untabled_animal() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-adaptation.adj\"\n\
         ? animal_adaptation(penguin, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "penguin is a real animal but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
