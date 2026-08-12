//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/sun-layer.adj`) driven through the built
//! CLI: a native `table` naming two layers of the Sun and what each
//! actually is, quoted verbatim from NASA's "Layers of the Sun" blog
//! post. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_sun_layer_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/sun-layer.adj");
    std::fs::copy(&src, dir.join("sun-layer.adj")).expect("copy shipped sun-layer.adj");
}

#[test]
fn sun_layer_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer(photosphere, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"the_visible_surface_of_the_sun\""),
        "photosphere means the_visible_surface_of_the_sun: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn sun_layer_reverse_binds_the_layer_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer($L, the_suns_outer_atmosphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"corona\""),
        "the shipped the_suns_outer_atmosphere example is corona: {out}"
    );
}

#[test]
fn sun_layer_abstains_honestly_on_an_untabled_layer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer(chromosphere, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "chromosphere is a real solar layer the same source names, but its sentence bundles position with a temperature range rather than one clean fact -- honest abstention, never invented: {out}"
    );
}
