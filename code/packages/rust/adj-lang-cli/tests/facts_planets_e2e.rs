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
            // ESCAPE-AWARE. Taking the next `"` truncates a span at its
            // first `\\"` and inverts the quote state for the rest of the
            // line. No span in this file carries one, but
            // `anatomy/brain-parts.adj` does, and this parser is the kind of
            // thing that gets copied.
            let body = &rest_trim[1..];
            let mut end = None;
            let mut esc = false;
            for (n, &c) in body.as_bytes().iter().enumerate() {
                if esc {
                    esc = false;
                    continue;
                }
                match c {
                    b'\\' => esc = true,
                    b'"' => {
                        end = Some(n);
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(end) = end {
                out.push(body[..end].to_string());
                rest = &body[end + 1..];
            } else {
                break;
            }
        }
        out
    }
    fn bare_after(hay: &str, key: &str) -> Vec<String> {
        // `%` COMMENT LINES ARE SKIPPED. Scanning the whole body picks the
        // keyword out of prose: one block here discusses `trust inferred` in
        // a comment and is missed only because a backtick makes the token
        // `` `trust ``. Deleting those backticks would start failing this
        // test on an unchanged fact.
        hay.lines()
            .filter(|l| !l.trim_start().starts_with('%'))
            .flat_map(|l| {
                l.split_whitespace()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .filter(|w| w[0] == key)
                    .map(|w| w[1].trim_end_matches('}').trim().to_string())
                    .collect::<Vec<_>>()
            })
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
            let mut esc = false;
            let mut at = None;
            for (n, &c) in b.iter().enumerate() {
                if esc {
                    esc = false;
                    continue;
                }
                match c {
                    b'\\' if in_q => esc = true,
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
            let mut esc = false;
            let mut found = false;
            for &c in b {
                if esc {
                    esc = false;
                    continue;
                }
                match c {
                    b'\\' if in_q => esc = true,
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
            // BOUNDED BY THE NEXT NEWLINE, not by "the rest of the body".
            // Searching the remainder means a `cites` written without its
            // mandatory locator would steal the NEXT line's locator -- which
            // could be the row's own override.
            while let Some(at) = rest.find("cites ") {
                rest = &rest[at + 6..];
                let line_end = rest.find('\n').unwrap_or(rest.len());
                for l in quoted_after(&rest[..line_end], "locator")
                    .into_iter()
                    .take(1)
                {
                    out.push(l);
                }
                if line_end >= rest.len() {
                    break;
                }
                rest = &rest[line_end..];
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
fn planet_order_asserts_its_own_span_and_trust_structure() {
    // STRUCTURAL, READ FROM THE SHIPPED FILE. #15193 recorded that only 2 of
    // 17 converted tables asserted their own span structure; re-measured with
    // an inline-aware parser it was 21 of 26, and this table was one of the
    // five without.
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/planets.adj"))
        .expect("read shipped planets.adj");
    let rows = parse_rows(&adj);
    let (env_source, env_locator) = envelope(&adj);
    let _ = (&env_source, &env_locator);

    // THE PARSE FOUND WHAT IT EXPECTED, asserted BEFORE anything is asserted
    // about the contents. A parser that silently read nothing would make
    // every check below pass over an empty list.
    assert_eq!(rows.len(), 8, "8 rows");
    let spans: Vec<String> = rows.iter().flat_map(|r| r.1.clone()).collect();
    assert_eq!(spans.len(), 8, "8 row spans: {spans:?}");

    let mut counts: std::collections::BTreeMap<&String, usize> =
        std::collections::BTreeMap::new();
    for s in &spans {
        *counts.entry(s).or_insert(0) += 1;
    }
    assert_eq!(counts.len(), 8, "8 distinct spans: {counts:?}");
    let mut sizes: Vec<usize> = counts.values().copied().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, vec![1, 1, 1, 1, 1, 1, 1, 1], "the span multiset: {counts:?}");

    let locators: Vec<String> = rows.iter().flat_map(|r| r.2.clone()).collect();
    assert_eq!(locators.len(), 0, "row locator overrides: {locators:?}");

    assert!(adj.contains("    columns planet, order_from_sun"), "the shipped column names are unchanged");

    // SEVEN OF THE EIGHT ROW BLOCKS ARE WRITTEN INLINE, which is why this
    // test parses them properly rather than with an eight-space prefix: that
    // needle reads ZERO spans here and would make every assertion above pass
    // over an empty list.
    // COUNT THE INLINE ROW BLOCKS, not every `}` in the file. This was
    // `assert_eq!(adj.matches("}").count() >= 7, true, ...)`, which was two
    // defects at once: it is a clippy `bool_assert_comparison` error under
    // the repo's `cargo clippy --all-targets -- -D warnings` gate (a green
    // `cargo test` says nothing about that), and it was INERT -- the file has
    // nine `}`, and reflowing all eight rows to multi-line, the exact
    // regression the message claims to guard, still leaves nine.
    let inline_rows = adj
        .lines()
        .filter(|l| l.starts_with("    row (") && l.contains('}'))
        .count();
    assert_eq!(
        inline_rows, 7,
        "seven of the eight row blocks are written inline; if they were \
         reflowed this parser must be re-read rather than silently still \
         passing"
    );

    // ONE ROW IS AT A DIFFERENT TRUST TIER, and it is the one the page does
    // not state literally. Mercury's position is read off "nearest to the
    // Sun" plus the ordering of a second span, so it ships `trust inferred`
    // while its seven siblings inherit `authoritative` from the envelope.
    let tiers: Vec<(String, Vec<String>)> =
        rows.iter().map(|r| (r.0.clone(), r.3.clone())).collect();
    let overridden: Vec<&(String, Vec<String>)> =
        tiers.iter().filter(|(_, t)| !t.is_empty()).collect();
    assert_eq!(
        overridden.len(),
        1,
        "exactly one row overrides trust: {overridden:?}"
    );
    assert_eq!(overridden[0].0, "mercury", "and it is mercury: {overridden:?}");
    assert_eq!(
        overridden[0].1,
        vec!["inferred".to_string()],
        "at the inferred tier, because that row is read off rather than stated"
    );
}
