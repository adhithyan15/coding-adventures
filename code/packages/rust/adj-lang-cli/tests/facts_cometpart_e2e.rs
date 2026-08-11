//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/comet-part.adj`) driven through the built
//! CLI: a native `table` naming three parts of a comet and what each
//! actually is, quoted verbatim from NASA Space Place's "What Is a
//! Comet?" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comet_part_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/comet-part.adj");
    std::fs::copy(&src, dir.join("comet-part.adj")).expect("copy shipped comet-part.adj");
}

#[test]
fn comet_part_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part(nucleus, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"solid_frozen_core_at_the_heart_of_the_comet\""),
        "nucleus means solid_frozen_core_at_the_heart_of_the_comet: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn comet_part_reverse_binds_the_part_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part($P, fuzzy_cloud_of_gas_and_dust_around_the_nucleus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"coma\""),
        "the shipped fuzzy_cloud_of_gas_and_dust_around_the_nucleus example is coma: {out}"
    );
}

#[test]
fn comet_part_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part(short_period_comet, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "short_period_comet is a real comet-related term the source covers but not one of the three physical parts tabled here -- honest abstention, never invented: {out}"
    );
}
