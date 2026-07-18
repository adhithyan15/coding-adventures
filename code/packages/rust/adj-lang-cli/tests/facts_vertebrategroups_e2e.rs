//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/vertebrate-groups.adj`) driven through the built
//! CLI: a native `table` of the five vertebrate classes → the ONE distinctive
//! body-covering / characteristic the source assigns each one resolves
//! binding-query recalls (forward AND backward) with the source's U.S. National
//! Park Service ("Vertebrate Grab Bag") citation, and abstains on a word that is
//! not one of the five vertebrate classes (an `insect` — an invertebrate) —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsv_{tag}_{}", std::process::id()));
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
fn biology_vertebrate_groups_recall_binds_trait_with_citation() {
    let dir = scratch("vertebrategroups");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/vertebrate-groups.adj");
    std::fs::copy(&src, dir.join("vertebrate-groups.adj"))
        .expect("copy shipped vertebrate-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"vertebrate-groups.adj\"\n\
         ? vertebrate_trait(bird, $Trait)\n\
         ? vertebrate_trait(mammal, $Trait)\n\
         ? vertebrate_trait(reptile, $Trait)\n\
         ? vertebrate_trait($Class, feathers)\n\
         ? vertebrate_trait(insect, $Trait)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Birds have feathers, mammals have hair, reptiles have dry scaly skin — the
    // recalled distinctive features (forward binds).
    assert!(
        out.contains("\"Trait\":\"feathers\""),
        "bird → feathers: {out}"
    );
    assert!(out.contains("\"Trait\":\"hair\""), "mammal → hair: {out}");
    assert!(
        out.contains("\"Trait\":\"dry_scaly_skin\""),
        "reptile → dry_scaly_skin: {out}"
    );
    // The relation runs BACKWARD: bind the trait `feathers`, recall its class.
    assert!(
        out.contains("\"Class\":\"bird\""),
        "feathers → bird (reverse recall): {out}"
    );
    // The answer carries the NPS "Vertebrate Grab Bag" citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // An insect is an invertebrate, not one of the five vertebrate classes —
    // honest abstention, never a fabricated trait.
    assert!(out.contains("\"abstained\":true"), "insect abstains: {out}");
}
