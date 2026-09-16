//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/solar-eclipse-type.adj`) driven through
//! the built CLI: a native `table` naming three solar eclipse types and
//! what each actually is, quoted verbatim from NASA's "Types of Solar
//! Eclipses" page. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the total_solar_eclipse span, so an annular or partial answer was warranted
//! primarily by a sentence about a TOTAL eclipse.
//!
//! THE ENVELOPE'S APOSTROPHE IS U+2019. The same sentence with an ASCII
//! apostrophe occurs zero times on the page, so shipping one would cite a
//! sentence the source does not contain; `the_envelope_keeps_the_pages_own_apostrophe`
//! pins both directions.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_solar_eclipse_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/solar-eclipse-type.adj");
    std::fs::copy(&src, dir.join("solar-eclipse-type.adj"))
        .expect("copy shipped solar-eclipse-type.adj");
}

#[test]
fn solar_eclipse_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type(total_solar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATION, anchored on its JSON key and closed by the
    // terminating quote. This sentence carries a qualifier, so a
    // truncation would silently drop meaning -- the defect issue #13916
    // shipped. Pinning a fragment narrows that hole rather than closing
    // it, because `contains` on a fragment cannot see what precedes or
    // follows it. See issue #13918.
    //
    // RE-POINTED (RS-5e, #14986). This pinned the whole ENVELOPE sentence,
    // which passed only because that sentence was the table's envelope and so
    // rode on every answer -- including the annular and partial ones. It is
    // now the total row's OWN citations array, closing on both the
    // corroborations `]` and the citations `]`, so it can no longer be
    // satisfied by a sentence about a different eclipse type.
    assert!(
        out.contains(&only_citation(TOTAL)),
        "the total answer carries the sentence defining a TOTAL eclipse, whole: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope is primary for no answer: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"completely_blocking_the_face_of_the_sun\""),
        "total_solar_eclipse means completely_blocking_the_face_of_the_sun: {out}"
    );
    // This was `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any NASA citation satisfies and which constrains no sentence text.
    // The whole-array needle above is what pins the sentence now.
    assert!(
        out.contains(&format!("\"locator\":\"{LOCATOR}\"")),
        "carries the NASA locator: {out}"
    );
}

#[test]
fn solar_eclipse_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type($T, moon_at_or_near_its_farthest_point_from_earth)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"annular_solar_eclipse\""),
        "the shipped moon_at_or_near_its_farthest_point_from_earth example is annular_solar_eclipse: {out}"
    );
}

#[test]
fn solar_eclipse_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solar-eclipse-type.adj\"\n\
         ? solar_eclipse_type(hybrid_solar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hybrid_solar_eclipse is a real eclipse type the source covers but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}

const LOCATOR: &str = "https://science.nasa.gov/eclipses/types/";
/// The page's sentence defining a solar eclipse as such -- it names none of the
/// three types. Quoted with the page's own U+2019 apostrophe in "Sun's"; the
/// ASCII-apostrophe spelling occurs ZERO times on the page.
const ENVELOPE: &str = "A solar eclipse happens when the Moon passes between the Sun and Earth, casting a shadow on Earth that either fully or partially blocks the Sun\u{2019}s light in some areas.";
const TOTAL: &str = "A total solar eclipse happens when the Moon passes between the Sun and Earth, completely blocking the face of the Sun.";
const ANNULAR: &str = "An annular solar eclipse happens when the Moon passes between the Sun and Earth, but when it is at or near its farthest point from Earth.";
const PARTIAL: &str = "A partial solar eclipse happens when the Moon passes between the Sun and Earth but the Sun, Moon, and Earth are not perfectly lined up.";

/// (type, its description atom, the NASA sentence defining that type)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("total_solar_eclipse", "completely_blocking_the_face_of_the_sun", TOTAL),
        (
            "annular_solar_eclipse",
            "moon_at_or_near_its_farthest_point_from_earth",
            ANNULAR,
        ),
        (
            "partial_solar_eclipse",
            "sun_moon_and_earth_not_perfectly_lined_up",
            PARTIAL,
        ),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/solar-eclipse-type.adj"))
        .expect("read shipped solar-eclipse-type.adj");
    adj[adj.find("table solar_eclipse_type").expect("table")..].to_string()
}

#[test]
fn every_type_answer_carries_only_the_sentence_defining_that_type() {
    for (kind, description, span) in scale() {
        let dir = scratch(&format!("type_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"solar-eclipse-type.adj\"\n? solar_eclipse_type({kind}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {kind}: {out}"
        );
        assert!(
            out.contains(&format!("\"D\":\"{description}\"")),
            "{kind} -> {description}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{kind}: the NASA sentence defining it, whole, and the only citation: {out}"
        );
        for other in [TOTAL, ANNULAR, PARTIAL] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another type's sentence must not reach {kind}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {kind}: {out}"
        );
    }
}

#[test]
fn the_envelope_keeps_the_pages_own_apostrophe() {
    // The page writes "Sun\u{2019}s" with U+2019. An ASCII-apostrophe spelling
    // occurs ZERO times on the page, so shipping one would be a citation to a
    // sentence the source does not contain -- the defect #15324 records for
    // soil-texture-class, and the one volcano-type and metamorphism-cause both
    // shipped before their conversions.
    let body = shipped_table();
    assert!(
        body.contains(ENVELOPE),
        "the envelope ships with the page's own U+2019: {body}"
    );
    let ascii_variant = ENVELOPE.replace('\u{2019}', "'");
    assert!(
        !body.contains(&ascii_variant),
        "an ASCII-apostrophe envelope occurs zero times on the page and must not ship: {body}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        3,
        "three row sources"
    );
    // Keyword-anchored, not indent-scoped: a row-level `cites` at eight spaces
    // must fail this too, without false-positiving on prose in a comment.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent"
    );
    // Indent-independent, unlike the `cites` arm above -- `cites` can assert
    // ABSENCE because no table-level `cites` exists, whereas the envelope has a
    // `locator` and a `trust` of its own. So the pin is EXACTLY one of each.
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("locator "))
            .count(),
        1,
        "exactly one locator line, the envelope's: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("trust "))
            .count(),
        1,
        "exactly one trust line, the envelope's: {body}"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's eclipse-defining sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{TOTAL}\"\n    locator")),
        "not the total_solar_eclipse span as the envelope again"
    );
    // WHOLE WORDS, not substrings. The shipped envelope contains "partially",
    // and a bare `contains("partial")` matches inside it -- the assertion would
    // fail on a correct envelope. Split on non-alphanumerics and compare whole
    // tokens, the same fix the hurricane bare-word needle needed in reverse.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["total", "annular", "partial", "hybrid"] {
        assert!(
            !tokens.contains(&word),
            "the shipped envelope must name no eclipse type, but contains {word:?} \
             as a whole word: {shipped_envelope}"
        );
    }
}
