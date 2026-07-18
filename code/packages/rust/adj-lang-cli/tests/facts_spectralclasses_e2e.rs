//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/spectral-classes.adj`) driven through the built
//! CLI: a native `table` of the seven main-sequence stellar spectral classes →
//! the color NASA assigns each resolves binding-query recalls (forward AND
//! backward) with the source's NASA Science citation, and abstains on a letter
//! that is not one of the seven classes (`z`) — 0 model calls.

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
fn astronomy_spectral_classes_recall_binds_color_with_citation() {
    let dir = scratch("spectralclasses");
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/spectral-classes.adj");
    std::fs::copy(&src, dir.join("spectral-classes.adj"))
        .expect("copy shipped spectral-classes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"spectral-classes.adj\"\n\
         ? spectral_class_color(o, $Color)\n\
         ? spectral_class_color(g, $Color)\n\
         ? spectral_class_color(m, $Color)\n\
         ? spectral_class_color($Class, red)\n\
         ? spectral_class_color(z, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // O is the hottest, bluest class; G is our Sun's yellow class; M is the
    // coolest, reddest class — the recalled colors (forward binds).
    assert!(
        out.contains("\"Color\":\"blue\""),
        "o → blue: {out}"
    );
    assert!(
        out.contains("\"Color\":\"yellow\""),
        "g → yellow: {out}"
    );
    assert!(
        out.contains("\"Color\":\"red\""),
        "m → red: {out}"
    );
    // The relation runs BACKWARD: bind the color `red`, recall its spectral class.
    assert!(
        out.contains("\"Class\":\"m\""),
        "red → m (reverse recall): {out}"
    );
    // The answer carries the NASA Science citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Z is not one of the seven main-sequence spectral classes — honest
    // abstention, never a fabricated color.
    assert!(out.contains("\"abstained\":true"), "z abstains: {out}");
}
