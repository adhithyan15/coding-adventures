//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/flame-colors.adj`) driven through the built CLI:
//! a native `table` of each metal → the flame-test color it gives resolves
//! binding-query recalls (forward AND backward) with the source's University of
//! Washington Department of Chemistry citation, and abstains on a metal whose
//! flame color this source does not state (copper) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsf_{tag}_{}", std::process::id()));
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
fn chemistry_flame_colors_recall_binds_color_with_citation() {
    let dir = scratch("flamecolors");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/flame-colors.adj");
    std::fs::copy(&src, dir.join("flame-colors.adj")).expect("copy shipped flame-colors.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"flame-colors.adj\"\n\
         ? flame_color(sodium, $Color)\n\
         ? flame_color(potassium, $Color)\n\
         ? flame_color(calcium, $Color)\n\
         ? flame_color($Metal, violet)\n\
         ? flame_color(copper, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Sodium glows orange, potassium violet, calcium red_orange — the recalled
    // colors (forward binds), each the source's own color word.
    assert!(
        out.contains("\"Color\":\"orange\""),
        "sodium → orange: {out}"
    );
    assert!(
        out.contains("\"Color\":\"violet\""),
        "potassium → violet: {out}"
    );
    assert!(
        out.contains("\"Color\":\"red_orange\""),
        "calcium → red_orange: {out}"
    );
    // The relation runs BACKWARD: bind the color `violet`, recall its metal.
    assert!(
        out.contains("\"Metal\":\"potassium\""),
        "violet → potassium (reverse recall): {out}"
    );
    // The answer carries the University of Washington Department of Chemistry
    // citation as its proof, at the `authoritative` trust tier for a university
    // chemistry department source.
    assert!(
        out.contains("chem.washington.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Copper's flame color is not stated on this source page — honest
    // abstention, never a fabricated color.
    assert!(out.contains("\"abstained\":true"), "copper abstains: {out}");
}
