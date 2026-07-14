//! End-to-end test for the nutrition FACTS library
//! (`adj-facts-stdlib/nutrition/food-groups.adj`) driven through the built CLI:
//! a native `table` of food → USDA MyPlate food group resolves binding-query
//! recalls with the source's citation, and abstains on a non-food — 0 model calls.

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
fn nutrition_food_groups_recall_binds_group_with_citation() {
    let dir = scratch("foodgroups");
    // Copy the shipped nutrition table beside the entry program and import it.
    let src = facts_stdlib().join("nutrition/food-groups.adj");
    std::fs::copy(&src, dir.join("food-groups.adj")).expect("copy shipped food-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"food-groups.adj\"\n\
         ? food_group(apple, $Group)\n\
         ? food_group(chicken, $Group)\n\
         ? food_group(rock, $Group)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // An apple is a fruit; chicken is a protein food — the recalled groups.
    assert!(out.contains("\"Group\":\"fruits\""), "apple → fruits: {out}");
    assert!(out.contains("\"Group\":\"protein\""), "chicken → protein: {out}");
    // The answer carries the USDA MyPlate citation as its proof.
    assert!(
        out.contains("myplate.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A rock is not a food — honest abstention, never a fabricated group.
    assert!(out.contains("\"abstained\":true"), "rock abstains: {out}");
}
