//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/seed-dispersal-mechanism.adj`) driven through
//! the built CLI: a native `table` naming four seed-dispersal mechanisms
//! and how each actually works, quoted verbatim from Wikipedia's "Seed
//! dispersal" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! barochory quote, so a ballochory, anemochory or epizoochory answer was
//! warranted primarily by a sentence about gravity dispersal.
//!
//! THE ANEMOCHORY ROW CARRIES TWO SENTENCES. Its quote opens "Wind dispersal
//! can take on one of two primary forms" and never says "anemochory", so read
//! detached from its page it grounds nothing. The sentence before it in the
//! same paragraph names the mechanism, and the two are contiguous -- the
//! remedy ../README.md prescribes under "A citation must name its own
//! subject". Trust is `consensus` here, not `authoritative`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_seed_dispersal_mechanism_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/seed-dispersal-mechanism.adj");
    std::fs::copy(&src, dir.join("seed-dispersal-mechanism.adj"))
        .expect("copy shipped seed-dispersal-mechanism.adj");
}

#[test]
fn seed_dispersal_mechanism_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism(barochory, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"uses_gravity_as_a_simple_means_of_seed_dispersal\""),
        "barochory means uses_gravity_as_a_simple_means_of_seed_dispersal: {out}"
    );
    // This was `contains("en.wikipedia.org") && contains("\"trust\":\"consensus\"")`,
    // which any Wikipedia citation satisfies and which constrains no sentence
    // text at all. The needle is now the whole citations array as the serialiser
    // emits it, closing on both the corroborations `]` and the citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(BAROCHORY)),
        "the barochory answer carries its own subsection sentence, whole: {out}"
    );
    for other in [BALLOCHORY, ANEMOCHORY, EPIZOOCHORY] {
        assert!(!out.contains(other), "another mechanism's span must not reach it: {out}");
    }
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
}

const LOCATOR: &str = "https://en.wikipedia.org/wiki/Seed_dispersal";
const ENVELOPE: &str = "Seeds can be dispersed away from the parent plant individually or collectively, as well as dispersed in both space and time.";
const BAROCHORY: &str = "Barochory or the plant use of gravity for dispersal is a simple means of achieving seed dispersal.";
const BALLOCHORY: &str = "Ballochory is a type of dispersal where the seed is forcefully ejected by explosive dehiscence of the fruit.";
/// Two sentences, contiguous in one paragraph. The second names no mechanism,
/// so the row carries the sentence before it, which does.
const ANEMOCHORY: &str = "Wind dispersal (anemochory) is one of the more primitive means of dispersal. Wind dispersal can take on one of two primary forms: seeds or fruits can float on the breeze or, alternatively, they can flutter to the ground.";
const ANEMO_TAIL: &str = "Wind dispersal can take on one of two primary forms: seeds or fruits can float on the breeze or, alternatively, they can flutter to the ground.";
const EPIZOOCHORY: &str = "Seeds can be transported on the outside of vertebrate animals (mostly mammals), a process known as epizoochory.";

/// (mechanism, its description atom, the span that names both)
fn mechanisms() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("barochory", "uses_gravity_as_a_simple_means_of_seed_dispersal", BAROCHORY),
        (
            "ballochory",
            "seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit",
            BALLOCHORY,
        ),
        ("anemochory", "seeds_float_on_the_breeze_or_flutter_to_the_ground", ANEMOCHORY),
        ("epizoochory", "transported_on_the_outside_of_vertebrate_animals", EPIZOOCHORY),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/seed-dispersal-mechanism.adj"))
        .expect("read shipped seed-dispersal-mechanism.adj");
    adj[adj.find("table seed_dispersal_mechanism").expect("table")..].to_string()
}

#[test]
fn every_mechanism_answer_carries_only_the_span_that_names_it() {
    for (mech, description, span) in mechanisms() {
        let dir = scratch(&format!("mech_{mech}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"seed-dispersal-mechanism.adj\"\n? seed_dispersal_mechanism({mech}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {mech}: {out}");
        assert!(out.contains(&format!("\"D\":\"{description}\"")), "{mech} -> {description}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{mech}: its own span, whole, at the consensus tier, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(mech),
            "the span carried by {mech} names {mech} -- the defect was that it did not"
        );
        for other in [BAROCHORY, BALLOCHORY, ANEMOCHORY, EPIZOOCHORY] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another mechanism's span must not reach {mech}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {mech}: {out}");
    }
}

#[test]
fn the_anemochory_row_carries_the_sentence_that_names_the_mechanism() {
    // "Wind dispersal can take on one of two primary forms: ..." never says
    // "anemochory", so a row warranted by it alone would cite a sentence that
    // cannot identify its own subject.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{ANEMOCHORY}\"\n")),
        "the anemochory row carries both sentences: {body}"
    );
    assert!(
        !body.contains(&format!("        source \"{ANEMO_TAIL}\"\n")),
        "no row is warranted by the dangling sentence alone"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the trust tier.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 4, "four row sources");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n"
        )),
        "the envelope is the article's framing sentence, at the consensus tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{BAROCHORY}\"\n    locator")),
        "not the barochory span as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in [
        "barochory",
        "ballochory",
        "anemochory",
        "epizoochory",
        "gravity",
        "ejected",
        "breeze",
        "flutter",
        "vertebrate",
    ] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no mechanism and no description, but contains {word:?}"
        );
    }
}

#[test]
fn seed_dispersal_mechanism_reverse_binds_the_mechanism_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism($M, seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"ballochory\""),
        "the shipped seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit example is ballochory: {out}"
    );
}

#[test]
fn seed_dispersal_mechanism_abstains_honestly_on_an_untabled_mechanism() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism(hydrochory, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hydrochory is a real dispersal mechanism the source names, but every candidate sentence checked either conflates the mechanism with dispersal distance or is qualified by a following sentence rather than standing alone -- honest abstention, never invented: {out}"
    );
}
