//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/solar-eclipse-type.adj`) driven through
//! the built CLI: a native `table` naming three solar eclipse types and
//! what each actually is, quoted verbatim from NASA's "Types of Solar
//! Eclipses" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_solar_eclipse_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/solar-eclipse-type.adj");
    std::fs::copy(&src, dir.join("solar-eclipse-type.adj"))
        .expect("copy shipped solar-eclipse-type.adj");
}

#[test]
fn solar_eclipse_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type(total_solar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"completely_blocking_the_face_of_the_sun\""),
        "total_solar_eclipse means completely_blocking_the_face_of_the_sun: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn solar_eclipse_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type($T, moon_at_or_near_its_farthest_point_from_earth)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"annular_solar_eclipse\""),
        "the shipped moon_at_or_near_its_farthest_point_from_earth example is annular_solar_eclipse: {out}"
    );
}

#[test]
fn solar_eclipse_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type(hybrid_solar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hybrid_solar_eclipse is a real eclipse type the source covers but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
