//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-current-drivers.adj`) driven
//! through the built CLI: a native `table` naming three ocean-current
//! categories and the physical driver that creates each, quoted from NOAA
//! National Ocean Service's "What is a current?" page -- a sibling library
//! to `ocean-zones.adj`, a different oceanography axis (what moves the
//! water, not how deep light reaches). 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! tidal sentence, the primary source of all three answers; it is now a
//! framing sentence from the page, which every row overrides. The
//! thermohaline row carries its numbered label with its sentence ("This is a
//! process ..." alone names no current type). Each answer's citations array
//! is pinned whole.

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

const LOCATOR: &str = "https://oceanservice.noaa.gov/facts/current.html";
const ENVELOPE: &str = "Oceanic currents describe the movement of water from one location to another.";
const TIDAL: &str = "Tides create a current in the oceans, which are strongest near the shore, and in bays and estuaries along the coast.";
const WIND: &str = "Winds drive currents that are at or near the ocean's surface.";
const THERMOHALINE: &str = "3. Thermohaline circulation. This is a process driven by density differences in water due to temperature (thermo) and salinity (haline) variations in different parts of the ocean.";

/// (current type, driver, that row's own span)
const ROWS: [(&str, &str, &str); 3] = [
    ("tidal_currents", "tides", TIDAL),
    ("wind_driven_currents", "wind", WIND),
    ("thermohaline_circulation", "density_differences_from_temperature_and_salinity", THERMOHALINE),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("oceanography/ocean-current-drivers.adj"))
        .expect("read shipped ocean-current-drivers.adj");
    adj[adj.find("table ocean_current_driver").expect("table")..].to_string()
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
    // ONE CONTIGUOUS RUN, not `contains("oceanservice.noaa.gov") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(WIND)), "the wind sentence is the only citation: {out}");
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

#[test]
fn every_driver_answer_carries_its_own_span() {
    for (kind, driver, span) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"ocean-current-drivers.adj\"\n? ocean_current_driver($C, {driver})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {driver}: {out}");
        assert!(out.contains(&format!("\"C\":\"{kind}\"")), "{driver} drives {kind}: {out}");
        assert!(out.contains(&only_citation(span)), "{kind}: its own span, whole, and the only citation: {out}");
        for (other, _, other_span) in ROWS {
            if other != kind {
                assert!(!out.contains(other_span), "the {other} span must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_thermohaline_row_carries_the_label_that_names_it() {
    // "This is a process driven by density differences ..." names no current
    // type; the numbered label before it does. The row carries both.
    let body = shipped_table();
    assert!(body.contains(&format!("        source \"{THERMOHALINE}\"\n")));
    assert!(!body.contains("        source \"This is a process"), "no row is warranted by 'This is a process' alone");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, driver, span) in ROWS {
        let expected = format!("    row ({kind}, {driver}) {{\n        source \"{span}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {driver}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{TIDAL}\"\n    locator")), "not the tidal sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["tide", "tidal", "wind", "thermohaline", "densit", "temperature", "salin"] {
        assert!(!folded.contains(word), "the envelope must name no current type or driver, but contains {word:?}");
    }
}
