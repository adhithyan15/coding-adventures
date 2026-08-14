//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/mammal-origin.adj`) driven through the built
//! CLI: a native `table` recording whether each of animal-classes.adj's
//! seven mammals is introduced or marsupial -- a sibling to the
//! already-shipped `animal-classes.adj` (which only carries which
//! vertebrate class an animal belongs to), decoding the origin clause
//! already sitting unused inside that table's own header quotes. Resolves
//! forward and backward recall queries with the source's citation -- 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mammalorigin_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/mammal-origin.adj");
    std::fs::copy(&src, dir.join("mammal-origin.adj"))
        .expect("copy shipped mammal-origin.adj");
}

#[test]
fn mammal_origin_recalls_kangaroo_as_marsupial_with_citation() {
    let dir = scratch("kangaroo");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mammal-origin.adj\"\n\
         ? mammal_origin(kangaroo, $Origin)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"mammal_origin(kangaroo, marsupial)\""),
        "kangaroo should recall as marsupial: {out}"
    );
    assert!(
        out.contains("australian.museum") && out.contains("\"trust\":\"authoritative\""),
        "carries the Australian Museum citation: {out}"
    );
}

#[test]
fn mammal_origin_backward_recalls_all_introduced_mammals() {
    let dir = scratch("introduced");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mammal-origin.adj\"\n\
         ? mammal_origin($Animal, introduced)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for animal in ["cat", "fox", "rabbit"] {
        assert!(
            out.contains(&format!("\"term\":\"mammal_origin({animal}, introduced)\"")),
            "{animal} should be recalled as introduced: {out}"
        );
    }
    assert!(
        !out.contains("mammal_origin(kangaroo, introduced)"),
        "kangaroo is marsupial, not introduced: {out}"
    );
}

#[test]
fn mammal_origin_covers_all_seven_mammals_without_abstention() {
    let dir = scratch("noabstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mammal-origin.adj\"\n\
         ? mammal_origin(cat, $O1)\n\
         ? mammal_origin(fox, $O2)\n\
         ? mammal_origin(rabbit, $O3)\n\
         ? mammal_origin(kangaroo, $O4)\n\
         ? mammal_origin(bandicoot, $O5)\n\
         ? mammal_origin(quoll, $O6)\n\
         ? mammal_origin(koala, $O7)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        !out.contains("\"abstained\":true"),
        "all seven mammals have an origin fact on record -- no abstention expected: {out}"
    );
}
