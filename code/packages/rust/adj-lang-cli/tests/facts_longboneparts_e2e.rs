//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/long-bone-parts.adj`) driven through the built
//! CLI: a native `table` of long-bone region → defining descriptor resolves a
//! binding-query recall with the source's NCBI Bookshelf "Anatomy, Bones"
//! citation, runs the relation backward (descriptor → region), and abstains on
//! a non-region (a tendon) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslbp_{tag}_{}", std::process::id()));
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
fn anatomy_long_bone_parts_recall_binds_descriptor_with_citation() {
    let dir = scratch("longboneparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/long-bone-parts.adj");
    std::fs::copy(&src, dir.join("long-bone-parts.adj")).expect("copy shipped long-bone-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-bone-parts.adj\"\n\
         ? long_bone_part(diaphysis, $D)\n\
         ? long_bone_part(epiphysis, $D)\n\
         ? long_bone_part(metaphysis, $D)\n\
         ? long_bone_part(periosteum, $D)\n\
         ? long_bone_part(epiphyseal_plate, $D)\n\
         ? long_bone_part($P, shaft)\n\
         ? long_bone_part(tendon, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each region binds to the defining descriptor the source states verbatim.
    assert!(out.contains("\"D\":\"shaft\""), "diaphysis → shaft: {out}");
    assert!(out.contains("\"D\":\"tip_of_bone\""), "epiphysis → tip_of_bone: {out}");
    assert!(
        out.contains("\"D\":\"between_diaphysis_and_epiphysis\""),
        "metaphysis → between_diaphysis_and_epiphysis: {out}"
    );
    assert!(
        out.contains("\"D\":\"surrounds_bone_surface\""),
        "periosteum → surrounds_bone_surface: {out}"
    );
    assert!(
        out.contains("\"D\":\"linear_bone_growth\""),
        "epiphyseal_plate → linear_bone_growth: {out}"
    );
    // The relation runs backward: the descriptor `shaft` recalls the diaphysis.
    assert!(
        out.contains("\"P\":\"diaphysis\""),
        "shaft → diaphysis (reverse recall): {out}"
    );
    // The answer carries the NCBI Bookshelf "Anatomy, Bones" citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/books/NBK537199") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A tendon is not a region of a long bone — honest abstention, never a
    // fabricated descriptor.
    assert!(out.contains("\"abstained\":true"), "unknown region abstains: {out}");
}
