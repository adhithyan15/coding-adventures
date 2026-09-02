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

const AH_PREFIX_PIN: &str = r#""bindings":{"Home":"web"},"citations":[{"source":"A beehive is an enclosed structure in which honey bees raise their young and produce honey as part of their seasonal cycle.","locator":"https://en.wikipedia.org/wiki/Beehive","trust":"consensus","corroborations":[{"source":"A bird nest is the spot in which a bird lays and incubates its eggs and raises its young.","locator":"https://en.wikipedia.org/wiki/Bird_nest"},{"source":"A spider web, spiderweb, spider's web, cobweb or even just web (from the Middle English coppeweb)[1] is a structure created by a spider out of proteinaceous spider silk extruded from its spinnerets, generally meant to catch its prey.","locator":"https://en.wikipedia.org/wiki/Spider_web""#;

const AH_ALL_PIN: &str = r#""bindings":{"Home":"hive"},"citations":[{"source":"A beehive is an enclosed structure in which honey bees raise their young and produce honey as part of their seasonal cycle.","locator":"https://en.wikipedia.org/wiki/Beehive","trust":"consensus","corroborations":[{"source":"A bird nest is the spot in which a bird lays and incubates its eggs and raises its young.","locator":"https://en.wikipedia.org/wiki/Bird_nest"},{"source":"A spider web, spiderweb, spider's web, cobweb or even just web (from the Middle English coppeweb)[1] is a structure created by a spider out of proteinaceous spider silk extruded from its spinnerets, generally meant to catch its prey.","locator":"https://en.wikipedia.org/wiki/Spider_web"},{"source":"The European rabbit notably lives in extensive burrow networks called warrens.","locator":"https://en.wikipedia.org/wiki/Rabbit"},{"source":"Beavers make two types of lodges: bank lodges and open-water lodges.","locator":"https://en.wikipedia.org/wiki/Beaver""#;

#[test]
fn animal_homes_spider_answer_carries_its_full_unelided_sentence() {
    let dir = scratch("cite_spider");
    std::fs::copy(
        facts_stdlib().join("biology/animal-homes.adj"),
        dir.join("animal-homes.adj"),
    )
    .expect("copy shipped animal-homes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-homes.adj\"\n? animal_home(spider, $Home)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The pin extends through spider's OWN corroboration, which is the
    // SECOND in the list. The first version stopped after corroboration[0]
    // -- bird's sentence -- so it was unique, anchored, and bound to the
    // wrong evidence while the test name claimed otherwise. Restoring the
    // old elided spider quote left it GREEN. Only a directional mutation
    // found that; it is the same defect as the chamber-branch pin.
    //
    // The library header used to quote this as "A spider web ... is a
    // structure created by a spider..." -- eliding the whole alias list. That
    // shortened form names the subject only if you supply the elided material
    // yourself. The full sentence names BOTH the subject and the row's value,
    // and the `[1]` is a real rendered footnote marker. Pinning the unelided
    // form is what stops it being trimmed back.
    assert!(
        out.contains(AH_PREFIX_PIN),
        "spider's answer carries the full unelided Wikipedia sentence: {out}"
    );
}

#[test]
fn animal_homes_bee_answer_carries_all_four_corroborations_in_order() {
    let dir = scratch("cite_bee");
    std::fs::copy(
        facts_stdlib().join("biology/animal-homes.adj"),
        dir.join("animal-homes.adj"),
    )
    .expect("copy shipped animal-homes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-homes.adj\"\n? animal_home(bee, $Home)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Spans the WHOLE corroboration list, so a reorder or a dropped middle
    // entry fails here even though every sentence is still present somewhere
    // in the blob. A pure reorder is invisible to any per-sentence check.
    assert!(
        out.contains(AH_ALL_PIN),
        "bee's answer carries all four corroborations in order: {out}"
    );
}
