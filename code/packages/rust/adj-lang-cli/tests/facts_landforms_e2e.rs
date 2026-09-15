//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/landforms.adj`) driven through the built CLI:
//! a native `table` of common landform → the short defining descriptor its
//! source states resolves binding-query recalls (forward AND backward) with the
//! USGS-hosted Feature Type Thesaurus citation, and abstains on `ocean` (a body
//! of water, not a landform) — 0 model calls.
//!
//! Each row CITES its own landform's definition. `cites`, not `source`: on the
//! page the landform is a separate term link and the definition a span beneath
//! it that never names the landform (#13934's held question). The envelope used
//! to be mountain's definition, and so was the primary source of every other
//! answer; it is now the thesaurus's own description, which names no landform.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslf_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3";
const ENVELOPE: &str = "Types of named geographic features.";
const OLD_ENVELOPE: &str = "Landmasses that project conspicuously above their surroundings.";

/// (landform, descriptor, the definition that row cites) -- generated from the
/// converter's page-verified spans, not retyped.
const ROWS: [(&str, &str, &str); 5] = [
    ("mountain", "projects_above_surroundings", "Landmasses that project conspicuously above their surroundings."),
    ("valley", "low_between_mountains", "Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet."),
    ("plateau", "flat_elevated", "Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level."),
    ("plain", "comparatively_level", "Regions of general uniform slope, comparatively level and of considerable extent."),
    ("canyon", "deep_narrow", "Relatively narrow, deep depressions with steep sides, the bottom of which generally has a continuous slope"),
];

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("geography/landforms.adj"))
        .expect("read shipped landforms.adj");
    adj[adj.find("table landform_description").expect("table")..].to_string()
}

/// The whole primary citation, as the serializer emits it.
fn primary() -> String {
    format!("\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\"")
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(facts_stdlib().join("geography/landforms.adj"), dir.join("landforms.adj"))
        .expect("copy shipped landforms.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"landforms.adj\"\n? {query}\n")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn landforms_recall_binds_description_with_citation() {
    let dir = scratch("landforms");
    // Copy the shipped geography table beside the entry program and import it.
    let src = facts_stdlib().join("geography/landforms.adj");
    std::fs::copy(&src, dir.join("landforms.adj")).expect("copy shipped landforms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"landforms.adj\"\n\
         ? landform_description(mountain, $Desc)\n\
         ? landform_description(canyon, $Desc)\n\
         ? landform_description(plateau, $Desc)\n\
         ? landform_description($Landform, deep_narrow)\n\
         ? landform_description(ocean, $Desc)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Desc\":\"projects_above_surroundings\""),
        "mountain → projects_above_surroundings: {out}"
    );
    assert!(out.contains("\"Desc\":\"deep_narrow\""), "canyon → deep_narrow: {out}");
    assert!(out.contains("\"Desc\":\"flat_elevated\""), "plateau → flat_elevated: {out}");
    assert!(
        out.contains("\"Landform\":\"canyon\""),
        "deep_narrow → canyon (reverse recall): {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("apps.usgs.gov") && contains(trust)`
    // (#15209's two-loose-needles shape).
    assert!(out.contains(&primary()), "carries the USGS source citation, whole: {out}");
    assert!(out.contains("\"abstained\":true"), "ocean abstains: {out}");
}

#[test]
fn every_row_is_corroborated_by_its_own_landforms_definition_only() {
    for (landform, desc, span) in ROWS {
        // BIND THE LANDFORM: a fully ground query is ranked as a hypothesis
        // and carries no citations, which would let the negative arm below
        // pass over an empty answer.
        let out = ask(&format!("corr_{landform}"), &format!("landform_description($L, {desc})"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {landform}: {out}");
        assert!(out.contains(&format!("\"L\":\"{landform}\"")), "{desc} binds {landform}: {out}");
        assert!(
            out.contains(&format!(
                "{},\"corroborations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\"}}]",
                primary()
            )),
            "{landform}: envelope primary, its own definition the only corroboration: {out}"
        );
        for (other, _, other_span) in ROWS {
            if other != landform {
                assert!(!out.contains(other_span), "the {other} definition must not reach {landform}: {out}");
            }
        }
    }
}

#[test]
fn no_row_carries_a_source_and_no_cites_is_table_level() {
    let body = shipped_table();
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("row (") && l.trim_end().ends_with('{'))
            .count(),
        5,
        "every row opens its own block"
    );
    let cites: Vec<&str> = body.lines().filter(|l| l.trim_start().starts_with("cites \"")).collect();
    assert_eq!(cites.len(), 5, "one corroboration per row: {cites:?}");
    assert!(cites.iter().all(|l| l.starts_with("        cites \"")), "no table-level `cites`: {cites:?}");
    let source_lines: Vec<&str> = body.lines().filter(|l| l.trim_start().starts_with("source \"")).collect();
    assert_eq!(
        source_lines,
        vec![format!("    source \"{ENVELOPE}\"").as_str()],
        "exactly one `source` line in the table -- the envelope's"
    );
    let out = ask("nosource", "landform_description(valley, $D)");
    assert!(out.contains(&primary()), "the envelope is the PRIMARY source: {out}");
}

#[test]
fn the_envelope_describes_the_thesaurus_and_names_no_landform() {
    let body = shipped_table();
    assert!(
        !body.contains(&format!("source \"{OLD_ENVELOPE}\"")),
        "the envelope must not be mountain's definition again"
    );
    assert!(body.contains(&format!("    source \"{ENVELOPE}\"")), "the envelope is shipped");
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            keys += 1;
            assert!(!folded.contains(key), "the envelope must name no landform, but names {key:?}");
        }
    }
    assert_eq!(keys, 5, "all five row keys were actually checked");
    for (landform, desc, span) in ROWS {
        assert!(
            body.contains(&format!("    row ({landform}, {desc}) {{\n        cites \"{span}\" locator \"{LOCATOR}\"\n    }}")),
            "row ({landform}, {desc}) is shipped with its own definition"
        );
    }
}
