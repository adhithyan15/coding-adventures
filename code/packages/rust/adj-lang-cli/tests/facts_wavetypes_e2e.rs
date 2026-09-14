//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/wave-types.adj`) driven through the built CLI:
//! a native `table` of named wave → wave family (mechanical / electromagnetic)
//! resolves a binding query recall with the NASA citation, runs backward
//! (family → wave), and abstains on a wave the source never classifies
//! (a seismic wave) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factswt_{tag}_{}", std::process::id()));
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
fn physics_wave_types_recall_binds_family_with_citation() {
    let dir = scratch("wavetypes");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/wave-types.adj");
    std::fs::copy(&src, dir.join("wave-types.adj")).expect("copy shipped wave-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-types.adj\"\n\
         ? wave_family(sound, $F)\n\
         ? wave_family(radio, $F)\n\
         ? wave_family(gamma, $F)\n\
         ? wave_family($W, electromagnetic)\n\
         ? wave_family(seismic, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each named wave to the family NASA sorts it into.
    assert!(out.contains("\"F\":\"mechanical\""), "sound binds to mechanical: {out}");
    assert!(
        out.contains("\"F\":\"electromagnetic\""),
        "radio binds to electromagnetic: {out}"
    );
    assert!(
        out.contains("wave_family(sound, mechanical)"),
        "sound is governing-bound to mechanical: {out}"
    );
    assert!(
        out.contains("wave_family(radio, electromagnetic)"),
        "radio is governing-bound to electromagnetic: {out}"
    );
    assert!(
        out.contains("wave_family(gamma, electromagnetic)"),
        "gamma is governing-bound to electromagnetic: {out}"
    );
    // The relation runs BACKWARD: bind the family, recall a wave that belongs to it.
    assert!(
        out.contains("\"W\":\"gamma\""),
        "reverse recall binds W=gamma from electromagnetic: {out}"
    );
    // THIS ASSERTION COULD NOT HAVE CAUGHT THE DEFECT. Both sentences on this
    // table are on the same NASA page, so a hostname needle held just as well
    // when all ten rows carried the MECHANICAL sentence — which names no
    // electromagnetic band at all. Kept because the tier still matters; the
    // per-row pins below are what bind a row to the sentence that names it.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries a citation on the cited page, at the authoritative tier: {out}"
    );
    // A seismic wave is never sorted into a family by the source — honest abstention.
    assert!(out.contains("\"abstained\":true"), "seismic abstains: {out}");
}

#[test]
fn physics_wave_types_extension_recalls_newly_added_electromagnetic_waves() {
    let dir = scratch("wavetypes_ext");
    let src = facts_stdlib().join("physics/wave-types.adj");
    std::fs::copy(&src, dir.join("wave-types.adj")).expect("copy shipped wave-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-types.adj\"\n\
         ? wave_family(ultraviolet, $F)\n\
         ? wave_family(infrared, $F)\n\
         ? wave_family(microwave, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // ultraviolet, infrared, and microwave were sitting in the header's own
    // already-quoted electromagnetic sentence but never turned into rows
    // until this cycle -- all three now bind to electromagnetic.
    assert!(
        out.contains("wave_family(ultraviolet, electromagnetic)"),
        "ultraviolet → electromagnetic (added this cycle): {out}"
    );
    assert!(
        out.contains("wave_family(infrared, electromagnetic)"),
        "infrared → electromagnetic (added this cycle): {out}"
    );
    assert!(
        out.contains("wave_family(microwave, electromagnetic)"),
        "microwave → electromagnetic (added this cycle): {out}"
    );
}

const PAGE: &str = "https://science.nasa.gov/asset/webb/electromagnetic-wave-light-wave-vs-mechanical-wave/";
const MECH: &str = "Water waves, sound waves, and waves on a rope are all examples of mechanical waves.";
const EM: &str = "Light waves include all forms of electromagnetic radiation: gamma rays, X-rays, ultraviolet light, visible light, infrared light, microwaves, and radio waves.";

/// Assert one row's warrant, binding the WAVE so exactly one row answers.
fn assert_wave(tag: &str, wave: &str, family: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("physics/wave-types.adj"),
        dir.join("wave-types.adj"),
    )
    .expect("copy shipped wave-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"wave-types.adj\"\n? wave_family({wave}, $F)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {wave}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"F\":\"{family}\"")),
        "{wave} binds {family}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{PAGE}\",\"trust\":\"authoritative\""
        )),
        "{wave} is warranted by the sentence that names it: {out}"
    );
}

/// #14986. The MECHANICAL membership sentence was this table's `source` — the
/// field that carries the tier — for all TEN rows, so a recall of `gamma` came
/// back proved by *"Water waves, sound waves, and waves on a rope are all
/// examples of mechanical waves."* Seven of the ten rows were in that position.
///
/// Both sentences are on the SAME page, so the locator was never wrong and a
/// hostname assertion could never have caught it — only the span was.
#[test]
fn every_wave_carries_the_sentence_that_names_it() {
    for (tag, wave) in [("wvwater", "water"), ("wvsound", "sound"), ("wvrope", "rope")] {
        assert_wave(tag, wave, "mechanical", MECH);
    }
    for (tag, wave) in [
        ("wvgamma", "gamma"),
        ("wvxray", "xray"),
        ("wvuv", "ultraviolet"),
        ("wvvis", "visible_light"),
        ("wvir", "infrared"),
        ("wvmicro", "microwave"),
        ("wvradio", "radio"),
    ] {
        assert_wave(tag, wave, "electromagnetic", EM);
    }
}

/// #15193. The sharing structure, asserted against the SHIPPED FILE rather
/// than described in a comment: two groups, three rows and seven rows, and no
/// row with a sentence of its own.
#[test]
fn the_shared_spans_are_exactly_the_declared_ones() {
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/wave-types.adj"))
        .expect("read shipped wave-types.adj");
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut key: Option<String> = None;
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix("    row (") {
            key = rest.split(')').next().map(|k| {
                k.split(',').map(|p| p.trim()).collect::<Vec<_>>().join(", ")
            });
        } else if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            pairs.push((
                key.clone().expect("a row precedes every source"),
                rest.trim_end_matches(0x22 as char).to_string(),
            ));
        }
    }
    assert_eq!(pairs.len(), 10, "ten rows carry their own source: {pairs:?}");
    let mech: Vec<&String> = pairs.iter().filter(|(_, s)| s == MECH).map(|(k, _)| k).collect();
    let em: Vec<&String> = pairs.iter().filter(|(_, s)| s == EM).map(|(k, _)| k).collect();
    assert_eq!(mech.len(), 3, "three mechanical rows: {mech:?}");
    assert_eq!(em.len(), 7, "seven electromagnetic rows: {em:?}");
    assert_eq!(
        mech.len() + em.len(),
        pairs.len(),
        "and no row carries a third sentence: {pairs:?}"
    );
    assert!(
        adj.contains("    columns wave, family"),
        "the shipped column names are unchanged"
    );
}

/// The envelope is the mechanical-wave DEFINITION, and the file discloses that
/// choice rather than dressing it up as neutral framing: this page frames each
/// FAMILY separately and has no sentence framing the table as a whole. What the
/// framing slot requires is that the envelope not MIS-WARRANT a row, and this
/// sentence names no specific wave.
#[test]
fn the_envelope_definition_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("wvenvelope");
    std::fs::copy(
        facts_stdlib().join("physics/wave-types.adj"),
        dir.join("wave-types.adj"),
    )
    .expect("copy shipped wave-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"wave-types.adj\"\n? wave_family($W, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        10,
        "all ten rows answer: {out}"
    );
    assert!(
        !out.contains("also called a matter wave"),
        "the envelope definition warrants no row: {out}"
    );
    // The mechanical sentence warrants exactly THREE rows where it used to be
    // the `source` on all ten. Twice each: citations and steps.
    assert_eq!(
        out.matches(MECH).count(),
        6,
        "the mechanical sentence warrants its three rows and no others: {out}"
    );
    assert_eq!(
        out.matches(EM).count(),
        14,
        "the electromagnetic sentence warrants its seven rows: {out}"
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/wave-types.adj"))
        .expect("read shipped wave-types.adj");
    assert!(
        adj.contains(
            "    source \"A mechanical wave, also called a matter wave, is a propagation of energy through matter.\"\n    locator \"https://science.nasa.gov/asset/webb/electromagnetic-wave-light-wave-vs-mechanical-wave/\"\n    trust authoritative"
        ),
        "the envelope carries the page's definition sentence, verbatim"
    );
}
