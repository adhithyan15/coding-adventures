//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-homes.adj`) driven through the built CLI:
//! a native `table` of animal → the name of its home resolves a binding-query
//! recall with the source's citation, and abstains on a non-animal — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn biology_animal_homes_recall_binds_home_with_citation() {
    let dir = scratch("animalhomes");
    // Copy the shipped animal-homes table beside the entry program and import it.
    let src = facts_stdlib().join("biology/animal-homes.adj");
    std::fs::copy(&src, dir.join("animal-homes.adj")).expect("copy shipped animal-homes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-homes.adj\"\n\
         ? animal_home(bee, $Home)\n\
         ? animal_home(spider, $Home)\n\
         ? animal_home(rock, $Home)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A bee lives in a hive; a spider lives in a web — the recalled home names.
    assert!(out.contains("\"Home\":\"hive\""), "bee → hive: {out}");
    assert!(out.contains("\"Home\":\"web\""), "spider → web: {out}");
    // The answer carries the Wikipedia citation as its proof, at the honest
    // `consensus` trust tier for a collaboratively edited encyclopedia.
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A rock is not an animal — honest abstention, never a fabricated home.
    assert!(out.contains("\"abstained\":true"), "rock abstains: {out}");
}
