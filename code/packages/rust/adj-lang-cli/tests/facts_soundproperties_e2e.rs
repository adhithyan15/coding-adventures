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
