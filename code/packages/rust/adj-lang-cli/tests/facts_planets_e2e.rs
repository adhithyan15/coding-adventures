//! End-to-end test for the astronomy planets FACTS library
//! (`adj-facts-stdlib/astronomy/planets.adj`): a native `table` of
//! planet → order-from-the-Sun resolves forward AND reverse binding queries with
//! the NASA citation, and abstains on a non-planet — 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn astronomy_planets_recall_binds_order_forward_and_reverse() {
    let dir = scratch("planets");
    let src = facts_stdlib().join("astronomy/planets.adj");
    std::fs::copy(&src, dir.join("planets.adj")).expect("copy shipped planets.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"planets.adj\"\n\
         ? planet_order(earth, $N)\n\
         ? planet_order($Planet, 1)\n\
         ? planet_order(pluto, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: Earth is the third planet from the Sun.
    assert!(out.contains("\"N\":\"3\""), "earth → 3: {out}");
    // Reverse: the first planet from the Sun is Mercury (binds the other column).
    assert!(out.contains("\"Planet\":\"mercury\""), "order 1 → mercury: {out}");
    // The answer carries the NASA citation as its proof.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
    // Pluto (a dwarf planet) is not a row — honest abstention, no fabricated order.
    assert!(out.contains("\"abstained\":true"), "pluto abstains: {out}");
}

const LOCATOR: &str = "https://science.nasa.gov/solar-system/planets/";

/// Assert one planet's row carries the page's own sentence for THAT planet,
/// as a whole `source`/`locator`/`trust` object, and that it is the only
/// answer in the program so the needle cannot be satisfied by a sibling row.
fn assert_planet_row(tag: &str, planet: &str, order: &str, span: &str, tier: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("astronomy/planets.adj"),
        dir.join("planets.adj"),
    )
    .expect("copy shipped planets.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"planets.adj\"\n? planet_order({planet}, $N)\n"),
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to {planet}: {out}"
    );
    assert!(out.contains(&format!("\"N\":\"{order}\"")), "{planet} -> {order}: {out}");
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"{tier}\""
        )),
        "{planet} carries the page's own sentence for it, at tier {tier}: {out}"
    );
    // NAMED NEGATIVE. The Venus sentence was the old envelope and therefore
    // every row's warrant; a recall of Jupiter quoted a sentence about Venus.
    if planet != "venus" {
        assert!(
            !out.contains("Venus is the second planet from the Sun"),
            "{planet} is not warranted by a sentence about Venus: {out}"
        );
    }
    out
}

/// #14986: this table had one envelope — the VENUS sentence — so every row
/// answered with it. Ask for Jupiter and the citation was about Venus.
///
/// Seven of the eight rows are now warranted by a sentence that LITERALLY
/// states their ordinal, so they are read, and inherit `authoritative`.
#[test]
fn astronomy_planets_rows_are_read_off_their_own_sentence() {
    assert_planet_row(
        "planetsjupiter",
        "jupiter",
        "5",
        "Jupiter is the fifth planet from the Sun, and the largest planet in our solar system.",
        "authoritative",
    );
    assert_planet_row(
        "planetsneptune",
        "neptune",
        "8",
        "Neptune is the eighth and most distant planet in our solar system.",
        "authoritative",
    );
    // The en dashes in the Earth sentence are the page's own (U+2013); it
    // occurs exactly once in the raw HTML and once in the rendered text.
    assert_planet_row(
        "planetsearth",
        "earth",
        "3",
        "Earth – our home planet – is the third planet from the Sun, and the fifth largest planet.",
        "authoritative",
    );
}

/// MERCURY IS THE ONE ROW THE PAGE DOES NOT STATE LITERALLY. It says "the
/// planet nearest to the Sun", never "the first planet". Position 1 is read
/// off that plus the ordering of a second span in which Mercury is named
/// first — a reading, not a quotation — so the row carries `trust inferred`
/// and both spans, exactly as the tropic rows in
/// `geography/reference-lines.adj` do.
///
/// The tier is the whole point: it is what distinguishes this row from the
/// seven above, and it would be wrong for any of them.
#[test]
fn astronomy_planets_mercury_is_reasoned_not_read() {
    let out = assert_planet_row(
        "planetsmercury",
        "mercury",
        "1",
        "Mercury is the planet nearest to the Sun, and the smallest planet in our solar system.",
        "inferred",
    );
    // The ordering span AND its locator, as one object. Asserting only the
    // text left the `cites` locator unpinned: review repointed it at
    // en.wikipedia.org and every test stayed green, so the second span could
    // have been attributed to a page nobody checked.
    assert!(
        out.contains(&format!(
            "\"source\":\"The first four planets from the Sun are Mercury, Venus, Earth, and Mars.\",\"locator\":\"{LOCATOR}\""
        )),
        "the ordering span that supplies position 1 reaches the answer, \
         under the locator it was read from: {out}"
    );
    assert!(
        !out.contains("\"trust\":\"authoritative\""),
        "a reasoned row does not claim the read tier: {out}"
    );
}

/// The other four. Review measured that `venus`, `mars`, `saturn` and
/// `uranus` were entirely unpinned: dropping a block, fabricating a span, and
/// even CORRUPTING THE ORDER VALUE all stayed green. Four of eight, not the
/// one the changelog first disclosed.
#[test]
fn astronomy_planets_remaining_rows_are_pinned_too() {
    assert_planet_row(
        "planetsvenus",
        "venus",
        "2",
        "Venus is the second planet from the Sun, and the sixth largest planet.",
        "authoritative",
    );
    assert_planet_row(
        "planetsmars",
        "mars",
        "4",
        "Mars is the fourth planet from the Sun, and the seventh largest planet.",
        "authoritative",
    );
    assert_planet_row(
        "planetssaturn",
        "saturn",
        "6",
        "Saturn is the sixth planet from the Sun, the second largest planet in our solar system.",
        "authoritative",
    );
    assert_planet_row(
        "planetsuranus",
        "uranus",
        "7",
        "Uranus is the seventh planet from the Sun, and the third largest planet in our solar system.",
        "authoritative",
    );
}

/// THE ENVELOPE IS UNREACHABLE BY CONSTRUCTION, and that is worth asserting
/// rather than assuming.
///
/// Every row now overrides `source`, so the table's framing span can never
/// appear in any answer. Review measured the consequence: replacing it with
/// `"Our solar system has nine planets: entirely fabricated."` kept every test
/// green. No output-based pin can fix that — the string is documentation-only.
///
/// What CAN be pinned is the property itself. If a future change let the
/// envelope leak back into an answer — a row losing its block, the override
/// semantics changing — this reddens. That is the regression worth catching;
/// the envelope's wording is not something a CLI test can see at all.
#[test]
fn astronomy_planets_envelope_never_reaches_an_answer() {
    let dir = scratch("planetsenvelope");
    std::fs::copy(
        facts_stdlib().join("astronomy/planets.adj"),
        dir.join("planets.adj"),
    )
    .expect("copy shipped planets.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"planets.adj\"\n\
         ? planet_order(mercury, $N)\n\
         ? planet_order(venus, $N)\n\
         ? planet_order(earth, $N)\n\
         ? planet_order(mars, $N)\n\
         ? planet_order(jupiter, $N)\n\
         ? planet_order(saturn, $N)\n\
         ? planet_order(uranus, $N)\n\
         ? planet_order(neptune, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Positive arm first: all eight rows answered, so the absence below is
    // about the envelope and not about an empty output.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        8,
        "all eight planets answer: {out}"
    );
    assert!(
        !out.contains("Our solar system has eight planets"),
        "the framing span warrants no row -- every row overrides it: {out}"
    );
}
