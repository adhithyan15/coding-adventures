//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/galaxy-types.adj`) driven through the built CLI:
//! a native `table` of the main galaxy types → their defining shape resolves
//! binding-query recalls (forward AND backward) with the source's NASA Science
//! citation, and abstains on a word that is not one of the main galaxy types (a
//! planet) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! spiral sentence, the primary source of all five answers; it is now the
//! page's introduction to its types, which every row overrides. The
//! lenticular row carries two sentences ("They have ..." alone names no type).
//! Each answer's citations array is pinned whole.

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

const LOCATOR: &str = "https://science.nasa.gov/universe/galaxies/types/";
const ENVELOPE: &str = "Scientists sometimes categorize galaxies based on their shapes and physical features.";
const SPIRAL: &str = "Our Milky Way is one example of a broad class of galaxies defined by the presence of spiral arms.";
const BARRED: &str = "Barred spirals sport ribbons of stars, gas, and dust that cut across their centers.";
const ELLIPTICAL: &str = "Elliptical galaxies have shapes that range from completely round to oval.";
const LENTICULAR: &str = "Lenticular galaxies are a kind of cross between spirals and ellipticals. They have the central bulge and disk common to spiral galaxies but no arms.";
const IRREGULAR: &str = "Irregular galaxies have unusual shapes, like toothpicks, rings, or even little groupings of stars.";

/// (type, shape, that row's own span)
const ROWS: [(&str, &str, &str); 5] = [
    ("spiral", "spiral_arms", SPIRAL),
    ("barred_spiral", "bar_across_center", BARRED),
    ("elliptical", "round_to_oval", ELLIPTICAL),
    ("lenticular", "disk_no_arms", LENTICULAR),
    ("irregular", "unusual_shapes", IRREGULAR),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/galaxy-types.adj"))
        .expect("read shipped galaxy-types.adj");
    adj[adj.find("table galaxy_shape").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/galaxy-types.adj");
    std::fs::copy(&src, dir.join("galaxy-types.adj")).expect("copy shipped galaxy-types.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"galaxy-types.adj\"\n{query}")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn astronomy_galaxy_types_recall_binds_shape_with_citation() {
    let out = ask(
        "galaxytypes",
        "? galaxy_shape(spiral, $Shape)\n\
         ? galaxy_shape(elliptical, $Shape)\n\
         ? galaxy_shape(irregular, $Shape)\n\
         ? galaxy_shape($Type, round_to_oval)\n\
         ? galaxy_shape(planet, $Shape)\n",
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A spiral galaxy is defined by its spiral arms, an elliptical ranges from
    // round to oval, an irregular has unusual shapes — the recalled shapes
    // (forward binds).
    assert!(
        out.contains("\"Shape\":\"spiral_arms\""),
        "spiral → spiral_arms: {out}"
    );
    assert!(
        out.contains("\"Shape\":\"round_to_oval\""),
        "elliptical → round_to_oval: {out}"
    );
    assert!(
        out.contains("\"Shape\":\"unusual_shapes\""),
        "irregular → unusual_shapes: {out}"
    );
    // The relation runs BACKWARD: bind the shape `round_to_oval`, recall its
    // galaxy type.
    assert!(
        out.contains("\"Type\":\"elliptical\""),
        "round_to_oval → elliptical (reverse recall): {out}"
    );
    // Each answer carries its own row's NASA Science sentence, whole, at the
    // `authoritative` tier — not `contains("science.nasa.gov") && contains(trust)`
    // (#15209), and no longer the spiral sentence for every answer (#14986).
    assert_eq!(out.matches("\"citations\":[").count(), 4, "four answers: {out}");
    assert!(out.contains(&only_citation(SPIRAL)), "spiral: its own sentence: {out}");
    assert_eq!(out.matches(&only_citation(ELLIPTICAL)).count(), 2, "both elliptical answers: its own sentence: {out}");
    assert!(out.contains(&only_citation(IRREGULAR)), "irregular: its own sentence: {out}");
    // A planet is not one of the main galaxy types — honest abstention, never a
    // fabricated shape.
    assert!(out.contains("\"abstained\":true"), "planet abstains: {out}");
}

#[test]
fn every_shape_answer_carries_its_own_span() {
    for (kind, shape, span) in ROWS {
        let out = ask(&format!("row_{kind}"), &format!("? galaxy_shape($Type, {shape})\n"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {shape}: {out}");
        assert!(out.contains(&format!("\"Type\":\"{kind}\"")), "{shape} is the {kind} galaxy's: {out}");
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
fn the_lenticular_row_carries_the_sentence_that_names_it() {
    // "They have the central bulge and disk common to spiral galaxies but no
    // arms." names no type; the sentence before it does. The row carries both.
    let body = shipped_table();
    assert!(body.contains(&format!("    row (lenticular, disk_no_arms) {{\n        source \"{LENTICULAR}\"\n    }}")));
    assert!(!body.contains("        source \"They have the central bulge"), "no row is warranted by 'They have ...' alone");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, shape, span) in ROWS {
        let expected = format!("    row ({kind}, {shape}) {{\n        source \"{span}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {shape}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 5, "five row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's introduction to its types"
    );
    assert!(!body.contains(&format!("\n    source \"{SPIRAL}\"\n    locator")), "not the spiral sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["spiral", "bar", "ellip", "round", "oval", "lenticul", "disk", "arm", "irregular", "unusual"] {
        assert!(!folded.contains(word), "the envelope must name no type or shape value, but contains {word:?}");
    }
}
