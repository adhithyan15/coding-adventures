//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/em-spectrum.adj`) driven through the built CLI:
//! a native `table` of the seven electromagnetic-spectrum bands → a
//! representative everyday use resolves binding-query recalls (forward AND
//! backward) with the source's NASA "Imagine the Universe!" citation, and
//! abstains on a word that is not one of the seven EM bands (sound, a mechanical
//! wave) — 0 model calls.

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
fn physics_em_spectrum_recall_binds_use_with_citation() {
    let dir = scratch("emspectrum");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/em-spectrum.adj");
    std::fs::copy(&src, dir.join("em-spectrum.adj")).expect("copy shipped em-spectrum.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"em-spectrum.adj\"\n\
         ? band_use(radio, $Application)\n\
         ? band_use(microwave, $Application)\n\
         ? band_use(x_ray, $Application)\n\
         ? band_use($Band, night_vision)\n\
         ? band_use(sound, $Application)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Radio waves come from radio stations, microwaves cook, X-rays image teeth —
    // the recalled everyday uses (forward binds).
    assert!(
        out.contains("\"Application\":\"radio_stations\""),
        "radio → radio_stations: {out}"
    );
    assert!(
        out.contains("\"Application\":\"cooking\""),
        "microwave → cooking: {out}"
    );
    assert!(
        out.contains("\"Application\":\"teeth\""),
        "x_ray → teeth: {out}"
    );
    // The relation runs BACKWARD: bind the use `night_vision`, recall its band.
    assert!(
        out.contains("\"Band\":\"infrared\""),
        "night_vision → infrared (reverse recall): {out}"
    );
    // The answer carries the NASA Imagine the Universe! citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("imagine.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Sound is a mechanical wave, not one of the seven EM bands — honest
    // abstention, never a fabricated use.
    assert!(out.contains("\"abstained\":true"), "sound abstains: {out}");
}
