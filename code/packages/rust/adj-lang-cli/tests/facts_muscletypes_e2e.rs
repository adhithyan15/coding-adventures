//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-types.adj`) driven through the built CLI:
//! a native `table` of the three muscle-tissue types → the ONE distinctive
//! characteristic the source assigns each resolves binding-query recalls
//! (forward AND backward) with the source's NCI SEER Training Modules citation,
//! and abstains on a word that is not one of the three muscle-tissue types
//! (bone) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsm_{tag}_{}", std::process::id()));
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
fn biology_muscle_types_recall_binds_trait_with_citation() {
    let dir = scratch("muscletypes");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/muscle-types.adj");
    std::fs::copy(&src, dir.join("muscle-types.adj")).expect("copy shipped muscle-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-types.adj\"\n\
         ? muscle_trait(skeletal, $Trait)\n\
         ? muscle_trait(smooth, $Trait)\n\
         ? muscle_trait(cardiac, $Trait)\n\
         ? muscle_trait($Muscle, involuntary)\n\
         ? muscle_trait(bone, $Trait)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Skeletal muscle is voluntary, smooth muscle is involuntary, and cardiac
    // muscle is the one distinguished by intercalated disks — the recalled
    // traits (forward binds).
    assert!(
        out.contains("\"Trait\":\"voluntary\""),
        "skeletal → voluntary: {out}"
    );
    assert!(
        out.contains("\"Trait\":\"involuntary\""),
        "smooth → involuntary: {out}"
    );
    assert!(
        out.contains("\"Trait\":\"intercalated_disks\""),
        "cardiac → intercalated_disks: {out}"
    );
    // The relation runs BACKWARD: bind the trait `involuntary`, recall its
    // muscle type.
    assert!(
        out.contains("\"Muscle\":\"smooth\""),
        "involuntary → smooth (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Bone is not one of the three muscle-tissue types — honest abstention,
    // never a fabricated trait.
    assert!(out.contains("\"abstained\":true"), "bone abstains: {out}");
}
