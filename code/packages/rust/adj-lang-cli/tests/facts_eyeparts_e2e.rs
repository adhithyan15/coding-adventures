//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/eye-parts.adj`) driven through the built CLI:
//! a native `table` of eye part → function resolves binding-query recalls with
//! the source's NEI "How the Eyes Work" citation, runs the relation backward
//! (function → part, recalling the lens for focuses_light), and abstains on a
//! non-listed part (the eardrum, which belongs to the ear) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factseye_{tag}_{}", std::process::id()));
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
fn anatomy_eye_parts_recall_binds_function_with_citation() {
    let dir = scratch("eyeparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/eye-parts.adj");
    std::fs::copy(&src, dir.join("eye-parts.adj")).expect("copy shipped eye-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-parts.adj\"\n\
         ? eye_part_function(retina, $F)\n\
         ? eye_part_function(cornea, $F)\n\
         ? eye_part_function(optic_nerve, $F)\n\
         ? eye_part_function($P, focuses_light)\n\
         ? eye_part_function(eardrum, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The retina turns light into electrical signals; the cornea bends light;
    // the optic nerve carries the signals to the brain — the recalled functions,
    // each a token verbatim from the NEI path-of-sight description.
    assert!(
        out.contains("\"F\":\"turns_light_into_signals\""),
        "retina → turns_light_into_signals: {out}"
    );
    assert!(out.contains("\"F\":\"bends_light\""), "cornea → bends_light: {out}");
    assert!(
        out.contains("\"F\":\"carries_signals_to_brain\""),
        "optic_nerve → carries_signals_to_brain: {out}"
    );
    // The relation runs backward: the function focuses_light recalls the lens.
    assert!(out.contains("\"P\":\"lens\""), "focuses_light → lens (reverse recall): {out}");
    // The answer carries the NEI citation as its proof.
    assert!(
        out.contains("nei.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The eardrum is not an eye part — honest abstention, never a fabricated
    // function.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown part abstains: {out}"
    );
}
