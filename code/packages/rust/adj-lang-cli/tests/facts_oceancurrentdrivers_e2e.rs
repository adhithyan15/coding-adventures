//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-current-drivers.adj`) driven
//! through the built CLI: a native `table` naming three ocean-current
//! categories and the physical driver that creates each, quoted verbatim
//! from NOAA National Ocean Service's "What is a current?" page -- a
//! sibling library to `ocean-zones.adj`, a different oceanography axis
//! (what moves the water, not how deep light reaches). 0 answer-time
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceancurrentdrivers_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-current-drivers.adj");
    std::fs::copy(&src, dir.join("ocean-current-drivers.adj"))
        .expect("copy shipped ocean-current-drivers.adj");
}

#[test]
fn ocean_current_driver_recall_binds_the_driver_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-drivers.adj\"\n\
         ? ocean_current_driver(wind_driven_currents, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"wind\""),
        "wind-driven currents are driven by wind: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_current_driver_reverse_binds_the_current_type_for_that_driver() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-drivers.adj\"\n\
         ? ocean_current_driver($C, tides)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"tidal_currents\""),
        "tides drive tidal currents: {out}"
    );
}

#[test]
fn ocean_current_driver_abstains_honestly_on_an_untabled_current_name() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-drivers.adj\"\n\
         ? ocean_current_driver(gulf_stream, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"gulf_stream\" is a real named current the source mentions but not one of the three driver categories tabled here -- honest abstention, never invented: {out}"
    );
}
