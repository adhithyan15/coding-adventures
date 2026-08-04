//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/insect-parts.adj`) driven through the built CLI:
//! a native `table` of the three main body regions (tagmata) of an insect →
//! what each one bears / is used for resolves binding-query recalls (forward
//! AND backward) with the source's UF/IFAS EDIS entomology citation, and
//! abstains on a word that is not one of the three body regions (a `tail`) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn biology_insect_parts_recall_binds_feature_with_citation() {
    let dir = scratch("insectparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/insect-parts.adj");
    std::fs::copy(&src, dir.join("insect-parts.adj")).expect("copy shipped insect-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"insect-parts.adj\"\n\
         ? insect_region_bears(head, $Bears)\n\
         ? insect_region_bears(thorax, $Bears)\n\
         ? insect_region_bears(abdomen, $Bears)\n\
         ? insect_region_bears($Region, legs_and_wings)\n\
         ? insect_region_bears(tail, $Bears)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The head carries the eyes/antennae/mouthparts, the legs and wings attach
    // at the thorax, the abdomen is used for digestion and reproduction — the
    // recalled features (forward binds).
    assert!(
        out.contains("\"Bears\":\"eyes_antennae_and_mouthparts\""),
        "head → eyes_antennae_and_mouthparts: {out}"
    );
    assert!(
        out.contains("\"Bears\":\"legs_and_wings\""),
        "thorax → legs_and_wings: {out}"
    );
    assert!(
        out.contains("\"Bears\":\"digestion_and_reproduction\""),
        "abdomen → digestion_and_reproduction: {out}"
    );
    // The relation runs BACKWARD: bind the feature `legs_and_wings`, recall its
    // body region.
    assert!(
        out.contains("\"Region\":\"thorax\""),
        "legs_and_wings → thorax (reverse recall): {out}"
    );
    // The answer carries the UF/IFAS EDIS citation as its proof, at the
    // `authoritative` trust tier for a university entomology teaching resource.
    assert!(
        out.contains("ask.ifas.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // An insect has no tail region — `tail` is not one of the three main body
    // regions, so recall abstains honestly, never a fabricated feature.
    assert!(out.contains("\"abstained\":true"), "tail abstains: {out}");
}
