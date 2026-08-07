//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/kingdoms.adj`) driven through the built CLI:
//! a native `table` of the biological kingdoms of life → a representative
//! example organism resolves binding-query recalls (forward AND backward) with
//! the source's Science Notes citation at the `consensus` trust tier, and
//! abstains on a word that is not one of these kingdoms (a virus) — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn biology_kingdoms_recall_binds_example_with_citation() {
    let dir = scratch("kingdoms");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/kingdoms.adj");
    std::fs::copy(&src, dir.join("kingdoms.adj")).expect("copy shipped kingdoms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"kingdoms.adj\"\n\
         ? kingdom_example(animalia, $Example)\n\
         ? kingdom_example(fungi, $Example)\n\
         ? kingdom_example(bacteria, $Example)\n\
         ? kingdom_example($Kingdom, mushrooms)\n\
         ? kingdom_example(virus, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The animal kingdom's listed example is humans, the fungus kingdom's is
    // mushrooms, the bacteria kingdom's is cyanobacteria — the recalled examples
    // (forward binds).
    assert!(
        out.contains("\"Example\":\"humans\""),
        "animalia → humans: {out}"
    );
    assert!(
        out.contains("\"Example\":\"mushrooms\""),
        "fungi → mushrooms: {out}"
    );
    assert!(
        out.contains("\"Example\":\"cyanobacteria\""),
        "bacteria → cyanobacteria: {out}"
    );
    // The relation runs BACKWARD: bind the example `mushrooms`, recall its
    // kingdom.
    assert!(
        out.contains("\"Kingdom\":\"fungi\""),
        "mushrooms → fungi (reverse recall): {out}"
    );
    // The answer carries the Science Notes citation as its proof, at the
    // `consensus` trust tier for a secondary teaching source.
    assert!(
        out.contains("sciencenotes.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A virus is not placed in any of these kingdoms — honest abstention, never
    // a fabricated example.
    assert!(out.contains("\"abstained\":true"), "virus abstains: {out}");
}
