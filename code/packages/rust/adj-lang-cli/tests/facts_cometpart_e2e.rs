//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/comet-part.adj`) driven through the built
//! CLI: a native `table` naming three parts of a comet and what each
//! actually is, quoted verbatim from NASA Space Place's "What Is a
//! Comet?" page. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the nucleus span, so a coma or tail answer was warranted primarily by a
//! sentence about the nucleus.
//!
//! ALL THREE SPANS MENTION THE NUCLEUS -- the coma is a cloud AROUND it, and
//! the tail streams AWAY FROM it. So every negative arm here names a WHOLE
//! SPAN; the bare word `nucleus` is satisfied by all three and would prove
//! nothing about which sentence reached an answer.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comet_part_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/comet-part.adj");
    std::fs::copy(&src, dir.join("comet-part.adj")).expect("copy shipped comet-part.adj");
}

#[test]
fn comet_part_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part(nucleus, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"solid_frozen_core_at_the_heart_of_the_comet\""),
        "nucleus means solid_frozen_core_at_the_heart_of_the_comet: {out}"
    );
    // This was `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any NASA citation satisfies and which constrains no sentence text.
    // Before the RS-5e conversion it was satisfied by the nucleus span riding
    // on every answer -- including the coma and tail ones.
    //
    // Two SEPARATE assertions, not one `&&`: joined, a failure cannot say which
    // arm broke.
    // Exactly one array, so a second could not carry a citation this test never
    // looks at. The per-part check asserts the same for every row.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one citations array: {out}"
    );
    assert!(
        out.contains(&only_citation(NUCLEUS)),
        "the nucleus answer carries the sentence introducing the nucleus, whole, \
         and as its only citation: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
}

#[test]
fn comet_part_reverse_binds_the_part_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part($P, fuzzy_cloud_of_gas_and_dust_around_the_nucleus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"coma\""),
        "the shipped fuzzy_cloud_of_gas_and_dust_around_the_nucleus example is coma: {out}"
    );
}

#[test]
fn comet_part_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comet-part.adj\"\n\
         ? comet_part(short_period_comet, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "short_period_comet is a real comet-related term the source covers but not one of the three physical parts tabled here -- honest abstention, never invented: {out}"
    );
}

const LOCATOR: &str = "https://spaceplace.nasa.gov/comets/en/";
/// The page's sentence framing the anatomy as a whole. It names no part.
const ENVELOPE: &str = "This diagram is not to scale, but it shows the anatomy of a comet.";
const NUCLEUS: &str = "At the heart of every comet is a solid, frozen core called the nucleus.";
const COMA: &str = "The gas and dust create a huge, fuzzy cloud around the nucleus called the coma.";
const TAIL: &str = "As dust and gases stream away from the nucleus, sunlight and particles coming from the Sun push them into a bright tail that stretches away from the Sun for millions of miles.";

/// (part, its description atom, the NASA sentence introducing that part)
///
/// ALL THREE SPANS MENTION THE NUCLEUS -- the coma is a cloud AROUND it and the
/// tail streams AWAY FROM it. So a negative arm must name a WHOLE SPAN; the
/// bare word `nucleus` is satisfied by all three and would prove nothing.
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("nucleus", "solid_frozen_core_at_the_heart_of_the_comet", NUCLEUS),
        ("coma", "fuzzy_cloud_of_gas_and_dust_around_the_nucleus", COMA),
        (
            "tail",
            "streams_away_from_the_nucleus_pushed_by_sunlight_and_solar_particles",
            TAIL,
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
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/comet-part.adj"))
        .expect("read shipped comet-part.adj");
    adj[adj.find("table comet_part").expect("table")..].to_string()
}

#[test]
fn every_part_answer_carries_only_the_sentence_introducing_that_part() {
    for (part, description, span) in scale() {
        let dir = scratch(&format!("part_{part}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"comet-part.adj\"\n? comet_part({part}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {part}: {out}"
        );
        assert!(
            out.contains(&format!("\"D\":\"{description}\"")),
            "{part} -> {description}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{part}: the NASA sentence introducing it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS only. Never the bare word `nucleus`: all three spans
        // contain it, so such an arm would fire for every part and prove
        // nothing about which sentence reached this answer.
        for other in [NUCLEUS, COMA, TAIL] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another part's sentence must not reach {part}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {part}: {out}"
        );
    }
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
        "the envelope is the page's anatomy-framing sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{NUCLEUS}\"\n    locator")),
        "not the nucleus span as the envelope again"
    );
    // WHOLE WORDS, not substrings: the envelope must name no part. Split on
    // non-alphanumerics so a longer word containing a part name cannot trip it
    // -- the mirror of the solar-eclipse case, where "partially" contained
    // "partial" and a substring needle failed on a CORRECT file.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in ["nucleus", "coma", "tail", "tails"] {
        assert!(
            !tokens.contains(&word),
            "the shipped envelope must name no comet part, but contains {word:?} \
             as a whole word: {shipped_envelope}"
        );
    }
}

#[test]
fn every_row_description_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs: the description atom is a phrase drawn from the sentence, so the
    // normalized span must contain it. This is what `element-categories`
    // (#15318) failed -- a row whose span named no category at all.
    let body = shipped_table();
    for (part, description, span) in scale() {
        assert!(
            body.contains(&format!("        source \"{span}\"\n")),
            "{part} carries its own span: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        // The atom is a compressed paraphrase, so assert the span names the
        // PART itself plus the atom's distinctive content word -- not the whole
        // de-underscored atom, which is not a contiguous phrase on the page.
        // WHOLE WORDS, not a substring. `contains("tail")` is satisfied by
        // "tails" or "detail", and this page uses the plural -- the shape test
        // below lists "tails" as its own needle for that reason. The project
        // has already been bitten by the inverse, where a substring needle for
        // "partial" failed on a CORRECT file containing "partially".
        let tokens: Vec<&str> = normalized
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| !t.is_empty())
            .collect();
        assert!(
            tokens.contains(&part),
            "{part}: its own span must name the part as a whole word, but {part:?} \
             is not a token of {normalized:?}"
        );
        // EXHAUSTIVE, with no catch-all arm. A `_` arm silently applies one
        // row's needles to any row added later, so a new row would be checked
        // against content that has nothing to do with it -- passing by accident
        // or failing with a message naming the wrong content.
        let distinctive: &[&str] = match part {
            "nucleus" => &["frozen core"],
            "coma" => &["fuzzy cloud"],
            // The atom claims the push MECHANISM as well as the streaming, so
            // the proxy checks both: a span stating only that the tail streams
            // away would let the atom over-claim unchallenged.
            "tail" => &["stream away from the nucleus", "sunlight", "particles"],
            other => panic!("no distinctive phrase registered for row {other}"),
        };
        for needle in distinctive {
            assert!(
                normalized.contains(*needle),
                "{part}: its span must state {needle:?}, part of the content the atom \
                 {description:?} compresses, but it is not in {normalized:?}"
            );
        }
    }
}
