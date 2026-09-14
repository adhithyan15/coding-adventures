//! End-to-end test for the geography reference-lines FACTS library
//! (`adj-facts-stdlib/geography/reference-lines.adj`): a native `table` of
//! reference line → what it marks resolves forward AND reverse binding queries
//! with the NOAA citation, and abstains on a line the table does not ground —
//! 0 answer-time model calls.

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
fn geography_reference_lines_recall_binds_marks_with_citation_and_abstains() {
    let dir = scratch("referencelines");
    // Copy the shipped geography table beside the entry program and import it.
    let src = facts_stdlib().join("geography/reference-lines.adj");
    std::fs::copy(&src, dir.join("reference-lines.adj"))
        .expect("copy shipped reference-lines.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-lines.adj\"\n\
         ? reference_line(equator, $Marks)\n\
         ? reference_line($Line, zero_degrees_longitude)\n\
         ? reference_line(international_date_line, $Marks)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // (a) Forward: the equator marks 0 degrees latitude — the source's atom.
    assert!(
        out.contains("\"Marks\":\"zero_degrees_latitude\""),
        "equator → zero_degrees_latitude: {out}"
    );
    // Reverse: the line at 0 degrees longitude is the prime meridian.
    assert!(
        out.contains("\"Line\":\"prime_meridian\""),
        "0 deg longitude → prime_meridian: {out}"
    );
    // First, the welded value itself must not come back. This names the
    // DEFECT rather than a citation, so no duplicate elsewhere in the output
    // can satisfy it on the real one's behalf. It is asserted BEFORE the
    // citation object deliberately: with the order the other way round, a
    // RESTORE mutant tripped the citation assertion first and this one was
    // never once observed to fire.
    assert!(
        !out.contains(
            "usually December 21. Two other significant lines of latitude"
        ),
        "the two NESDIS spans are not welded back into one: {out}"
    );
    // (a cont.) The answer carries the NOAA citation as its proof.
    //
    // The pin here used to be a HOSTNAME and a TRUST TIER:
    // `contains("oceanservice.noaa.gov") && contains(trust)`. Both were
    // satisfied by the welded 392-character NESDIS `cites` that #13934
    // installment 4m split -- a string that occurs ZERO times on the page it
    // named -- exactly as happily as by the two real spans that replaced it.
    // 4l's oceans pin had the same shape and the same hole.
    //
    // The needle is the whole citation object as the serialiser actually
    // emits it, taken from a real run rather than from memory of the format,
    // and it CLOSES on the corroborations `]`. That bounds four things at
    // once: the source text, the locator, the trust tier, and the
    // corroboration SET -- so a fabricated `cites` cannot be appended without
    // reddening (#14735).
    //
    // Its string appears FOUR times in this test's output, at four distinct
    // JSON paths: `recall/[0]/answers/[0]/citations/[0]` and `.../steps/[0]`,
    // and the same pair under `recall/[1]` for the reverse query. That was
    // counted BEFORE trusting the needle, and six mutations of the `.adj`
    // each drove all four to zero together -- echoes of one field, not the
    // independently-driftable copies of #14745.
    assert!(
        out.contains(
            "\"source\":\"The equator is the most well known parallel. At 0 degrees latitude, it equally divides the Earth into the Northern and Southern hemispheres.\",\"locator\":\"https://oceanservice.noaa.gov/facts/latitude.html\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"The prime meridian, which runs through Greenwich, England, has a longitude of 0 degrees. It divides the Earth into the eastern and western hemispheres.\",\"locator\":\"https://oceanservice.noaa.gov/facts/longitude.html\"},{\"source\":\"In the Northern Hemisphere, the Summer Solstice occurs when the sun is directly above the Tropic of Cancer, usually June 21. In the Southern Hemisphere, the Summer Solstice occurs when the sun is directly above the Tropic of Capricorn, usually December 21.\",\"locator\":\"https://www.nesdis.noaa.gov/about/k-12-education/optical-phenomena/what-solstice\"},{\"source\":\"Two other significant lines of latitude are the Arctic Circle (around the North Pole) and the Antarctic Circle (around the South Pole).\",\"locator\":\"https://www.nesdis.noaa.gov/about/k-12-education/optical-phenomena/what-solstice\"}]"
        ),
        "carries the NOAA citation, its three corroborations, and nothing else: {out}"
    );
    // (b) The international date line is a real line but is NOT grounded in a row
    // here — honest abstention, never a fabricated `marks` atom.
    assert!(
        out.contains("\"abstained\":true"),
        "international_date_line abstains: {out}"
    );
}

/// #14758: the two tropic rows assert a SUPERLATIVE — northernmost /
/// southernmost latitude at which the noon Sun stands overhead. Before RS-5e
/// per-row provenance they inherited the table envelope, whose `source` is a
/// sentence about the EQUATOR that states no bound at all. A reader auditing
/// the answer got NOAA, `authoritative`, and an irrelevant sentence.
///
/// Each row is queried in its OWN program, so the output holds exactly one
/// answer and a needle found in it necessarily belongs to that row. The first
/// draft of this test queried both at once and three of four mutations of the
/// `tropic_of_cancer` row stayed GREEN — the untouched `tropic_of_capricorn`
/// row satisfied every needle. A citation pin that is not bound to a binding
/// proves only that the string exists somewhere.
fn assert_tropic_row_is_self_grounded(tag: &str, query: &str, binding: &str) {
    let dir = scratch(tag);
    let src = facts_stdlib().join("geography/reference-lines.adj");
    std::fs::copy(&src, dir.join("reference-lines.adj"))
        .expect("copy shipped reference-lines.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"reference-lines.adj\"\n{query}\n"),
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");

    // The row still resolves. A provenance assertion over a row the engine
    // never reaches is worthless.
    assert!(out.contains(binding), "query binds {binding}: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to THIS row: {out}"
    );

    // The row's own span, its own locator, and the honest tier as ONE object.
    // Key-anchored: the bare URL also appears in the envelope's `cites`, so a
    // loose substring search would not be an exclusivity check.
    //
    // `inferred`, not `authoritative`, is the point. The SAME sentence is
    // `authoritative` in `reference-line-degree.adj` and
    // `reference-line-hemisphere-location.adj`, where it STATES its row (a
    // latitude; a hemisphere). Here the superlative is REASONED from it plus
    // the "never directly overhead" span. The tier describes the claim, not
    // the sentence.
    assert!(
        out.contains(
            "\"source\":\"One in the Northern Hemisphere called the Tropic of Cancer at +23.5° latitude and one in the Southern Hemisphere called the Tropic of Capricorn at − 23.5° latitude.\",\"locator\":\"https://www.nesdis.noaa.gov/about/k-12-education/optical-phenomena/what-solstice\",\"trust\":\"inferred\""
        ),
        "row carries its own span, its own locator, and trust inferred: {out}"
    );

    // The second justifying span reaches the answer. It existed ONLY inside
    // `%` comments, and comments do not travel with an answer: measured on the
    // PARENT COMMIT across all 1,330 `.adj` under `code/specs/data`, with
    // whitespace collapsed, it occurred in ZERO machine-readable values. It
    // occurs in one now — this file's row is that one.
    assert!(
        out.contains("the sun is never directly overhead."),
        "the bounding span reaches the answer, not just the comments: {out}"
    );

    // NAMED NEGATIVE. The defect was that a tropic recall shipped the equator
    // sentence as its warrant. It must not be THIS row's `source` — bounded by
    // the key, because the equator sentence legitimately remains the table
    // envelope and the test above still pins it there.
    assert!(
        !out.contains(
            "\"source\":\"The equator is the most well known parallel. At 0 degrees latitude, it equally divides the Earth into the Northern and Southern hemispheres.\",\"locator\":\"https://oceanservice.noaa.gov/facts/latitude.html\",\"trust\":\"authoritative\""
        ),
        "not warranted by the equator sentence: {out}"
    );
    assert!(
        !out.contains("\"trust\":\"authoritative\""),
        "no authoritative tier survives on a reasoned superlative: {out}"
    );
}

#[test]
fn geography_tropic_of_cancer_cites_its_own_spans_at_the_inferred_tier() {
    assert_tropic_row_is_self_grounded(
        "reflinestropiccancer",
        "? reference_line(tropic_of_cancer, $Marks)",
        "\"Marks\":\"northernmost_sun_overhead\"",
    );
}

#[test]
fn geography_tropic_of_capricorn_cites_its_own_spans_at_the_inferred_tier() {
    assert_tropic_row_is_self_grounded(
        "reflinestropiccapricorn",
        "? reference_line($Line, southernmost_sun_overhead)",
        "\"Line\":\"tropic_of_capricorn\"",
    );
}

/// Parse `(row key, its own `source`)` out of a shipped `.adj`.
///
/// The full two-column key: column 1 alone is not a row identity. A first pass
/// at the #15193 census keyed on column 1 and printed brain-parts as
/// `brainstem+brainstem+…`, because ten rows there share that part and differ
/// only in the function.
fn row_spans(rel: &str) -> Vec<(String, String)> {
    let adj = std::fs::read_to_string(facts_stdlib().join(rel))
        .unwrap_or_else(|e| panic!("read shipped {rel}: {e}"));
    let mut out: Vec<(String, String)> = Vec::new();
    let mut key: Option<String> = None;
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix("    row (") {
            key = rest.split(')').next().map(|k| {
                k.split(',').map(|p| p.trim()).collect::<Vec<_>>().join(", ")
            });
        } else if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            let span = rest.trim_end_matches(0x22 as char).to_string();
            out.push((key.clone().expect("a row precedes every source"), span));
        }
    }
    out
}

/// The groups of rows that share one span, in first-appearance order.
fn sharing_groups(pairs: &[(String, String)]) -> Vec<Vec<String>> {
    let mut order: Vec<String> = Vec::new();
    for (_, span) in pairs {
        if !order.contains(span) {
            order.push(span.clone());
        }
    }
    order
        .iter()
        .map(|span| {
            pairs
                .iter()
                .filter(|(_, s)| s == span)
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>()
        })
        .filter(|g| g.len() > 1)
        .collect()
}

/// #15193. The sharing structure of this table, asserted against the SHIPPED
/// FILE rather than described in a comment.
///
/// Two rows, ONE sentence: the page fixes both tropics in a single
/// clause.
#[test]
fn the_shared_spans_are_exactly_the_declared_ones() {
    let pairs = row_spans("geography/reference-lines.adj");
    assert_eq!(pairs.len(), 2, "every row carries its own source: {pairs:?}");
    let groups = sharing_groups(&pairs);
    let expected: Vec<Vec<&str>> = vec![
        vec!["tropic_of_cancer, northernmost_sun_overhead", "tropic_of_capricorn, southernmost_sun_overhead"],
    ];
    assert_eq!(
        groups, expected,
        "the shared sentences are shared by exactly these rows: {groups:?}"
    );
    let shared: usize = groups.iter().map(|g| g.len()).sum();
    assert_eq!(
        pairs.len() - shared,
        0,
        "and 0 rows have a sentence to themselves: {groups:?}"
    );
}
