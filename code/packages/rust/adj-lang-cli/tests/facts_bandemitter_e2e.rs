//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/band-emitter.adj`) driven through the built
//! CLI: a native `table` recording, for three of the seven electromagnetic-
//! spectrum bands already tabled in `em-spectrum.adj`, WHO or WHAT the
//! same already-cited NASA sentence states emits that band -- a sibling
//! decoding the emitter half of three already-verified quotes. Resolves
//! forward and backward recall queries with the source's citation, plus
//! honest abstention on a band whose cited span states a use but no
//! emitter -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_bandemitter_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/band-emitter.adj");
    std::fs::copy(&src, dir.join("band-emitter.adj"))
        .expect("copy shipped band-emitter.adj");
}

#[test]
fn band_emitter_recalls_ultraviolet_emitter_with_citation() {
    let dir = scratch("uv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter(ultraviolet, $Emitter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"band_emitter(ultraviolet, sun)\""),
        "ultraviolet should recall its cited emitter: {out}"
    );
    assert!(
        out.contains("imagine.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn band_emitter_backward_recalls_ultraviolet_for_sun() {
    let dir = scratch("sun");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter($Band, sun)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"band_emitter(ultraviolet, sun)\""),
        "ultraviolet should be the only recalled sun-emitted band: {out}"
    );
    assert!(
        !out.contains("band_emitter(infrared, sun)"),
        "infrared's cited emitter is skin and heat objects, not the sun: {out}"
    );
}

#[test]
fn band_emitter_abstains_on_radio() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter(radio, $EmitterRadio)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "radio's cited span states a use (radio stations), not an emitter -- honest abstention expected: {out}"
    );
}
