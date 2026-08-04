//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/magnet-poles.adj`) driven through the built CLI:
//! a native `table` of magnetic pole pairing → interaction outcome resolves a
//! binding query recall with the NASA citation, runs backward (outcome →
//! pairing), and abstains on something that is not a pole pairing the source
//! names — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmp_{tag}_{}", std::process::id()));
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
fn physics_magnet_poles_recall_binds_outcome_with_citation() {
    let dir = scratch("magnetpoles");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/magnet-poles.adj");
    std::fs::copy(&src, dir.join("magnet-poles.adj")).expect("copy shipped magnet-poles.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"magnet-poles.adj\"\n\
         ? magnet_pole_interaction(like_poles, $Out)\n\
         ? magnet_pole_interaction(opposite_poles, $Out)\n\
         ? magnet_pole_interaction(north_to_north, $Out)\n\
         ? magnet_pole_interaction($P, attract)\n\
         ? magnet_pole_interaction(single_pole, $Out)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each pairing to the outcome NASA states.
    assert!(out.contains("\"Out\":\"repel\""), "like_poles binds to repel: {out}");
    assert!(out.contains("\"Out\":\"attract\""), "opposite_poles binds to attract: {out}");
    assert!(
        out.contains("magnet_pole_interaction(like_poles, repel)"),
        "like_poles is governing-bound to repel: {out}"
    );
    assert!(
        out.contains("magnet_pole_interaction(opposite_poles, attract)"),
        "opposite_poles is governing-bound to attract: {out}"
    );
    assert!(
        out.contains("magnet_pole_interaction(north_to_north, repel)"),
        "north_to_north is governing-bound to repel: {out}"
    );
    // The relation runs BACKWARD: bind the outcome, recall a pairing.
    assert!(
        out.contains("\"P\":\"opposite_poles\""),
        "reverse recall binds P=opposite_poles from attract: {out}"
    );
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A single pole is NOT a pole pairing the source names — honest abstention.
    assert!(out.contains("\"abstained\":true"), "single_pole abstains: {out}");
}
