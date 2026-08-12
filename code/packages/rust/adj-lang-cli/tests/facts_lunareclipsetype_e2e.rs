//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/lunar-eclipse-type.adj`) driven through the
//! built CLI: a native `table` naming the three named lunar eclipse types
//! and what each actually is, quoted verbatim from NASA's "Eclipses and
//! the Moon" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_lunar_eclipse_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/lunar-eclipse-type.adj");
    std::fs::copy(&src, dir.join("lunar-eclipse-type.adj")).expect("copy shipped lunar-eclipse-type.adj");
}

#[test]
fn lunar_eclipse_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type(total_lunar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra\""),
        "total_lunar_eclipse means the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn lunar_eclipse_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type($T, the_moon_travels_through_earths_penumbra_the_faint_outer_part_of_its_shadow)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"penumbral_eclipse\""),
        "the shipped the_moon_travels_through_earths_penumbra example is penumbral_eclipse: {out}"
    );
}

#[test]
fn lunar_eclipse_type_abstains_honestly_on_an_untabled_nickname() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type(blood_moon, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "blood_moon is a real term the source discusses, but as a nickname for the color effect, not a fourth peer eclipse type -- honest abstention, never invented: {out}"
    );
}
