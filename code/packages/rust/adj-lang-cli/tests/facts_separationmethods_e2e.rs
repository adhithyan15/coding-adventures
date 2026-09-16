//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/separation-methods.adj`) driven through the
//! built CLI: a native `table` of separation method → the property/basis its
//! source states it separates by resolves binding-query recalls (forward AND
//! backward) with the source's Chemistry LibreTexts citation, and abstains on a
//! method not in the table (centrifugation) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! FILTRATION span, so a distillation, evaporation or chromatography answer was
//! warranted primarily by a sentence about filtration. The evaporation row
//! carries TWO sentences: "The method drives off the liquid components from the
//! solid components." names no technique on its own.
//!
//! Note the trust tier here is `consensus`, not `authoritative` -- a needle
//! copied from an authoritative sibling would assert the wrong tier.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssep_{tag}_{}", std::process::id()));
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
fn chemistry_separation_basis_recall_binds_basis_with_citation() {
    let dir = scratch("separationmethods");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/separation-methods.adj");
    std::fs::copy(&src, dir.join("separation-methods.adj"))
        .expect("copy shipped separation-methods.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"separation-methods.adj\"\n\
         ? separation_basis(filtration, $Basis)\n\
         ? separation_basis(distillation, $Basis)\n\
         ? separation_basis(chromatography, $Basis)\n\
         ? separation_basis($Method, by_particle_size)\n\
         ? separation_basis(centrifugation, $Basis)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Filtration separates by particle size, distillation by volatility, and
    // chromatography because the components move at different rates — the
    // recalled basis values (forward binds).
    assert!(
        out.contains("\"Basis\":\"by_particle_size\""),
        "filtration → by_particle_size: {out}"
    );
    assert!(
        out.contains("\"Basis\":\"by_volatility\""),
        "distillation → by_volatility: {out}"
    );
    assert!(
        out.contains("\"Basis\":\"by_different_rates\""),
        "chromatography → by_different_rates: {out}"
    );
    // The relation runs BACKWARD: bind the basis `by_particle_size`, recall the
    // method that separates on it.
    assert!(
        out.contains("\"Method\":\"filtration\""),
        "by_particle_size → filtration (reverse recall): {out}"
    );
    // Each answer carries its OWN span as proof, at the `consensus` tier.
    //
    // This was `contains("chem.libretexts.org") && contains("\"trust\":\"consensus\"")`,
    // which any LibreTexts citation satisfies and which says nothing about what
    // the `source` field contains -- a truncated span passes it too. The needles
    // below are whole citations arrays as the serialiser emits them, closing on
    // both the corroborations `]` and the citations `]`.
    //
    // Four of the five queries bind: three forward, plus the reverse bind, which
    // recalls the filtration row and so carries the filtration span again.
    assert_eq!(out.matches("\"citations\":[").count(), 4, "four answers bind: {out}");
    assert!(out.contains(&only_citation(FILTRATION)), "filtration carries its own span: {out}");
    assert!(out.contains(&only_citation(DISTILLATION)), "distillation carries its own span: {out}");
    assert!(
        out.contains(&only_citation(CHROMATOGRAPHY)),
        "chromatography carries its own span: {out}"
    );
    // No query here binds evaporation, so its span must not ride along.
    assert!(!out.contains(EVAPORATION), "the evaporation span reaches no answer here: {out}");
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
    // Centrifugation is not one of the methods this source lists — honest
    // abstention, never a fabricated basis.
    assert!(
        out.contains("\"abstained\":true"),
        "centrifugation abstains: {out}"
    );
}

const LOCATOR: &str = "https://chem.libretexts.org/Courses/Portland_Community_College/CH100:_Everyday_Chemistry/01:_Matter_and_Measurements/1.16:_Methods_for_Separating_Mixtures";
const ENVELOPE: &str = "Mixtures can be separated using a variety of techniques.";
const FILTRATION: &str = "Filtration is a separation method used to separate out pure substances in mixtures comprised of particles\u{2014}some of which are large enough in size to be captured with a porous material. Particle size can vary considerably, given the type of mixture.";
const DISTILLATION: &str = "In simple distillation, a mixture is heated, and the most volatile component vaporizes at the lowest temperature.";
/// Two sentences, contiguous in one paragraph. "The method drives off..."
/// names no technique on its own, so the row carries the sentence before it.
const EVAPORATION: &str = "Evaporation is a technique used to separate out homogeneous mixtures that contain one or more dissolved salts. The method drives off the liquid components from the solid components.";
const EVAP_TAIL: &str = "The method drives off the liquid components from the solid components.";
const CHROMATOGRAPHY: &str = "Chromatography is the separation of a mixture by passing it in solution or suspension, or as a vapor (as in gas chromatography), through a medium in which the components move at different rates.";

/// (method, its basis atom, the span that names both)
fn methods() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("filtration", "by_particle_size", FILTRATION),
        ("distillation", "by_volatility", DISTILLATION),
        ("evaporation", "separates_liquid_from_solid", EVAPORATION),
        ("chromatography", "by_different_rates", CHROMATOGRAPHY),
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
    let adj = std::fs::read_to_string(facts_stdlib().join("chemistry/separation-methods.adj"))
        .expect("read shipped separation-methods.adj");
    adj[adj.find("table separation_basis").expect("table")..].to_string()
}

#[test]
fn every_method_answer_carries_only_the_span_that_names_that_method() {
    // The envelope used to be the FILTRATION span, so a distillation,
    // evaporation or chromatography answer was warranted primarily by a
    // sentence about filtration.
    for (method, basis, span) in methods() {
        let dir = scratch(&format!("method_{method}"));
        let src = facts_stdlib().join("chemistry/separation-methods.adj");
        std::fs::copy(&src, dir.join("separation-methods.adj"))
            .expect("copy shipped separation-methods.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"separation-methods.adj\"\n? separation_basis({method}, $Basis)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {method}: {out}");
        assert!(out.contains(&format!("\"Basis\":\"{basis}\"")), "{method} -> {basis}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{method}: its own span, whole, at the consensus tier, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(method),
            "the span carried by {method} names {method} -- the defect was that it did not"
        );
        for other in [FILTRATION, DISTILLATION, EVAPORATION, CHROMATOGRAPHY] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another method's span must not reach {method}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {method}: {out}");
    }
}

#[test]
fn the_evaporation_row_carries_the_sentence_its_span_points_back_to() {
    // "The method drives off the liquid components from the solid components."
    // names no technique, so a row warranted by it alone would cite a sentence
    // that cannot identify its own subject -- the remedy ../README.md prescribes
    // under "A citation must name its own subject" is to widen the quote.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{EVAPORATION}\"\n")),
        "the evaporation row carries both sentences: {body}"
    );
    assert!(
        !body.contains(&format!("        source \"{EVAP_TAIL}\"\n")),
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
        "the envelope is the page's Summary bullet, at the consensus tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{FILTRATION}\"\n    locator")),
        "not the filtration span as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in [
        "filtration",
        "distillation",
        "evaporation",
        "chromatography",
        "particle",
        "volatile",
        "boiling",
        "rates",
    ] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no method and no basis, but contains {word:?}"
        );
    }
}
