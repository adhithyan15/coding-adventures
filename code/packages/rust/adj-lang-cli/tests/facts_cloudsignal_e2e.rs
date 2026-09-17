//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/cloud-signal.adj`) driven through the
//! built CLI: a native `table` recording, for cirrus and stratus clouds, each
//! individual weather signal an NWS sentence lists for it. Resolves forward
//! (multi-answer) and backward recall queries with the source's citation, plus
//! honest abstention on cumulonimbus -- 0 model calls.
//!
//! Each row carries its own cloud's sentence as its `source`. The envelope used
//! to be the cirrus value sentence, the primary source of the stratus answers
//! too. The stratus sentence has a NON-BREAKING SPACE (U+00A0) after
//! "precipitation-free", which the table shipped as an ordinary space.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cloudsignal_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/cloud-signal.adj");
    std::fs::copy(&src, dir.join("cloud-signal.adj")).expect("copy shipped cloud-signal.adj");
}

const LOCATOR: &str = "https://www.weather.gov/lmk/cloud_classification";
const ENVELOPE: &str = "Clouds are classified according to their height above and appearance (texture) from the ground.";
/// The stratus sentence as it used to ship: an ordinary space where the page
/// has U+00A0 after "precipitation-free". It occurs zero times on the page.
const STRATUS_PLAIN: &str = "Stratus clouds are uniform and flat, producing a gray layer of cloud cover which may be precipitation-free or may cause periods of light precipitation or drizzle.";

/// (cloud, value, that row's own sentence) -- generated from the converter's
/// page-verified spans, not retyped.
const ROWS: [(&str, &str, &str); 5] = [
    ("cirrus", "approaching_warm_front", "Cirrus clouds are wispy, feathery, and composed entirely of ice crystals. They often are the first sign of an approaching warm front or upper-level jet streak."),
    ("cirrus", "upper_level_jet_streak", "Cirrus clouds are wispy, feathery, and composed entirely of ice crystals. They often are the first sign of an approaching warm front or upper-level jet streak."),
    ("stratus", "precipitation_free", "Stratus clouds are uniform and flat, producing a gray layer of cloud cover which may be precipitation-free\u{a0}or may cause periods of light precipitation or drizzle."),
    ("stratus", "light_precipitation", "Stratus clouds are uniform and flat, producing a gray layer of cloud cover which may be precipitation-free\u{a0}or may cause periods of light precipitation or drizzle."),
    ("stratus", "drizzle", "Stratus clouds are uniform and flat, producing a gray layer of cloud cover which may be precipitation-free\u{a0}or may cause periods of light precipitation or drizzle."),
];

/// A string as the serializer writes it inside JSON: a quote becomes \".
fn json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The whole citation a row's answer must carry.
fn row_citation(sentence: &str) -> String {
    format!(
        "\"source\":\"{}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]",
        json(sentence)
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("meteorology/cloud-signal.adj"))
        .expect("read shipped cloud-signal.adj");
    adj[adj.find("table cloud_signal").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(dir.join("case.adj"), format!("import \"cloud-signal.adj\"\n? {query}\n")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

fn sentence_of(cloud: &str) -> &'static str {
    ROWS.iter().find(|r| r.0 == cloud).map(|r| r.2).expect("known cloud")
}

#[test]
fn every_row_is_warranted_by_its_own_clouds_sentence() {
    for (cloud, value, sentence) in ROWS {
        // BIND THE CLOUD FROM ITS VALUE: every value is unique, so this is one
        // answer; a fully ground query is ranked as a hypothesis with no
        // citations, which would let the negative arms pass over nothing.
        let out = ask(&format!("row_{value}"), &format!("cloud_signal($C, {value})"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {cloud}/{value}: {out}");
        assert!(out.contains(&format!("\"C\":\"{cloud}\"")), "{value} binds {cloud}: {out}");
        assert!(out.contains(&row_citation(sentence)), "{cloud}/{value}: its own cloud's sentence is primary: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != cloud {
                assert!(!out.contains(&json(other_sentence)), "the {other} sentence must not reach {cloud}/{value}: {out}");
            }
        }
        assert!(!out.contains(&json(ENVELOPE)), "the framing envelope warrants no row, including {cloud}/{value}: {out}");
    }
}

#[test]
fn the_stratus_sentence_carries_the_pages_non_breaking_space() {
    let stratus = sentence_of("stratus");
    assert_eq!(stratus.matches('\u{a0}').count(), 1, "exactly one U+00A0 in the stratus sentence");
    assert_eq!(stratus.replace('\u{a0}', " "), STRATUS_PLAIN, "it differs from the old string only there");
    let out = ask("nbsp", "cloud_signal(stratus, $V)");
    assert!(out.contains(&row_citation(stratus)), "the answer carries the page's character: {out}");
    assert!(!out.contains(&json(STRATUS_PLAIN)), "the ordinary-space string reaches no answer: {out}");
    // SCOPED TO `source "` LINES (#15415), the correction #15337, #15338 and
    // #15417 made for their tables. `shipped_table()` runs to end of file and
    // six `%` comment lines sit inside this block, so, read against that slice,
    // this arm FAILED A CORRECT FILE when a comment quoted the old sentence --
    // measured by mutant, not reasoned. A comment is not a shipped citation.
    //
    // The trade: nothing now pins "not anywhere in the block", only "no
    // `source` line carries it". All three stratus rows are still covered.
    let source_lines: String = shipped_table()
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!source_lines.contains(STRATUS_PLAIN), "and no `source` line ships it: {source_lines}");
}

#[test]
fn the_table_shape_is_one_source_per_row_and_a_framing_envelope() {
    let body = shipped_table();
    let row_sources = body.lines().filter(|l| l.starts_with("        source \"")).count();
    assert_eq!(row_sources, ROWS.len(), "one `source` per row");
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("source \"")).count(),
        ROWS.len() + 1,
        "the row sources and one envelope, at any indentation"
    );
    assert!(!body.lines().any(|l| l.trim_start().starts_with("cites ")), "no corroboration anywhere");
    assert!(
        !body.lines().any(|l| l.starts_with("        locator ") || l.starts_with("        trust ")),
        "no row restates the envelope's locator or trust (ADJ-TABLES section 4)"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the framing sentence at the page"
    );
    let folded = ENVELOPE.to_lowercase();
    for (cloud, value, sentence) in ROWS {
        assert!(!folded.contains(cloud), "the envelope must name no cloud, but names {cloud:?}");
        for word in value.split('_').filter(|w| w.len() > 3) {
            assert!(!folded.contains(word), "the envelope must name no value, but names {word:?} of {value:?}");
        }
        assert!(
            body.contains(&format!("    row ({cloud}, {value}) {{\n")) && body.contains(&format!("        source \"{}\"\n    }}", json(sentence))),
            "row ({cloud}, {value}) is shipped with its own cloud's sentence"
        );
    }
}

#[test]
fn cloud_signal_recalls_stratus_signals_with_citation() {
    let out = ask("stratus", "cloud_signal(stratus, $Signal)");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for signal in ["precipitation_free", "light_precipitation", "drizzle"] {
        assert!(
            out.contains(&format!("\"term\":\"cloud_signal(stratus, {signal})\"")),
            "stratus should recall {signal}: {out}"
        );
    }
    // ONE CONTIGUOUS RUN, not `contains("weather.gov") && contains(trust)`.
    assert!(out.contains(&row_citation(sentence_of("stratus"))), "carries the NWS citation, whole: {out}");
    assert!(!out.contains(&json(sentence_of("cirrus"))), "the cirrus sentence does not reach a stratus answer: {out}");
}

#[test]
fn cloud_signal_backward_recalls_cirrus_for_upper_level_jet_streak() {
    let out = ask("jetstreak", "cloud_signal($Cloud, upper_level_jet_streak)");
    assert!(
        out.contains("\"term\":\"cloud_signal(cirrus, upper_level_jet_streak)\""),
        "cirrus should be the only recalled cloud for upper_level_jet_streak: {out}"
    );
    assert!(out.contains(&row_citation(sentence_of("cirrus"))), "carries the cirrus citation, whole: {out}");
}

#[test]
fn cloud_signal_abstains_on_cumulonimbus() {
    let out = ask("abstain", "cloud_signal(cumulonimbus, $SignalCumulonimbus)");
    assert!(
        out.contains("\"abstained\":true"),
        "cumulonimbus's cited span names only one signal, no listed alternatives -- honest abstention expected: {out}"
    );
}
