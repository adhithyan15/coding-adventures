//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/plant-tropisms.adj`) driven through the built CLI:
//! a native `table` of the classic plant tropisms → the environmental stimulus
//! each responds to resolves binding-query recalls (forward AND backward) with
//! the source's Wikipedia "Tropism" citation, and abstains on a word that is not
//! one of these tropisms (`sound`) — 0 model calls.

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

#[test]
fn biology_plant_tropisms_recall_binds_stimulus_with_citation() {
    let dir = scratch("planttropisms");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/plant-tropisms.adj");
    std::fs::copy(&src, dir.join("plant-tropisms.adj")).expect("copy shipped plant-tropisms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-tropisms.adj\"\n\
         ? tropism_stimulus(phototropism, $Stimulus)\n\
         ? tropism_stimulus(thigmotropism, $Stimulus)\n\
         ? tropism_stimulus(hydrotropism, $Stimulus)\n\
         ? tropism_stimulus($Tropism, gravity)\n\
         ? tropism_stimulus(sound, $Stimulus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A phototropism tracks light, a thigmotropism tracks touch, a hydrotropism
    // tracks water — the recalled stimuli (forward binds).
    assert!(
        out.contains("\"Stimulus\":\"light\""),
        "phototropism → light: {out}"
    );
    assert!(
        out.contains("\"Stimulus\":\"touch\""),
        "thigmotropism → touch: {out}"
    );
    assert!(
        out.contains("\"Stimulus\":\"water\""),
        "hydrotropism → water: {out}"
    );
    // The relation runs BACKWARD: bind the stimulus `gravity`, recall its tropism.
    assert!(
        out.contains("\"Tropism\":\"gravitropism\""),
        "gravity → gravitropism (reverse recall): {out}"
    );
    // The answer carries the Wikipedia "Tropism" citation as its proof, at the
    // `consensus` trust tier for a teaching-quality encyclopedia summary.
    assert!(
        out.contains("en.wikipedia.org/wiki/Tropism") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // `sound` is not one of these tropisms — honest abstention, never a
    // fabricated stimulus.
    assert!(out.contains("\"abstained\":true"), "sound abstains: {out}");
}

#[test]
fn biology_plant_tropisms_extension_recalls_the_newly_added_rows() {
    let dir = scratch("planttropismsext");
    let src = facts_stdlib().join("biology/plant-tropisms.adj");
    std::fs::copy(&src, dir.join("plant-tropisms.adj")).expect("copy shipped plant-tropisms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-tropisms.adj\"\n\
         ? tropism_stimulus(aerotropism, $Stimulus)\n\
         ? tropism_stimulus($Tropism, temperature)\n\
         ? tropism_stimulus(inotropism, $Stimulus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // 7 tropisms were added this cycle from the SAME already-cited Wikipedia
    // "Tropism" article's own "Types of tropism" list.
    assert!(
        out.contains("\"Stimulus\":\"wind\""),
        "aerotropism → wind: {out}"
    );
    assert!(
        out.contains("\"Tropism\":\"thermotropism\""),
        "temperature → thermotropism (reverse recall): {out}"
    );
    // inotropism is a real term on the SAME Wikipedia page, but names a
    // MUSCLE's contraction response to drugs, not a plant tropism -- honest
    // abstention, never a fabricated stimulus.
    assert!(
        out.contains("\"abstained\":true"),
        "inotropism abstains (wrong domain): {out}"
    );
}

const TROPISM_LOCATOR: &str = "https://en.wikipedia.org/wiki/Tropism";

/// Assert one row carries its own definition line, in a program returning only
/// that row. Each tropism names exactly one row, so a single-answer query is
/// available here — unlike `speleothem-substrate` (#15175), where two rows
/// share a substrate and the needle had to span from binding into citation.
fn assert_tropism(tag: &str, tropism: &str, stimulus: &str, span: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/plant-tropisms.adj"),
        dir.join("plant-tropisms.adj"),
    )
    .expect("copy shipped plant-tropisms.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"plant-tropisms.adj\"\n? tropism_stimulus({tropism}, $S)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {tropism}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"S\":\"{stimulus}\"")),
        "{tropism} binds {stimulus}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{TROPISM_LOCATOR}\",\"trust\":\"consensus\""
        )),
        "{tropism} carries its own definition line: {out}"
    );
    out
}

/// #14986: the envelope was the PHOTOTROPISM definition, so a recall of
/// `traumatotropism` came back warranted by a sentence about light. All twelve
/// rows are pinned individually — the same table shape where, in
/// `skeleton-bones` (#15171), testing one row per shared span left five rows
/// pinned by nothing.
#[test]
fn every_tropism_row_carries_its_own_definition_line() {
    assert_tropism(
        "trphoto", "phototropism", "light",
        "Phototropism: movement or growth in response to lights or colors of light",
    );
    assert_tropism(
        "trthigmo", "thigmotropism", "touch",
        "Thigmotropism: movement or growth in response to touch or contact",
    );
    assert_tropism(
        "trhydro", "hydrotropism", "water",
        "Hydrotropism: movement or growth in response to water; in plants, the root cap senses differences in water moisture in the soil, and signals cellular changes that cause the root to curve towards the area of higher moisture",
    );
    assert_tropism(
        "traero", "aerotropism", "wind",
        "Aerotropism: the growth of plants towards or away from a source of wind",
    );
    assert_tropism(
        "trhelio", "heliotropism", "sun_direction",
        "Heliotropism: the diurnal motion or seasonal motion of plant parts in response to the direction of the Sun, (e.g. the sunflower)",
    );
    assert_tropism(
        "trmagneto", "magnetotropism", "magnetic_fields",
        "Magnetotropism: movement or growth in response to magnetic fields",
    );
    assert_tropism(
        "trseleno", "selenotropism", "moon_direction",
        "Selenotropism: motion of plant parts in response to the direction of the Moon",
    );
    assert_tropism(
        "trthermo", "thermotropism", "temperature",
        "Thermotropism: movement or growth in response to temperature",
    );
    assert_tropism(
        "trtrauma", "traumatotropism", "wounding",
        "Traumatotropism: orientation deviation after suffering a wounding",
    );
}

/// Two rows whose definition line does NOT open with a bare `Tropism:` head.
/// A probe that required the bare form reported both as having no definition
/// line at all, which was the probe's prefix test and not the page.
#[test]
fn the_two_variant_heads_carry_the_pages_own_wording() {
    assert_tropism(
        "trgravi", "gravitropism", "gravity",
        "Gravitropism (sometimes referred to as geotropism): is movement or growth in response to gravity",
    );
    assert_tropism(
        "trelectro", "electrotropism", "electric_field",
        "Electrotropism, or galvanotropism: the movement or growth in response to an electric field",
    );
}

/// Every span stops before the page's reference markers — `chemicals[8]`
/// becomes `chemicals`. That leaves a verbatim PREFIX of the page's own line:
/// nothing reworded, and what is dropped is a footnote marker, never content.
/// Pinned so nobody "restores" a bracket that is not part of the sentence, and
/// so nobody trims further.
#[test]
fn spans_stop_before_the_pages_reference_markers() {
    // (A `!out.contains("...chemicals[8]")` line stood here, and the binding
    // it read went with it. It could never fire: the exact
    // `"source":"...chemicals","locator":...` needle inside `assert_tropism`
    // already excludes the bracketed form, and there is one citation in the
    // output. An assertion never observed to fire is decoration.
    //
    // Leaving `let out = ...` behind broke CI, which denies warnings where a
    // local `cargo test` does not. Deleting an assertion means deleting what
    // fed it.)
    assert_tropism(
        "trchemo", "chemotropism", "chemicals",
        "Chemotropism: the movement or growth in response to chemicals",
    );
}

/// The envelope is the page's definition of a tropism. It warrants no row; its
/// wording is unreachable from any answer once every row overrides `source`,
/// which is disclosed rather than implied.
#[test]
fn the_framing_envelope_never_reaches_an_answer() {
    let dir = scratch("trenvelope");
    std::fs::copy(
        facts_stdlib().join("biology/plant-tropisms.adj"),
        dir.join("plant-tropisms.adj"),
    )
    .expect("copy shipped plant-tropisms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-tropisms.adj\"\n? tropism_stimulus($T, $S)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        12,
        "all twelve rows answer: {out}"
    );
    assert!(
        !out.contains("In biology, a tropism is a phenomenon"),
        "the framing span warrants no row: {out}"
    );

    // AND PIN ITS WORDING. Every sibling entry in this cascade disclosed the
    // envelope's text as unpinnable, because no answer carries it. That is
    // true of the OUTPUT and need not be true of the FILE: read the shipped
    // `.adj` and assert the span literally, so a drift from the page is a
    // test failure rather than a disclosed gap.
    let adj = std::fs::read_to_string(
        facts_stdlib().join("biology/plant-tropisms.adj"),
    )
    .expect("read shipped plant-tropisms.adj");
    assert!(
        adj.contains(
            "    source \"In biology, a tropism is a phenomenon indicating the growth or turning movement of an organism, usually a plant, in response to an environmental stimulus.\"
    locator"
        ),
        "the envelope carries the page's definition of a tropism, verbatim"
    );
}
