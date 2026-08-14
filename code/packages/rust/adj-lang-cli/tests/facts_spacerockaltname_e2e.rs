//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/space-rock-alt-name.adj`) driven through the
//! built CLI: a native `table` recording the two everyday alternate names
//! NASA gives a meteor, decoded from the SAME sentence already tabled as a
//! single compound description in `space-rock-stage.adj` -- a sibling
//! decoding the two synonym terms that sentence's own compound clause
//! already names. Resolves forward and backward recall queries with the
//! source's citation, plus honest abstention on meteoroid (whose cited
//! span names no alternate everyday term) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_spacerockaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/space-rock-alt-name.adj");
    std::fs::copy(&src, dir.join("space-rock-alt-name.adj"))
        .expect("copy shipped space-rock-alt-name.adj");
}

#[test]
fn space_rock_alt_name_recalls_meteor_with_citation() {
    let dir = scratch("meteor");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-alt-name.adj\"\n\
         ? space_rock_alt_name(meteor, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"space_rock_alt_name(meteor, fireball)\""),
        "meteor should recall its cited alternate name: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn space_rock_alt_name_backward_recalls_meteor_for_fireball() {
    let dir = scratch("fireball");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-alt-name.adj\"\n\
         ? space_rock_alt_name($Stage, fireball)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"space_rock_alt_name(meteor, fireball)\""),
        "meteor should be the only recalled stage called a fireball: {out}"
    );
}

#[test]
fn space_rock_alt_name_abstains_on_meteoroid() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"space-rock-alt-name.adj\"\n\
         ? space_rock_alt_name(meteoroid, $AltNameMeteoroid)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "meteoroid's cited span names no alternate everyday term -- honest abstention expected: {out}"
    );
}
