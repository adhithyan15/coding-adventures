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
    // ONE CONTIGUOUS SPAN, not two loose substrings. `contains(host) &&
    // contains(trust)` was here, and that is the #15139 shape: two halves
    // satisfiable by different parts of the output, which drift apart the
    // moment one of them moves. This pins the RADIO row's whole warrant --
    // and note the source is now radio's OWN sentence, not the table's.
    assert!(
        out.contains(
            "\"source\":\"Your radio captures radio waves emitted by radio stations, bringing your favorite tunes.\",\"locator\":\"https://imagine.gsfc.nasa.gov/science/toolbox/emspectrum1.html\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "the radio row's whole warrant, contiguous: {out}"
    );
    // Sound is a mechanical wave, not one of the seven EM bands — honest
    // abstention, never a fabricated use.
    assert!(out.contains("\"abstained\":true"), "sound abstains: {out}");
}

/// The envelope sentence and locator, named once so the pins below bind the
/// CLI's real output rather than a restatement of it.
const ENVELOPE: &str = "The image below shows where you might encounter each portion of the EM spectrum in your day-to-day life.";
const LOCATOR: &str = "https://imagine.gsfc.nasa.gov/science/toolbox/emspectrum1.html";

/// Run one query against the shipped table and return stdout.
fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    let src = facts_stdlib().join("physics/em-spectrum.adj");
    std::fs::copy(&src, dir.join("em-spectrum.adj")).expect("copy shipped em-spectrum.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"em-spectrum.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

const BANDS: [(&str, &str, &str); 7] = [
        ("radio", "radio_stations", "Your radio captures radio waves emitted by radio stations, bringing your favorite tunes."),
        ("microwave", "cooking", "Microwave radiation will cook your popcorn in just a few minutes, but is also used by astronomers to learn about the structure of nearby galaxies."),
        ("infrared", "night_vision", "Night vision goggles pick up the infrared light emitted by our skin and objects with heat."),
        ("visible", "eyes", "Our eyes detect visible light."),
        ("ultraviolet", "tanning", "Ultraviolet radiation is emitted by the Sun and are the reason skin tans and burns."),
        ("x_ray", "teeth", "A dentist uses X-rays to image your teeth, and airport security uses them to see through your bag."),
        ("gamma_ray", "medical_imaging", "Doctors use gamma-ray imaging to see inside your body."),
];

#[test]
fn every_band_is_warranted_by_the_sentence_that_names_it() {
    // THIS IS THE CHANGE. The RADIO sentence used to be the envelope `source`
    // and therefore the primary warrant for all seven rows, so
    // `? band_use(x_ray, $A)` came back proved by a sentence about radio
    // stations. Six of the seven were in that position.
    for (band, application, sentence) in BANDS {
        let out = ask(&format!("warrant_{band}"), &format!("band_use({band}, $A)"));
        // ONE ANSWER, ASSERTED. A query binding several answers would carry
        // other rows' sentences in the same stdout, so the negative arm below
        // would measure nothing (#15164).
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {band}: {out}"
        );
        assert!(
            out.contains(&format!("\"A\":\"{application}\"")),
            "{band} still binds {application}: {out}"
        );
        // SEMANTIC, NOT A FILE TEXT MATCH: the whole warrant as one run of
        // bytes, so the row's own sentence is pinned as its PRIMARY source
        // together with the locator and tier it inherits.
        assert!(
            out.contains(&format!(
                "\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]"
            )),
            "{band}'s warrant is its own sentence, whole: {out}"
        );
        // THE NEGATIVE ARM. The radio sentence must not reach any other
        // answer -- it reached all seven before this change.
        if band != "radio" {
            assert!(
                !out.contains(
                    "Your radio captures radio waves emitted by radio stations, bringing your favorite tunes."
                ),
                "the radio sentence must not warrant {band}: {out}"
            );
        }
    }
}

#[test]
fn the_envelope_framing_sentence_reaches_no_answer() {
    // All seven rows override `source`, so the framing sentence is emitted
    // ZERO times. THE ZERO IS THE INSTRUMENT: the sharpest available form of
    // "the envelope mis-warrants no row", and it reddens the moment a row
    // loses its block and falls back to the envelope.
    let out = ask("framing", "band_use($B, $A)");
    assert_eq!(
        out.matches(ENVELOPE).count(),
        0,
        "the framing sentence warrants no answer: {out}"
    );
    // POSITIVE CONTROL, because a zero proves nothing about a program that
    // printed nothing. Seven rows, and provenance is emitted twice per answer
    // (once under `citations`, once under `steps`).
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven rows answered: {out}"
    );
    for (band, _, sentence) in BANDS {
        assert_eq!(
            out.matches(sentence).count(),
            2,
            "{band}'s span appears once per citations and once per steps: {out}"
        );
    }
}

#[test]
fn every_row_carries_its_own_distinct_span_and_none_restates_the_locator() {
    // STRUCTURAL, AND IT READS THE SHIPPED FILE, so one side of each
    // comparison is the artifact rather than another literal in this test.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/em-spectrum.adj"))
        .expect("read shipped em-spectrum.adj");
    let body = &adj[adj.find("table band_use").expect("table")..];

    assert_eq!(body.matches("\n    row (").count(), 7, "seven rows");
    let mut spans: Vec<&str> = body
        .lines()
        .filter_map(|l| l.strip_prefix("        source \""))
        .map(|r| r.trim_end_matches(0x22 as char))
        .collect();
    assert_eq!(spans.len(), 7, "every row carries its own source block");

    // TWO DIFFERENT PROPERTIES, and only the first used to be asserted while
    // the CHANGELOG claimed the second.
    //
    // (a) NO TWO ROWS CARRY THE IDENTICAL SPAN. String dedup. This is what
    //     stops a single-row truncation hiding behind a sibling's copy of the
    //     same sentence, which is what let a mutant survive in
    //     `water-movement-route`.
    let keys: Vec<String> = body
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("row ("))
        .map(|r| r.split(',').next().expect("row key").trim().to_lowercase())
        .collect();
    assert_eq!(keys.len(), 7, "seven row keys read: {keys:?}");
    let ordered = spans.clone();
    let before = spans.len();
    spans.sort_unstable();
    spans.dedup();
    assert_eq!(spans.len(), before, "all seven spans are distinct: {spans:?}");

    // (b) NO SPAN NAMES ANOTHER ROW'S BAND. This is the property the header
    //     actually argues for, and nothing asserted it: string dedup says
    //     nothing about a span that mentions a sibling. Checked in the PAGE'S
    //     wording, because `x_ray` is written "X-rays" and `gamma_ray` is
    //     written "gamma-ray" -- an atom-only scan would miss both.
    for (i, span) in ordered.iter().enumerate() {
        let low = span.to_lowercase();
        for (j, key) in keys.iter().enumerate() {
            if i == j {
                continue;
            }
            for form in [key.replace('_', " "), key.replace('_', "-"), key.clone()] {
                assert!(
                    !low.contains(&form),
                    "the {} span must name no other band, but names {form:?}: {span:?}",
                    keys[i]
                );
            }
        }
        // And it DOES name its own -- otherwise the loop above would pass
        // vacuously on a table whose spans mention no bands at all.
        let own = &keys[i];
        assert!(
            [own.replace('_', " "), own.replace('_', "-"), own.clone()]
                .iter()
                .any(|f| low.contains(f)),
            "the {own} span must name its own band: {span:?}"
        );
    }

    // TOTAL DERIVATIONS, not positional ones.
    let table_sources: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect();
    assert_eq!(table_sources.len(), 1, "exactly one table-level source");
    let table_locators: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    locator \""))
        .collect();
    assert_eq!(table_locators.len(), 1, "exactly one table-level locator");
    // MATCH ON THE TOKEN, not on `locator "` as one string. The old form
    // missed `locator  "…"` with two spaces -- which escaped BOTH this arm
    // and the four-space filter above -- and an inline `row (…) { locator
    // "…" }`, whose `trim_start()` yields `row (`.
    for l in adj.lines() {
        if l.split_whitespace().next() == Some("locator") {
            let indent = l.len() - l.trim_start().len();
            assert_eq!(indent, 4, "the only locator is the table's: {l:?}");
        }
    }
    for l in body.lines() {
        if l.contains("row (") && l.contains('{') {
            assert!(
                !l.contains("locator"),
                "no row states a locator inline either: {l:?}"
            );
        }
    }

    let envelope = table_sources[0]
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char);
    assert_eq!(envelope, ENVELOPE, "the framing slot, verbatim");
}

#[test]
fn the_envelope_names_no_band_in_the_pages_own_wording() {
    // A KEY-ONLY CHECK WOULD NOT BE ENOUGH. The page writes `x_ray` as
    // "X-rays" and `gamma_ray` as "gamma-ray", so a framing sentence naming
    // X-rays would pass a scan for the atom `x_ray` while plainly naming a
    // band. Both forms are checked, and the keys are read from the table so a
    // new row re-checks the envelope automatically.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/em-spectrum.adj"))
        .expect("read shipped em-spectrum.adj");
    let body = &adj[adj.find("table band_use").expect("table")..];
    // READ FROM THE FILE, NOT FROM THE CONST. This used to test
    // `ENVELOPE.to_lowercase()` while harvesting the keys from the `.adj`, so
    // a mutant editing the shipped envelope to name X-rays was not killed
    // here at all -- it was killed by the `assert_eq!(envelope, ENVELOPE)` in
    // the other test, and this one only caught a COORDINATED edit to both.
    // Reading the file makes it buy what its name promises.
    let table_sources: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect();
    assert_eq!(table_sources.len(), 1, "exactly one table-level source");
    let envelope = table_sources[0]
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char)
        .to_lowercase();

    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim().to_lowercase();
            keys += 1;
            for form in [key.replace('_', " "), key.replace('_', "-"), key.clone()] {
                assert!(
                    !envelope.contains(&form),
                    "the framing sentence must name no band, but names {form:?}"
                );
            }
        }
    }
    assert_eq!(keys, 7, "all seven keys were actually checked");
}
