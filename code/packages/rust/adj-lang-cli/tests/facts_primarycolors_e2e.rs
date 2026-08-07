//! End-to-end test for the art FACTS library
//! (`adj-facts-stdlib/art/primary-colors.adj`) driven through the built CLI:
//! a native `table` of color → is-a-traditional-(RYB)-primary resolves a binding
//! query recall with the Tate citation, and abstains on a non-primary
//! (secondary) color — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factspc_{tag}_{}", std::process::id()));
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
fn art_primary_colors_recall_binds_membership_with_citation() {
    let dir = scratch("primarycolors");
    // Copy the shipped art table beside the entry program and import it.
    let src = facts_stdlib().join("art/primary-colors.adj");
    std::fs::copy(&src, dir.join("primary-colors.adj")).expect("copy shipped primary-colors.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"primary-colors.adj\"\n\
         ? primary_color(red, $Is)\n\
         ? primary_color(blue, $Is)\n\
         ? primary_color(green, $Is)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Red and blue are traditional RYB primaries — each binds to `yes`.
    assert!(out.contains("\"Is\":\"yes\""), "a primary binds to yes: {out}");
    assert!(
        out.contains("primary_color(red, yes)"),
        "red is governing-bound to yes: {out}"
    );
    assert!(
        out.contains("primary_color(blue, yes)"),
        "blue is governing-bound to yes: {out}"
    );
    // The answer carries the Tate locator + trust tier as its proof.
    assert!(
        out.contains("tate.org.uk") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Green is a SECONDARY color — honest abstention, never a fabricated "yes".
    assert!(out.contains("\"abstained\":true"), "green abstains: {out}");
}
