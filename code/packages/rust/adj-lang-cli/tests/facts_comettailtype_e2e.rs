//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/comet-tail-type.adj`) driven through the
//! built CLI: a native `table` naming the two separate tails a comet
//! actually has and the defining path each one traces, quoted verbatim from
//! NASA Space Place's "What Is a Comet?" page -- the same page the sibling
//! `comet-part.adj` already cites. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comet_tail_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/comet-tail-type.adj");
    std::fs::copy(&src, dir.join("comet-tail-type.adj")).expect("copy shipped comet-tail-type.adj");
}

#[test]
fn comet_tail_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type(dust_tail, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"traces_a_broad_gently_curving_path_away_from_the_sun\""),
        "dust_tail traces a broad, gently curving path away from the Sun: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn comet_tail_type_reverse_binds_the_tail_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type($T, always_points_directly_away_from_the_sun)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"ion_tail\""),
        "the shipped always_points_directly_away_from_the_sun example is ion_tail: {out}"
    );
}

#[test]
fn comet_tail_type_abstains_honestly_on_a_different_physical_part() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-tail-type.adj\"\n\
         ? comet_tail_type(coma, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "coma is a real comet physical part (tabled in the sibling comet-part.adj) but not one of the two tail sub-types tabled here -- honest abstention, never invented: {out}"
    );
}
