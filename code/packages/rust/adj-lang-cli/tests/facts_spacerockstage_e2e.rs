//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/space-rock-stage.adj`) driven through the
//! built CLI: a native `table` naming three stages a single rocky object
//! passes through -- meteoroid, meteor, meteorite -- quoted verbatim from
//! NASA Science's "Meteors & Meteorites" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_space_rock_stage_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/space-rock-stage.adj");
    std::fs::copy(&src, dir.join("space-rock-stage.adj")).expect("copy shipped space-rock-stage.adj");
}

#[test]
fn space_rock_stage_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-stage.adj\"\n\
         ? space_rock_stage(meteor, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere\""),
        "meteor means called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn space_rock_stage_reverse_binds_the_stage_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-stage.adj\"\n\
         ? space_rock_stage($S, still_a_rock_in_space)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"meteoroid\""),
        "the shipped still_a_rock_in_space example is meteoroid: {out}"
    );
}

#[test]
fn space_rock_stage_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-stage.adj\"\n\
         ? space_rock_stage(asteroid, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "asteroid is a real object the source mentions but never defines in a sentence of its own on this page -- honest abstention, never invented: {out}"
    );
}
