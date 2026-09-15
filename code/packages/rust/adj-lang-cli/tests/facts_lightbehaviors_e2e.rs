//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/light-behaviors.adj`) driven through the built
//! CLI: a native `table` of the ways a light wave interacts with matter → the
//! effect the source states resolves binding-query recalls (forward AND
//! backward) with the source's NASA Science citation, and abstains on a word
//! that is not one of these light behaviors (gravity) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the reflection sentence, the primary source of all five answers; it is now
//! a framing sentence from the page, which every row overrides. Each answer's
//! citations array is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsl_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://science.nasa.gov/ems/03_behaviors/";
const ENVELOPE: &str = "Light waves across the electromagnetic spectrum behave in similar ways.";
const REFLECTION: &str = "Reflection is when incident light (incoming light) hits an object and bounces off.";
const REFRACTION: &str = "Refraction is when light waves change direction as they pass from one medium to another.";
const ABSORPTION: &str = "Absorption occurs when photons from incident light hit atoms and molecules and cause them to vibrate.";
const DIFFRACTION: &str = "Diffraction is the bending and spreading of waves around an obstacle.";
const SCATTERING: &str = "Scattering occurs when light bounces off an object in a variety of directions.";

/// (behavior, effect, that row's own sentence)
const ROWS: [(&str, &str, &str); 5] = [
    ("reflection", "bounces_off", REFLECTION),
    ("refraction", "changes_direction", REFRACTION),
    ("absorption", "vibrate", ABSORPTION),
    ("diffraction", "bends_and_spreads", DIFFRACTION),
    ("scattering", "variety_of_directions", SCATTERING),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/light-behaviors.adj"))
        .expect("read shipped light-behaviors.adj");
    adj[adj.find("table light_behavior").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/light-behaviors.adj");
    std::fs::copy(&src, dir.join("light-behaviors.adj")).expect("copy shipped light-behaviors.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"light-behaviors.adj\"\n{query}")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn physics_light_behaviors_recall_binds_effect_with_citation() {
    let out = ask(
        "lightbehaviors",
        "? light_behavior(reflection, $Effect)\n\
         ? light_behavior(refraction, $Effect)\n\
         ? light_behavior(diffraction, $Effect)\n\
         ? light_behavior($Behavior, bounces_off)\n\
         ? light_behavior(gravity, $Effect)\n",
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Reflection bounces off, refraction changes direction, diffraction bends
    // and spreads — the recalled effects (forward binds).
    assert!(
        out.contains("\"Effect\":\"bounces_off\""),
        "reflection → bounces_off: {out}"
    );
    assert!(
        out.contains("\"Effect\":\"changes_direction\""),
        "refraction → changes_direction: {out}"
    );
    assert!(
        out.contains("\"Effect\":\"bends_and_spreads\""),
        "diffraction → bends_and_spreads: {out}"
    );
    // The relation runs BACKWARD: bind the effect `bounces_off`, recall its
    // behavior.
    assert!(
        out.contains("\"Behavior\":\"reflection\""),
        "bounces_off → reflection (reverse recall): {out}"
    );
    // Each answer carries its own row's NASA sentence, whole, at the
    // `authoritative` tier — not `contains("science.nasa.gov") && contains(trust)`
    // (#15209), and no longer the reflection sentence for every answer (#14986).
    assert_eq!(out.matches("\"citations\":[").count(), 4, "four answers: {out}");
    assert_eq!(out.matches(&only_citation(REFLECTION)).count(), 2, "both reflection answers carry only its sentence: {out}");
    assert!(out.contains(&only_citation(REFRACTION)), "refraction: its own sentence: {out}");
    assert!(out.contains(&only_citation(DIFFRACTION)), "diffraction: its own sentence: {out}");
    // Gravity is a force, not a way light interacts with matter — honest
    // abstention, never a fabricated effect.
    assert!(out.contains("\"abstained\":true"), "gravity abstains: {out}");
}

#[test]
fn every_effect_answer_carries_its_own_sentence() {
    for (behavior, effect, sentence) in ROWS {
        let out = ask(&format!("row_{behavior}"), &format!("? light_behavior($Behavior, {effect})\n"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {effect}: {out}");
        assert!(out.contains(&format!("\"Behavior\":\"{behavior}\"")), "{effect} is {behavior}'s: {out}");
        assert!(out.contains(&only_citation(sentence)), "{behavior}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != behavior {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {behavior}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {behavior}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (behavior, effect, sentence) in ROWS {
        let expected = format!("    row ({behavior}, {effect}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({behavior}, {effect}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 5, "five row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{REFLECTION}\"\n    locator")), "not the reflection sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["reflect", "refract", "absor", "diffract", "scatter", "bounce", "bend", "vibrat", "direction"] {
        assert!(!folded.contains(word), "the envelope must name no behavior or effect, but contains {word:?}");
    }
}
