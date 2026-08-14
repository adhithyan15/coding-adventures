//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/dwarf-planet-criterion-status.adj`) driven
//! through the built CLI: a native `table` recording, for each of the three
//! IAU planet criteria already tabled in `planet-criterion.adj`, whether a
//! dwarf planet like Pluto satisfies it, decoded from the SAME NASA sentence
//! that table's own header already quotes to justify excluding
//! `dwarf_planet` as a row -- a sibling decoding the per-criterion status
//! half of that already-verified quote. Resolves forward and backward
//! recall queries with the source's citation, plus honest abstention on a
//! non-criterion atom -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_dwarfplanetcriterionstatus_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/dwarf-planet-criterion-status.adj");
    std::fs::copy(&src, dir.join("dwarf-planet-criterion-status.adj"))
        .expect("copy shipped dwarf-planet-criterion-status.adj");
}

#[test]
fn dwarf_planet_criterion_status_recalls_cleared_orbit_with_citation() {
    let dir = scratch("cleared");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dwarf-planet-criterion-status.adj\"\n\
         ? dwarf_planet_criterion_status(cleared_orbit, $Status)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"dwarf_planet_criterion_status(cleared_orbit, not_met)\""),
        "cleared_orbit should recall its cited not-met status: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn dwarf_planet_criterion_status_backward_recalls_cleared_orbit_for_not_met() {
    let dir = scratch("notmet");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dwarf-planet-criterion-status.adj\"\n\
         ? dwarf_planet_criterion_status($Criterion, not_met)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"dwarf_planet_criterion_status(cleared_orbit, not_met)\""),
        "cleared_orbit should be the only recalled not-met criterion: {out}"
    );
    assert!(
        !out.contains("dwarf_planet_criterion_status(orbit, not_met)"),
        "orbit's cited status is met, not not_met: {out}"
    );
}

#[test]
fn dwarf_planet_criterion_status_abstains_on_non_criterion() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dwarf-planet-criterion-status.adj\"\n\
         ? dwarf_planet_criterion_status(mass, $StatusMass)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"mass\" is not one of the three IAU planet criteria -- honest abstention expected: {out}"
    );
}
