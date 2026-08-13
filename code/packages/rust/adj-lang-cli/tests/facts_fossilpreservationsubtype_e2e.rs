//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/fossil-preservation-subtype.adj`) driven
//! through the built CLI: a native `table` naming a SPECIFIC KIND of one of
//! `fossil-preservation-type.adj`'s three peer preservation structures --
//! a sibling that decodes the `steinkern` definition already quoted inside
//! that table's own header, but structurally excluded there because the
//! source frames a steinkern as a specific kind of `cast`, not a fourth
//! peer type. Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a peer type (`mold`) that is not
//! itself a sub-category -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fossilpreservationsubtype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/fossil-preservation-subtype.adj");
    std::fs::copy(&src, dir.join("fossil-preservation-subtype.adj"))
        .expect("copy shipped fossil-preservation-subtype.adj");
}

#[test]
fn fossil_preservation_subtype_recalls_steinkern_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-subtype.adj\"\n\
         ? fossil_preservation_subtype(steinkern, $ParentType, $Description)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"ParentType\":\"cast\""),
        "steinkern's parent type is cast: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn fossil_preservation_subtype_recalls_backward_from_a_bound_parent_type() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-subtype.adj\"\n\
         ? fossil_preservation_subtype($Subtype, cast, $Description)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Subtype\":\"steinkern\""),
        "cast's sub-category is steinkern: {out}"
    );
}

#[test]
fn fossil_preservation_subtype_abstains_honestly_on_a_peer_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-subtype.adj\"\n\
         ? fossil_preservation_subtype(mold, $ParentType, $Description)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "mold is a peer type, not itself a subtype -- honest abstention: {out}"
    );
}
