//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/sound-properties.adj`) driven through the built
//! CLI: a native `table` of perceived sound property -> physical quantity
//! (pitch -> frequency, loudness -> amplitude, timbre -> waveform) resolves a
//! binding query recall with the LibreTexts citation, runs backward
//! (physical -> perceived), and abstains on a word the source never maps
//! (`rhythm`) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssp_{tag}_{}", std::process::id()));
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
fn physics_sound_properties_recall_binds_physical_with_citation() {
    let dir = scratch("soundproperties");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/sound-properties.adj");
    std::fs::copy(&src, dir.join("sound-properties.adj")).expect("copy shipped sound-properties.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"sound-properties.adj\"\n\
         ? sound_property(pitch, $P)\n\
         ? sound_property(loudness, $P)\n\
         ? sound_property(timbre, $P)\n\
         ? sound_property($X, frequency)\n\
         ? sound_property(rhythm, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each heard property to the physical quantity the
    // source says it corresponds to.
    assert!(out.contains("\"P\":\"frequency\""), "pitch binds to frequency: {out}");
    assert!(out.contains("\"P\":\"amplitude\""), "loudness binds to amplitude: {out}");
    assert!(out.contains("\"P\":\"waveform\""), "timbre binds to waveform: {out}");
    assert!(
        out.contains("sound_property(pitch, frequency)"),
        "pitch is governing-bound to frequency: {out}"
    );
    assert!(
        out.contains("sound_property(loudness, amplitude)"),
        "loudness is governing-bound to amplitude: {out}"
    );
    assert!(
        out.contains("sound_property(timbre, waveform)"),
        "timbre is governing-bound to waveform: {out}"
    );
    // The relation runs BACKWARD: bind the physical quantity, recall the
    // perceived property heard as it.
    assert!(
        out.contains("\"X\":\"pitch\""),
        "reverse recall binds X=pitch from frequency: {out}"
    );
    // The answer carries the LibreTexts locator + trust tier as its proof.
    assert!(
        out.contains("phys.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // `rhythm` is never mapped by the source — honest abstention.
    assert!(out.contains("\"abstained\":true"), "rhythm abstains: {out}");
}

const SP_PIN: &str = r#""bindings":{"P":"waveform"},"citations":[{"source":"The three subjective quantities of pitch, loudness and timbre are related to laboratory measurements of a sound wave's fundamental frequency, amplitude and waveform, respectively.","locator":"https://phys.libretexts.org/Courses/Joliet_Junior_College/Physics_110_-_by_Conceptual_Objective/12:_Conceptual_Objective_12/12.06:_Pitch_Loudness_and_Timbre","trust":"consensus","corroborations":[{"source":"For the human listener, the amplitude and frequency of a sound roughly correspond to loudness and pitch, respectively.","locator":"https://www.ncbi.nlm.nih.gov/books/NBK11126/""#;

#[test]
fn sound_properties_answer_carries_the_nih_corroboration() {
    let dir = scratch("cite_sp");
    std::fs::copy(
        facts_stdlib().join("physics/sound-properties.adj"),
        dir.join("sound-properties.adj"),
    )
    .expect("copy shipped sound-properties.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"sound-properties.adj\"\n? sound_property(timbre, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // NOTE THE TRUST DIRECTION. The envelope is a LibreTexts span at `trust
    // consensus`; this corroboration is NIH/Purves, which is authoritative.
    // `cites` has no trust field, so a STRONGER source sits under a WEAKER
    // envelope -- that UNDERSTATES the corroboration, which is harmless. The
    // gap only bites the other way (a weaker cite under a stronger envelope
    // would overstate the evidence).
    //
    // The library's header already reasoned this out: the NIH sentence grounds
    // only pitch and loudness, which is exactly why the envelope uses the
    // LibreTexts span that fixes all three rows and why `trust` is honestly
    // `consensus`. The reasoning was right; the evidence just never reached
    // the data. This query binds TIMBRE deliberately -- the one row the NIH
    // sentence does NOT ground -- to show the corroboration rides the table,
    // not the row.
    assert!(
        out.contains(SP_PIN),
        "timbre's answer carries the NIH corroboration verbatim: {out}"
    );
}
