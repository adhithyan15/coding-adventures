//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/kingdoms.adj`) driven through the built CLI:
//! a native `table` of the biological kingdoms of life → a representative
//! example organism resolves binding-query recalls (forward AND backward) with
//! the source's Science Notes citation at the `consensus` trust tier, and
//! abstains on a word that is not one of these kingdoms (a virus) — 0 model
//! calls.

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
fn biology_kingdoms_recall_binds_example_with_citation() {
    let dir = scratch("kingdoms");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/kingdoms.adj");
    std::fs::copy(&src, dir.join("kingdoms.adj")).expect("copy shipped kingdoms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"kingdoms.adj\"\n\
         ? kingdom_example(animalia, $Example)\n\
         ? kingdom_example(fungi, $Example)\n\
         ? kingdom_example(bacteria, $Example)\n\
         ? kingdom_example($Kingdom, mushrooms)\n\
         ? kingdom_example(virus, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The animal kingdom's listed example is humans, the fungus kingdom's is
    // mushrooms, the bacteria kingdom's is cyanobacteria — the recalled examples
    // (forward binds).
    assert!(
        out.contains("\"Example\":\"humans\""),
        "animalia → humans: {out}"
    );
    assert!(
        out.contains("\"Example\":\"mushrooms\""),
        "fungi → mushrooms: {out}"
    );
    assert!(
        out.contains("\"Example\":\"cyanobacteria\""),
        "bacteria → cyanobacteria: {out}"
    );
    // The relation runs BACKWARD: bind the example `mushrooms`, recall its
    // kingdom.
    assert!(
        out.contains("\"Kingdom\":\"fungi\""),
        "mushrooms → fungi (reverse recall): {out}"
    );
    // The answer carries the Science Notes citation as its proof, at the
    // `consensus` trust tier for a secondary teaching source.
    assert!(
        out.contains("sciencenotes.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A virus is not placed in any of these kingdoms — honest abstention, never
    // a fabricated example.
    assert!(out.contains("\"abstained\":true"), "virus abstains: {out}");
}

#[test]
fn biology_kingdoms_extension_recalls_every_example_per_kingdom() {
    let dir = scratch("kingdoms_ext");
    let src = facts_stdlib().join("biology/kingdoms.adj");
    std::fs::copy(&src, dir.join("kingdoms.adj")).expect("copy shipped kingdoms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"kingdoms.adj\"\n\
         ? kingdom_example(fungi, $Example)\n\
         ? kingdom_example(bacteria, $Example)\n\
         ? kingdom_example($Kingdom, diatoms)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // fungi now recalls THREE examples (mushrooms, yeast, molds), not just
    // the one originally shipped -- the table went from single-valued to
    // many-valued per kingdom this cycle.
    assert!(
        out.contains("\"Example\":\"yeast\""),
        "fungi → yeast (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Example\":\"molds\""),
        "fungi → molds (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Example\":\"gram_positive_bacteria\""),
        "bacteria → gram_positive_bacteria (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Example\":\"gram_negative_bacteria\""),
        "bacteria → gram_negative_bacteria (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Example\":\"actinobacteria\""),
        "bacteria → actinobacteria (added this cycle): {out}"
    );
    // Reverse recall on a newly-added example still resolves to its kingdom.
    assert!(
        out.contains("\"Kingdom\":\"protista\""),
        "diatoms → protista (reverse recall on a newly-added example): {out}"
    );
}

const LOCATOR: &str = "https://sciencenotes.org/kingdoms-of-life-in-biology/";

/// Assert that EVERY row of one kingdom carries that kingdom's own
/// "Examples:" line, as a whole `source`/`locator`/`trust` object.
///
/// Two things here were learned by a mutant surviving, not by design:
///
/// 1. The pin is the WHOLE object, not the span plus a loose
///    `contains("sciencenotes.org")`. The envelope's `cites` also carries that
///    locator, so a bare-substring check for it stayed green when the
///    envelope's own `locator` was deleted — it was satisfied by the
///    corroboration, never by the inheritance it claimed to test.
///
/// 2. The COUNT is asserted, not just presence. A kingdom query returns one
///    answer per row, so dropping ONE row's block left the others satisfying a
///    `contains` check — the same cross-row masking, inside a single kingdom.
///    `expected_rows * 2` because each answer emits the provenance twice, once
///    under `citations` and once under `steps`.
fn assert_kingdom_rows_cite_their_own_line(
    tag: &str,
    kingdom: &str,
    span: &str,
    expected_rows: usize,
) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/kingdoms.adj"),
        dir.join("kingdoms.adj"),
    )
    .expect("copy shipped kingdoms.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"kingdoms.adj\"\n? kingdom_example({kingdom}, $Example)\n"),
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // One `citations` array per answer. (`"bindings"` is NOT the needle for
    // this: it occurs twice per answer, measured, not assumed.)
    assert_eq!(
        out.matches("\"citations\":[").count(),
        expected_rows,
        "{kingdom} recalls exactly {expected_rows} examples: {out}"
    );

    let object = format!(
        "\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\""
    );
    assert_eq!(
        out.matches(&object).count(),
        expected_rows * 2,
        "every {kingdom} row carries its own line, its locator and its tier \
         as one object: {out}"
    );
    out
}

/// #14986: every row used to answer with the ANIMALIA "Examples:" line,
/// because an ADJ table had one envelope. Ask for a fungus and the citation
/// was a sentence about humans and sponges. Each row now carries its own
/// kingdom's line as an RS-5e per-row `source`.
#[test]
fn biology_kingdoms_each_row_cites_its_own_kingdoms_examples_line() {
    // Verified 2026-09-13 UTC by raw fetch: this line occurs exactly once in
    // the page's rendered text.
    let out = assert_kingdom_rows_cite_their_own_line(
        "kingdomsperrow",
        "fungi",
        "Examples: Mushrooms, yeast, molds, rusts",
        3,
    );
    // NAMED NEGATIVE. The animalia line was the old envelope and therefore
    // every row's warrant. No fungi answer may carry it now.
    assert!(
        !out.contains("Examples: Humans, birds, crustaceans, sponges"),
        "a fungus is not warranted by a sentence about humans and sponges: {out}"
    );
}

/// The cited page does not state `plantae -> multicellular_algae`. Measured
/// 2026-09-13 UTC by raw fetch: the plantae line reads "Examples: Flowers,
/// grasses, conifers, ferns, mosses", and "multicellular algae" occurs ZERO
/// times on the page -- in the rendered text AND in the raw HTML, which no
/// choice of extractor can move. The row was removed, so the recall
/// must decline to assert it.
///
/// The absence pin is paired with a positive arm in the same program, and the
/// row count is exact — otherwise it would keep passing if the whole table
/// vanished.
#[test]
fn biology_kingdoms_declines_the_example_its_page_does_not_state() {
    let out = assert_kingdom_rows_cite_their_own_line(
        "kingdomsdropped",
        "plantae",
        "Examples: Flowers, grasses, conifers, ferns, mosses",
        5,
    );
    for stated in ["flowers", "grasses", "conifers", "ferns", "mosses"] {
        assert!(
            out.contains(&format!("\"Example\":\"{stated}\"")),
            "plantae still recalls {stated}: {out}"
        );
    }
    assert!(
        !out.contains("multicellular"),
        "no example the cited page does not state: {out}"
    );
}

/// The other three kingdoms. Without these, 14 of the 22 rows had NO per-row
/// pin: stripping every animalia or bacteria `{ source }` block — reverting
/// them to the envelope, which is the exact defect #14986 exists to fix — left
/// the whole suite green, because the pre-existing test checks only
/// `contains("sciencenotes.org")`, the loose form a mutant already defeated.
#[test]
fn biology_kingdoms_animalia_rows_cite_their_own_line() {
    assert_kingdom_rows_cite_their_own_line(
        "kingdomsanimalia",
        "animalia",
        "Examples: Humans, birds, crustaceans, sponges",
        4,
    );
}

#[test]
fn biology_kingdoms_protista_rows_cite_their_own_line() {
    assert_kingdom_rows_cite_their_own_line(
        "kingdomsprotista",
        "protista",
        "Examples: Amoebas, diatoms, dinoflagellates, ciliates, slime molds, single-celled algae",
        6,
    );
}

#[test]
fn biology_kingdoms_bacteria_rows_cite_their_own_line() {
    let out = assert_kingdom_rows_cite_their_own_line(
        "kingdomsbacteria",
        "bacteria",
        "Examples: Gram-positive and Gram-negative bacteria, cyanobacteria, actinobacteria",
        4,
    );
    // The envelope's `cites` is what makes `bacteria` a kingdom key at all --
    // the five-kingdom framing span names Monera, not Bacteria. Deleting that
    // `cites` left the whole suite green until this assertion existed: the
    // object pin above stops at `"trust"`, before `corroborations`.
    assert!(
        out.contains("The six-kingdom system separates Monera into Bacteria and Archaea."),
        "the span that warrants `bacteria` AS A KEY reaches the answer: {out}"
    );
}

/// Row blocks, parsed the way the grammar writes them: `{ ... }` may sit
/// inline on the `row (...)` line or open a multi-line block. Returns one
/// `(key, spans, locator_overrides, trust_overrides)` tuple per row.
///
/// AN EIGHT-SPACE `strip_prefix` WOULD READ NOTHING FROM AN INLINE BLOCK and
/// would not fail — it would return an empty list and every assertion over
/// it would pass. That is exactly how three tables came to be miscounted in
/// the #14986 census, so this parser handles both forms and every caller
/// asserts the parse found what it expected before asserting anything else.
#[allow(clippy::type_complexity)]
fn parse_rows(adj: &str) -> Vec<(String, Vec<String>, Vec<String>, Vec<String>)> {
    fn quoted_after(hay: &str, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = hay;
        while let Some(at) = rest.find(key) {
            rest = &rest[at + key.len()..];
            let rest_trim = rest.trim_start();
            if !rest_trim.starts_with('"') {
                continue;
            }
            let body = &rest_trim[1..];
            if let Some(end) = body.find('"') {
                out.push(body[..end].to_string());
                rest = &body[end + 1..];
            } else {
                break;
            }
        }
        out
    }
    fn bare_after(hay: &str, key: &str) -> Vec<String> {
        hay.split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] == key)
            .map(|w| w[1].trim_end_matches('}').trim().to_string())
            .collect()
    }

    let mut rows = Vec::new();
    let lines: Vec<&str> = adj.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let Some(rest) = line.strip_prefix("    row (") else {
            i += 1;
            continue;
        };
        // QUOTE-AWARE. A row item may be a quoted string and a span may
        // contain `)` or `}` -- `row (length, meter, "m") { source "Length -
        // meter (m)" }` broke a naive `split(')')`, which ended the tuple
        // inside the span and swallowed the rest of the table into one row.
        let close = {
            let b = rest.as_bytes();
            let mut in_q = false;
            let mut at = None;
            for (n, &c) in b.iter().enumerate() {
                match c {
                    b'"' => in_q = !in_q,
                    b')' if !in_q => {
                        at = Some(n);
                        break;
                    }
                    _ => {}
                }
            }
            at.unwrap_or(rest.len())
        };
        let key = rest[..close]
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let after = if close < rest.len() {
            &rest[close + 1..]
        } else {
            ""
        };
        if !after.trim_start().starts_with('{') {
            rows.push((key, Vec::new(), Vec::new(), Vec::new()));
            i += 1;
            continue;
        }
        let closes_here = {
            let b = after.as_bytes();
            let mut in_q = false;
            let mut found = false;
            for &c in b {
                match c {
                    b'"' => in_q = !in_q,
                    b'}' if !in_q => {
                        found = true;
                        break;
                    }
                    _ => {}
                }
            }
            found
        };
        let mut body = String::new();
        if closes_here {
            body.push_str(after);
            i += 1;
        } else {
            body.push_str(after);
            i += 1;
            while i < lines.len() && !lines[i].starts_with("    }") {
                body.push('\n');
                body.push_str(lines[i]);
                i += 1;
            }
            i += 1;
        }
        // `cites "..." locator "..."` carries a MANDATORY locator that is the
        // corroboration's own address, not an override of the row's — so the
        // locator list drops any that follows a `cites`.
        let cites_locs = {
            let mut out = Vec::new();
            let mut rest = body.as_str();
            while let Some(at) = rest.find("cites ") {
                rest = &rest[at + 6..];
                for l in quoted_after(rest, "locator").into_iter().take(1) {
                    out.push(l);
                }
                match rest.find('\n') {
                    Some(n) => rest = &rest[n..],
                    None => break,
                }
            }
            out
        };
        let mut locs = quoted_after(&body, "locator");
        for c in &cites_locs {
            if let Some(p) = locs.iter().position(|x| x == c) {
                locs.remove(p);
            }
        }
        rows.push((
            key,
            quoted_after(&body, "source"),
            locs,
            bare_after(&body, "trust"),
        ));
    }
    rows
}

/// The table envelope, derived totally rather than positionally: exactly one
/// four-space `source` and one four-space `locator`.
fn envelope(adj: &str) -> (String, String) {
    let pick = |kw: &str| -> String {
        let hits: Vec<&str> = adj
            .lines()
            .filter(|l| l.split_whitespace().next() == Some(kw))
            .filter(|l| l.starts_with("    ") && !l.starts_with("     "))
            .collect();
        assert_eq!(hits.len(), 1, "exactly one table-level {kw}: {hits:?}");
        hits[0]
            .trim()
            .trim_start_matches(kw)
            .trim()
            .trim_matches(0x22 as char)
            .to_string()
    };
    (pick("source"), pick("locator"))
}

#[test]
fn kingdoms_asserts_its_own_span_structure() {
    // STRUCTURAL, READ FROM THE SHIPPED FILE. #15193 recorded that only 2 of
    // 17 converted tables asserted their own span structure; re-measured with
    // an inline-aware parser it was 21 of 26, and this table was one of the
    // five without.
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/kingdoms.adj"))
        .expect("read shipped kingdoms.adj");
    let rows = parse_rows(&adj);
    let (env_source, env_locator) = envelope(&adj);
    let _ = (&env_source, &env_locator);

    // THE PARSE FOUND WHAT IT EXPECTED, asserted BEFORE anything is asserted
    // about the contents. A parser that silently read nothing would make
    // every check below pass over an empty list.
    assert_eq!(rows.len(), 22, "22 rows");
    let spans: Vec<String> = rows.iter().flat_map(|r| r.1.clone()).collect();
    assert_eq!(spans.len(), 22, "22 row spans: {spans:?}");

    let mut counts: std::collections::BTreeMap<&String, usize> =
        std::collections::BTreeMap::new();
    for s in &spans {
        *counts.entry(s).or_insert(0) += 1;
    }
    assert_eq!(counts.len(), 5, "5 distinct spans: {counts:?}");
    let mut sizes: Vec<usize> = counts.values().copied().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, vec![6, 5, 4, 4, 3], "the span multiset: {counts:?}");

    let locators: Vec<String> = rows.iter().flat_map(|r| r.2.clone()).collect();
    assert_eq!(locators.len(), 0, "row locator overrides: {locators:?}");

    assert!(adj.contains("    columns kingdom, example"), "the shipped column names are unchanged");

    // TWENTY-TWO ROWS OVER FIVE SPANS, and the grouping is the structure
    // worth pinning: each kingdom's examples all cite that kingdom's own
    // "Examples: ..." sentence, so the span counts are the kingdom sizes.
    // Every block here is INLINE, so an eight-space needle reads nothing.
    let mut by_span: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (key, spans, _, _) in &rows {
        assert_eq!(spans.len(), 1, "{key} carries exactly one span");
        by_span.entry(spans[0].clone()).or_default().push(key.clone());
    }
    assert_eq!(by_span.len(), 5, "five kingdoms' worth of spans");
    for (span, members) in &by_span {
        let mut uniq = members.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(
            uniq.len(),
            1,
            "every row sharing a span is the same kingdom: {span:?} -> {members:?}"
        );
    }
}
