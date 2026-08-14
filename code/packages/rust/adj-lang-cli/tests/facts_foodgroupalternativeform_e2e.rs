//! End-to-end test for the nutrition FACTS library
//! (`adj-facts-stdlib/nutrition/food-group-alternative-form.adj`) driven
//! through the built CLI: a native `table` naming the non-solid-food
//! alternative form the SAME MyPlate definitional sentence already states
//! for three of the five food groups -- a sibling to the already-shipped
//! `food-groups.adj` (which only sorts whole, solid foods), decoding the
//! alternative-form half of spans already sitting unused inside that
//! table's own header quote. Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on grains, whose
//! own cited definitional sentence names no comparable alternative form
//! -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_foodgroupalternativeform_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("nutrition/food-group-alternative-form.adj");
    std::fs::copy(&src, dir.join("food-group-alternative-form.adj"))
        .expect("copy shipped food-group-alternative-form.adj");
}

#[test]
fn food_group_alternative_form_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"food-group-alternative-form.adj\"\n\
         ? food_group_alternative_form(fruits, $Form)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"food_group_alternative_form(fruits, hundred_percent_fruit_juice)\""),
        "100% fruit juice counts as fruits: {out}"
    );
    assert!(
        out.contains("myplate.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USDA MyPlate citation: {out}"
    );
}

#[test]
fn food_group_alternative_form_recalls_backward_from_a_bound_form() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"food-group-alternative-form.adj\"\n\
         ? food_group_alternative_form($Group, hundred_percent_vegetable_juice)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"food_group_alternative_form(vegetables, hundred_percent_vegetable_juice)\""),
        "100% vegetable juice names the vegetables group: {out}"
    );
}

#[test]
fn food_group_alternative_form_abstains_honestly_on_grains() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"food-group-alternative-form.adj\"\n\
         ? food_group_alternative_form(grains, $Form)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "grains' own cited span names no alternative form -- honest abstention: {out}"
    );
}
