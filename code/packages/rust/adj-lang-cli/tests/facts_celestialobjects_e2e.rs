//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/celestial-objects.adj`) driven through the built
//! CLI: a native `table` of the basic celestial-object types → a short defining
//! property resolves binding-query recalls (forward AND backward) with the
//! source's NASA citation, and abstains on a word that is not one of the basic
//! celestial-object types (a cloud) — 0 model calls.

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
fn astronomy_celestial_objects_recall_binds_property_with_citation() {
    let dir = scratch("celestialobjects");
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/celestial-objects.adj");
    std::fs::copy(&src, dir.join("celestial-objects.adj"))
        .expect("copy shipped celestial-objects.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-objects.adj\"\n\
         ? celestial_property(star, $Property)\n\
         ? celestial_property(planet, $Property)\n\
         ? celestial_property(asteroid, $Property)\n\
         ? celestial_property($Object, orbits_planet)\n\
         ? celestial_property(cloud, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A star gives off its own light, a planet revolves around a star, an
    // asteroid is rocky — the recalled properties (forward binds).
    assert!(
        out.contains("\"Property\":\"gives_off_light\""),
        "star → gives_off_light: {out}"
    );
    assert!(
        out.contains("\"Property\":\"revolves_around_star\""),
        "planet → revolves_around_star: {out}"
    );
    assert!(
        out.contains("\"Property\":\"rocky\""),
        "asteroid → rocky: {out}"
    );
    // The relation runs BACKWARD: bind the property `orbits_planet`, recall
    // which object it defines.
    assert!(
        out.contains("\"Object\":\"moon\""),
        "orbits_planet → moon (reverse recall): {out}"
    );
    // The answer carries the NASA citation as its proof, at the `authoritative`
    // trust tier for a primary U.S. government source.
    assert!(
        out.contains("starchild.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A cloud is not one of the basic celestial-object types — honest
    // abstention, never a fabricated property.
    assert!(out.contains("\"abstained\":true"), "cloud abstains: {out}");
}

const MOON_PIN: &str = r#""bindings":{"Object":"moon"},"citations":[{"source":"A star is a big ball of gas which gives off both heat and light.","locator":"https://starchild.gsfc.nasa.gov/docs/StarChild/universe_level1/stars.html","trust":"authoritative","corroborations":[{"source":"A planet is a large space object which revolves around a star.","locator":"https://starchild.gsfc.nasa.gov/docs/StarChild/solar_system_level1/planets.html"},{"source":"Naturally-formed bodies that orbit planets are called moons, or planetary satellites.","locator":"https://science.nasa.gov/solar-system/moons/""#;

const AST_PIN: &str = r#""bindings":{"Property":"rocky"},"citations":[{"source":"A star is a big ball of gas which gives off both heat and light.","locator":"https://starchild.gsfc.nasa.gov/docs/StarChild/universe_level1/stars.html","trust":"authoritative","corroborations":[{"source":"A planet is a large space object which revolves around a star.","locator":"https://starchild.gsfc.nasa.gov/docs/StarChild/solar_system_level1/planets.html"},{"source":"Naturally-formed bodies that orbit planets are called moons, or planetary satellites.","locator":"https://science.nasa.gov/solar-system/moons/"},{"source":"Comets are cosmic snowballs of frozen gases, rock, and dust that orbit the Sun.","locator":"https://science.nasa.gov/solar-system/comets/"},{"source":"Asteroids, sometimes called minor planets, are rocky, airless remnants left over from the early formation of our solar system about 4.6 billion years ago.","locator":"https://science.nasa.gov/solar-system/asteroids/""#;

#[test]
fn celestial_moon_answer_carries_its_nasa_corroboration_intact() {
    let dir = scratch("cite_moon");
    std::fs::copy(
        facts_stdlib().join("astronomy/celestial-objects.adj"),
        dir.join("celestial-objects.adj"),
    )
    .expect("copy shipped celestial-objects.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-objects.adj\"\n? celestial_property($Object, orbits_planet)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // science.nasa.gov is a modern CMS of the kind that is often JS-rendered,
    // and was treated as a blocker risk until checked: all three of its pages
    // returned 9000+ chars of body text and matched verbatim first try.
    assert!(
        out.contains(MOON_PIN),
        "moon's answer carries the NASA moons sentence verbatim: {out}"
    );
}

#[test]
fn celestial_asteroid_answer_carries_all_four_corroborations_in_order() {
    let dir = scratch("cite_ast");
    std::fs::copy(
        facts_stdlib().join("astronomy/celestial-objects.adj"),
        dir.join("celestial-objects.adj"),
    )
    .expect("copy shipped celestial-objects.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"celestial-objects.adj\"\n? celestial_property(asteroid, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(AST_PIN),
        "asteroid's answer carries all four corroborations in order: {out}"
    );
}
