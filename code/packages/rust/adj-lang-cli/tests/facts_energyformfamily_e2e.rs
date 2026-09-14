//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-form-family.adj`) driven through the
//! built CLI: a native `table` naming which of the source's two families
//! (potential/kinetic) an energy form belongs to -- a sibling to the
//! already-shipped `energy-forms.adj` (which only carries each form's
//! defining TOKEN), decoding the SAME EIA page's own two-heading structure,
//! re-verified live via WebFetch this cycle. Resolves binding-query recall
//! (both directions, including a genuine one-to-many reverse recall) with
//! the source's citation, and abstains on a form (sound) the source's page
//! names but `energy-forms.adj` never tabled -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_energyformfamily_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/energy-form-family.adj");
    std::fs::copy(&src, dir.join("energy-form-family.adj"))
        .expect("copy shipped energy-form-family.adj");
}

#[test]
fn energy_form_family_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family(chemical, $Family)\n\
         ? energy_form_family(mechanical, $Family)\n\
         ? energy_form_family(nuclear, $Family)\n\
         ? energy_form_family(gravitational, $Family)\n\
         ? energy_form_family(radiant, $Family)\n\
         ? energy_form_family(thermal, $Family)\n\
         ? energy_form_family(motion, $Family)\n\
         ? energy_form_family(electrical, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for (form, family) in [
        ("chemical", "potential"),
        ("mechanical", "potential"),
        ("nuclear", "potential"),
        ("gravitational", "potential"),
        ("radiant", "kinetic"),
        ("thermal", "kinetic"),
        ("motion", "kinetic"),
        ("electrical", "kinetic"),
    ] {
        let term = format!("\"term\":\"energy_form_family({form}, {family})\"");
        assert!(out.contains(&term), "{form} should be {family}: {out}");
    }
    // ONE CONTIGUOUS SPAN, not two loose needles. `contains("eia.gov") &&
    // contains("\"trust\":\"authoritative\"")` was here -- the #15139 shape,
    // two halves satisfiable by different parts of the output, and the third
    // table in this cascade found carrying it.
    assert!(
        out.contains(
            "\"source\":\"Many forms of energy exist, but energy is either potential energy or kinetic energy.\",\"locator\":\"https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php\",\"trust\":\"authoritative\""
        ),
        "carries the EIA citation, whole and contiguous: {out}"
    );
}

#[test]
fn energy_form_family_recalls_backward_all_four_kinetic_forms() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family($Form, kinetic)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for form in ["radiant", "thermal", "motion", "electrical"] {
        let term = format!("\"term\":\"energy_form_family({form}, kinetic)\"");
        assert!(out.contains(&term), "kinetic should include {form}: {out}");
    }
}

#[test]
fn energy_form_family_abstains_honestly_on_sound() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family(sound, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sound is not one of energy-forms.adj's eight tabled forms -- honest abstention: {out}"
    );
}

const LOCATOR: &str = "https://www.eia.gov/energyexplained/what-is-energy/forms-of-energy.php";
const ENVELOPE: &str = "Many forms of energy exist, but energy is either potential energy or kinetic energy.";

const FORMS: [(&str, &str, &str); 8] = [
        ("chemical", "potential", "Chemical energy is energy stored in the bonds of atoms and molecules."),
        ("mechanical", "potential", "Mechanical energy is energy stored in objects by tension."),
        ("nuclear", "potential", "Nuclear energy is energy stored in the nucleus of an atom—the energy that holds the nucleus together."),
        ("gravitational", "potential", "Gravitational energy is energy stored in an object's height."),
        ("radiant", "kinetic", "Radiant energy is electromagnetic energy that travels in transverse waves."),
        ("thermal", "kinetic", "Thermal energy, or heat, is the energy that comes from atoms and molecules moving in a substance."),
        ("motion", "kinetic", "Motion energy is energy stored in moving objects."),
        ("electrical", "kinetic", "Electrical energy is delivered by tiny, charged particles, called electrons, that typically move through a wire."),
];

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"energy-form-family.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn every_form_is_corroborated_by_the_pages_definition_of_it() {
    // WHAT THIS CHANGE ADDED. Before it, seven of the eight rows carried no
    // span of their own at all -- a query about `chemical` came back with
    // only the two-family sentence.
    for (form, family, span) in FORMS {
        let out = ask(&format!("corr_{form}"), &format!("energy_form_family({form}, $F)"));
        // ONE ANSWER, ASSERTED, so the negative arm below is about THIS
        // answer and cannot be masked by a sibling's citation (#15164).
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {form}: {out}"
        );
        assert!(
            out.contains(&format!("\"F\":\"{family}\"")),
            "{form} still binds {family}: {out}"
        );
        // A CORROBORATION, NOT A SOURCE, pinned as one contiguous run: the
        // envelope stays this row's PRIMARY warrant, because the definition
        // says nothing about the family.
        assert!(
            out.contains(&format!(
                "\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{{\"source\":\"{span}\""
            )),
            "{form}: the envelope stays primary and its own definition corroborates: {out}"
        );
        // NEGATIVE ARM: no other form's definition reaches this answer.
        for (other, _, other_span) in FORMS {
            if other != form {
                assert!(
                    !out.contains(other_span),
                    "the {other} definition must not reach the {form} answer: {out}"
                );
            }
        }
    }
}

#[test]
fn no_row_carries_a_source_because_no_span_states_a_family() {
    // THE ZERO IS THE INSTRUMENT. This table is deliberately NOT converted to
    // per-row `source` (#14986): the page assigns `potential`/`kinetic` by
    // which HEADING a form is listed under, and measured against the cited
    // page, seven of the eight forms have NO sentence naming both the form
    // and a family. A row `source` appearing here later would assert a
    // warrant none of these spans carries.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/energy-form-family.adj"))
        .expect("read shipped energy-form-family.adj");
    let body = &adj[adj.find("table energy_form_family").expect("table")..];

    assert_eq!(body.matches("\n    row (").count(), 8, "eight rows");
    assert_eq!(
        body.matches("\n        cites \"").count(),
        8,
        "each row carries exactly one corroboration"
    );
    assert_eq!(
        body.matches("\n        source \"").count(),
        0,
        "NO row carries a source -- no span on the page states a family"
    );

    // SEMANTIC, NOT ONLY A TEXT COUNT. An indentation match alone would miss
    // a row `source` written at any other depth, so the envelope is asserted
    // to remain the primary warrant on a real answer too.
    let out = ask("nosource", "energy_form_family(nuclear, $F)");
    assert!(
        out.contains(&format!("\"source\":\"{ENVELOPE}\"")),
        "the envelope is still the primary source on an answer: {out}"
    );
}

#[test]
fn no_row_span_states_a_family_which_is_why_they_are_cites() {
    // THE PROPERTY THAT MAKES THESE HONEST. A definition says what a form IS;
    // it does not say whether it is potential or kinetic. Asserted rather
    // than explained: if a future edit swaps in a span that DOES state a
    // family, this table should be reconsidered for per-row `source`, and
    // this test says so by failing rather than quietly allowing it.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/energy-form-family.adj"))
        .expect("read shipped energy-form-family.adj");
    let body = &adj[adj.find("table energy_form_family").expect("table")..];

    let spans: Vec<&str> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("cites \""))
        .map(|r| r.split('"').next().expect("span"))
        .collect();
    assert_eq!(spans.len(), 8, "eight row spans read: {spans:?}");
    for span in &spans {
        let low = span.to_lowercase();
        assert!(
            !low.contains("potential") && !low.contains("kinetic"),
            "a row span must not state a family -- if this fires, reconsider \
             whether the table can take a per-row `source`: {span:?}"
        );
    }
    // And they are all distinct, so a single-row truncation cannot hide
    // behind a sibling's copy.
    let mut uniq = spans.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), 8, "all eight definitions are distinct: {spans:?}");
}

#[test]
fn the_envelope_is_the_two_family_sentence_and_names_no_form() {
    // The envelope is unchanged by this change and was already the right
    // span. Pinned, and checked against the keys READ FROM THE TABLE so a
    // ninth row re-checks it automatically.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/energy-form-family.adj"))
        .expect("read shipped energy-form-family.adj");
    let body = &adj[adj.find("table energy_form_family").expect("table")..];

    let table_sources: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect();
    assert_eq!(table_sources.len(), 1, "exactly one table-level source");
    let envelope = table_sources[0]
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char);
    assert_eq!(envelope, ENVELOPE, "the two-family classification sentence");

    let folded = envelope.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim().to_lowercase();
            keys += 1;
            assert!(
                !folded.contains(&key),
                "the envelope must name no form, but names {key:?}"
            );
        }
    }
    assert_eq!(keys, 8, "all eight keys were actually checked");
    assert!(
        adj.contains("    columns form, family"),
        "the shipped column names are unchanged"
    );
}
