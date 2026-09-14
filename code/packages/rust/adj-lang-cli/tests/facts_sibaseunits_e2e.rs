//! End-to-end test for the metrology SI-BASE-UNITS facts library
//! (`adj-facts-stdlib/metrology/si-base-units.adj`) driven through the built CLI:
//! a native `table` of base-quantity → unit → symbol resolves a binding-query
//! recall with the NIST citation, runs the relation backwards (unit → quantity),
//! and abstains on anything that is not one of the seven base quantities — 0
//! model calls, never a fabricated unit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssi_{tag}_{}", std::process::id()));
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

fn with_case(dir: &Path, body: &str) -> PathBuf {
    let src = facts_stdlib().join("metrology/si-base-units.adj");
    std::fs::copy(&src, dir.join("si-base-units.adj")).expect("copy shipped si-base-units.adj");
    let p = dir.join("case.adj");
    std::fs::write(&p, format!("import \"si-base-units.adj\"\n{body}")).unwrap();
    p
}

#[test]
fn si_base_unit_forward_recall_binds_unit_and_symbol_with_citation() {
    let dir = scratch("forward");
    let p = with_case(
        &dir,
        "? si_base_unit(mass, $Unit, $Symbol)\n\
         ? si_base_unit(temperature, $Unit, $Symbol)\n",
    );

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Mass -> kilogram (kg); temperature -> kelvin (K).
    assert!(
        out.contains("\"Unit\":\"kilogram\""),
        "mass binds the kilogram: {out}"
    );
    assert!(out.contains("kg"), "mass carries the symbol kg: {out}");
    assert!(
        out.contains("\"Unit\":\"kelvin\""),
        "temperature binds the kelvin: {out}"
    );
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation: {out}"
    );
}

#[test]
fn si_base_unit_runs_backwards_from_unit_to_quantity() {
    let dir = scratch("reverse");
    // Given the unit `second`, recall the base quantity it measures — time.
    let p = with_case(&dir, "? si_base_unit($Quantity, second, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Quantity\":\"time\""),
        "the second measures time (reverse lookup): {out}"
    );
}

#[test]
fn a_non_base_quantity_abstains_rather_than_inventing_a_unit() {
    let dir = scratch("abstain");
    // Luminance is a real photometric quantity but NOT one of the seven SI base
    // quantities — the table must abstain, never fabricate a unit.
    let p = with_case(&dir, "? si_base_unit(luminance, $Unit, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a non-base quantity abstains: {out}"
    );
}

/// The envelope span — what defends the TABLE, not any one row. It states that
/// the SI has seven base units and names none of them, so it must no longer be
/// the warrant on a row answer.
const SI_ENVELOPE_SPAN: &str = r#""source":"The SI is made up of 7 base units that define the 22 derived units with special names and symbols, which are illustrated in NIST SP 1247, SI Base Units Relationship Poster.""#;

/// #14986: every row of this table used to answer with the envelope sentence,
/// which names none of the seven units. A recall of `mass` shipped NIST, an
/// `authoritative` tier, and a sentence that does not contain "kilogram".
///
/// The previous version of this test pinned that envelope on a row query and
/// said so in its own comment: *"a row-binding pin today would freeze the
/// #14124 defect into a test"*, and it recorded all seven per-row spans as
/// present on the page, *"Deferred to #14124 with the text recorded, NOT
/// unavailable."* Those spans are now shipped as RS-5e per-row `source`s, so
/// the sound row-binding pin that comment was waiting for is available, and
/// this is it.
fn assert_row_carries_its_own_span(tag: &str, query: &str, binding: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("metrology/si-base-units.adj"),
        dir.join("si-base-units.adj"),
    )
    .expect("copy shipped si-base-units.adj");
    let case = with_case(&dir, query);
    let (ok, out) = run(&case);
    assert!(ok, "cli should succeed: {out}");

    // The row resolves. A provenance assertion over a row the engine never
    // reaches proves nothing.
    assert!(out.contains(binding), "query binds {binding}: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to THIS row: {out}"
    );

    // The row's own span, key-anchored to `source` so a loose substring
    // elsewhere in the output cannot satisfy it. Each of these occurs EXACTLY
    // ONCE on the NIST page — in the raw HTML, under a block-only extractor,
    // and under a crude every-tag-is-a-break one. The separator is SPACE,
    // U+002D HYPHEN-MINUS, SPACE, read out of the page rather than assumed;
    // an en-dash variant and a no-spaces variant each occur zero times.
    assert!(
        out.contains(&format!("\"source\":\"{span}\"")),
        "row is warranted by the page's own line for it ({span}): {out}"
    );

    // `locator` and `trust` are NOT restated per row — all seven rows come
    // from the one NIST page at one tier, and a row inherits every field it
    // does not write.
    //
    // The `locator` pin is load-bearing: deleting the envelope's `locator`
    // reddens this.
    assert!(
        out.contains("\"locator\":\"https://www.nist.gov/pml/owm/metric-si/si-units\""),
        "row inherits the envelope locator: {out}"
    );
    // The `trust` pin is WEAKER THAN IT LOOKS, and saying so is the honest
    // version. `annotations_to_provenance` (adj-lang/src/lower.rs:2622)
    // defaults a tier to Authoritative whenever a `source` is present, so this
    // cannot distinguish "inherited from the envelope's declared tier" from
    // "silently defaulted because the row has a source" — deleting the
    // envelope's `trust authoritative` leaves this GREEN. What it does
    // discriminate is the other four tiers: setting the envelope to
    // `trust inferred` propagates and reddens it.
    //
    // It is kept because `authoritative` is the correct tier here for a reason
    // worth pinning: the span STATES the row, so the claim is read, not
    // reasoned.
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "tier is authoritative — the span STATES the row, read not reasoned: {out}"
    );

    // NAMED NEGATIVE, and the whole point of #14986: the framing sentence is
    // no longer any row's warrant.
    assert!(
        !out.contains(SI_ENVELOPE_SPAN),
        "the framing sentence does not warrant a row: {out}"
    );
}

#[test]
fn si_base_unit_mass_row_carries_the_pages_own_line() {
    assert_row_carries_its_own_span(
        "sibaseunitsmass",
        "? si_base_unit(mass, $Unit, $Symbol)",
        "\"Unit\":\"kilogram\"",
        "Mass - kilogram (kg)",
    );
}

#[test]
fn si_base_unit_candela_row_carries_the_pages_own_line_in_reverse() {
    assert_row_carries_its_own_span(
        "sibaseunitscandela",
        "? si_base_unit($Quantity, candela, $Symbol)",
        "\"Quantity\":\"luminous_intensity\"",
        "Luminous intensity - candela (cd)",
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
fn si_base_units_asserts_its_own_span_structure() {
    // STRUCTURAL, READ FROM THE SHIPPED FILE. #15193 recorded that only 2 of
    // 17 converted tables asserted their own span structure; re-measured with
    // an inline-aware parser it was 21 of 26, and this table was one of the
    // five without.
    let adj = std::fs::read_to_string(facts_stdlib().join("metrology/si-base-units.adj"))
        .expect("read shipped si-base-units.adj");
    let rows = parse_rows(&adj);
    let (env_source, env_locator) = envelope(&adj);
    let _ = (&env_source, &env_locator);

    // THE PARSE FOUND WHAT IT EXPECTED, asserted BEFORE anything is asserted
    // about the contents. A parser that silently read nothing would make
    // every check below pass over an empty list.
    assert_eq!(rows.len(), 7, "7 rows");
    let spans: Vec<String> = rows.iter().flat_map(|r| r.1.clone()).collect();
    assert_eq!(spans.len(), 7, "7 row spans: {spans:?}");

    let mut counts: std::collections::BTreeMap<&String, usize> =
        std::collections::BTreeMap::new();
    for s in &spans {
        *counts.entry(s).or_insert(0) += 1;
    }
    assert_eq!(counts.len(), 7, "7 distinct spans: {counts:?}");
    let mut sizes: Vec<usize> = counts.values().copied().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, vec![1, 1, 1, 1, 1, 1, 1], "the span multiset: {counts:?}");

    let locators: Vec<String> = rows.iter().flat_map(|r| r.2.clone()).collect();
    assert_eq!(locators.len(), 0, "row locator overrides: {locators:?}");

    assert!(adj.contains("    columns quantity, unit, symbol"), "the shipped column names are unchanged");

    // ZERO ROW LOCATORS: all seven spans are on the one NIST page the
    // envelope names, which is what ADJ-TABLES.md §4 says to do. Asserting
    // the zero is what makes "this table is single-page" a checked fact
    // rather than something a reader establishes by comparing URLs.
    let locs: Vec<String> = rows.iter().flat_map(|r| r.2.clone()).collect();
    assert!(locs.is_empty(), "no row restates the locator: {locs:?}");

    // THE SEVEN SPANS, PINNED BY VALUE. A mutant that truncated ONE inline
    // span survived everything above: the count stayed 7 and they stayed
    // distinct, and only the mass and candela rows are pinned by content
    // anywhere in this file. These spans are short enough that a truncation
    // is easy to miss and cheap to ship, so the set is pinned outright --
    // one side of the comparison is the shipped file, the other is this
    // literal, so they cannot drift into agreeing with each other.
    let mut got: Vec<&str> = spans.iter().map(|s| s.as_str()).collect();
    got.sort_unstable();
    assert_eq!(
        got,
        vec![
            "Amount of substance - mole (mol)",
            "Electric current - ampere (A)",
            "Length - meter (m)",
            "Luminous intensity - candela (cd)",
            "Mass - kilogram (kg)",
            "Temperature - kelvin (K)",
            "Time - second (s)",
        ],
        "the page's own seven lines, verbatim"
    );
}
