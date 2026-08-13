//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-baby-sex.adj`) driven through the
//! built CLI: a native `table` naming a baby animal's SEX-SPECIFIC name
//! where the source distinguishes one -- a sibling to the already-shipped
//! `animal-babies.adj` (which only carries ONE generic baby name per
//! animal), decoding a span already sitting unused inside that table's own
//! `source` field. Resolves binding-query recall with the source's
//! citation, and abstains on an animal/sex pair the source does not
//! distinguish -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_animalbabysex_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/animal-baby-sex.adj");
    std::fs::copy(&src, dir.join("animal-baby-sex.adj"))
        .expect("copy shipped animal-baby-sex.adj");
}

#[test]
fn animal_baby_sex_recalls_both_horse_terms_with_citation() {
    let dir = scratch("horse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-baby-sex.adj\"\n\
         ? animal_baby_sex(horse, male, $Baby)\n\
         ? animal_baby_sex(horse, female, $Baby)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"animal_baby_sex(horse, male, colt)\""),
        "male horse baby is a colt: {out}"
    );
    assert!(
        out.contains("\"term\":\"animal_baby_sex(horse, female, filly)\""),
        "female horse baby is a filly: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn animal_baby_sex_abstains_honestly_on_an_undistinguished_animal() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-baby-sex.adj\"\n\
         ? animal_baby_sex(dog, male, $Baby)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "dog has no sex-specific baby name in the source -- honest abstention: {out}"
    );
}
