//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-forms.adj`) driven through the built CLI:
//! a native `table` of energy form → defining token resolves a binding query
//! recall with the EIA citation, runs backward (token → form), and abstains on
//! something that is not one of the enumerated forms — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsef_{tag}_{}", std::process::id()));
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
fn physics_energy_forms_recall_binds_token_with_citation() {
    let dir = scratch("energyforms");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/energy-forms.adj");
    std::fs::copy(&src, dir.join("energy-forms.adj")).expect("copy shipped energy-forms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-forms.adj\"\n\
         ? energy_form_token(chemical, $T)\n\
         ? energy_form_token(nuclear, $T)\n\
         ? energy_form_token(electrical, $T)\n\
         ? energy_form_token($F, heat)\n\
         ? energy_form_token(magnetic, $T)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each form to the token EIA uses to define it.
    assert!(out.contains("\"T\":\"bonds\""), "chemical binds to bonds: {out}");
    assert!(
        out.contains("energy_form_token(chemical, bonds)"),
        "chemical is governing-bound to bonds: {out}"
    );
    assert!(
        out.contains("energy_form_token(nuclear, nucleus)"),
        "nuclear is governing-bound to nucleus: {out}"
    );
    assert!(
        out.contains("energy_form_token(electrical, electrons)"),
        "electrical is governing-bound to electrons: {out}"
    );
    // The relation runs BACKWARD: bind the token, recall the form.
    assert!(
        out.contains("energy_form_token(thermal, heat)"),
        "reverse recall binds F=thermal from heat: {out}"
    );
    // The answer carries the EIA locator + trust tier as its proof.
    assert!(
        out.contains(EIA_LOCATOR) && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Magnetic energy is NOT one of the enumerated forms — honest abstention.
    assert!(out.contains("\"abstained\":true"), "magnetic abstains: {out}");
}

const EIA_LOCATOR: &str =
    "https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php";

/// Assert one row carries its own defining sentence, in a program returning
/// only that row. Each form names exactly one row, so a one-answer query is
/// available here.
fn assert_form(tag: &str, form: &str, token: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("physics/energy-forms.adj"),
        dir.join("energy-forms.adj"),
    )
    .expect("copy shipped energy-forms.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"energy-forms.adj\"\n? energy_form_token({form}, $T)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {form}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"T\":\"{token}\"")),
        "{form} binds {token}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{EIA_LOCATOR}\",\"trust\":\"authoritative\""
        )),
        "{form} carries its own defining sentence: {out}"
    );
}

/// #14986: the envelope was the CHEMICAL sentence, so a recall of `electrical`
/// came back proved by *"Chemical energy is energy stored in the bonds of
/// atoms and molecules."*
///
/// SAME PAGE, DIFFERENT ANSWER FROM ITS SIBLING. `energy-form-family.adj`
/// cites this page too and is NOT convertible — there the potential/kinetic
/// grouping lives in a section heading and no sentence assigns a form to a
/// family. This table asks what each form IS, which the page states in prose.
#[test]
fn every_form_row_carries_its_own_defining_sentence() {
    assert_form(
        "efchem", "chemical", "bonds",
        "Chemical energy is energy stored in the bonds of atoms and molecules.",
    );
    assert_form(
        "efmech", "mechanical", "tension",
        "Mechanical energy is energy stored in objects by tension.",
    );
    assert_form(
        "efnuc", "nuclear", "nucleus",
        "Nuclear energy is energy stored in the nucleus of an atom—the energy that holds the nucleus together.",
    );
    assert_form(
        "efgrav", "gravitational", "height",
        "Gravitational energy is energy stored in an object's height.",
    );
    assert_form(
        "efrad", "radiant", "electromagnetic",
        "Radiant energy is electromagnetic energy that travels in transverse waves.",
    );
    assert_form(
        "efmotion", "motion", "moving",
        "Motion energy is energy stored in moving objects.",
    );
    assert_form(
        "efelec", "electrical", "electrons",
        "Electrical energy is delivered by tiny, charged particles, called electrons, that typically move through a wire.",
    );
}

/// The page writes *"Thermal energy, or heat, is…"*, not *"Thermal energy
/// is…"*. A probe that required the plain head reported this row as having no
/// defining sentence at all — the probe's prefix test, not the page. Same
/// shape as `gravitropism` and `electrotropism` in `plant-tropisms` (#15176),
/// and pinned here so nobody "normalises" the head the page does not use.
#[test]
fn the_thermal_row_keeps_the_pages_variant_head() {
    assert_form(
        "eftherm", "thermal", "heat",
        "Thermal energy, or heat, is the energy that comes from atoms and molecules moving in a substance.",
    );
}

/// The envelope is the framing sentence. It warrants no row — and its WORDING
/// is pinned against the shipped file, not merely disclosed as unreachable,
/// the way `plant-tropisms` (#15176) closed that gap.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("efenvelope");
    std::fs::copy(
        facts_stdlib().join("physics/energy-forms.adj"),
        dir.join("energy-forms.adj"),
    )
    .expect("copy shipped energy-forms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-forms.adj\"\n? energy_form_token($F, $T)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        8,
        "all eight rows answer: {out}"
    );
    assert!(
        !out.contains("Many forms of energy exist"),
        "the framing span warrants no row: {out}"
    );
    // THE PIN RUNS TO THE TIER, not to `\n    locator`. The shorter form
    // shipped here and in two sibling entries: it asserts that a locator
    // follows the envelope source, never what that locator is.
    //
    // MEASURED, and it corrected the reason for writing this. The short pin
    // does NOT let a repointed envelope through this table: no row here
    // overrides `locator` or `trust`, so both are inherited by all eight rows
    // and reach every answer, where the per-row citation assertions already
    // pin them. The hole WAS real in `brain-parts` (#15181), where all fifteen
    // rows carry their own locator and the envelope's reaches nothing — review
    // found it there and it is pinned this same way there now, on main.
    //
    // What this pin defends is the shape this table is moving toward. Give
    // every row its own locator — a refactor with identical output — and then
    // repoint the envelope: the short pin SURVIVES that, this one KILLS it.
    let adj = std::fs::read_to_string(
        facts_stdlib().join("physics/energy-forms.adj"),
    )
    .expect("read shipped energy-forms.adj");
    assert!(
        adj.contains(
            "    source \"Many forms of energy exist, but energy is either potential energy or kinetic energy.\"\n    locator \"https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php\"\n    trust authoritative"
        ),
        "the envelope carries the page's framing sentence, verbatim"
    );
}
