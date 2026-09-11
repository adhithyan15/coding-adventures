//! End-to-end test for the metrology TIME-UNITS facts library
//! (`adj-facts-stdlib/metrology/time-units.adj`) driven through the built CLI:
//! a native `table` of time-unit → length-in-seconds resolves a binding-query
//! recall with the NIST citation, and abstains on a non-listed unit — 0 model
//! calls. A recalled length (e.g. hour → 3600 s) is the number that flows into
//! duration arithmetic.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstu_{tag}_{}", std::process::id()));
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

/// The `columns` clauses of an ADJ source, with `%` comments removed.
///
/// Comments are stripped because the file's author writes those too: a security
/// review hid the shipped clause in a comment and put a PERMUTED one in its
/// place, and a raw `contains` passed. Rows are positional, so a permutation
/// changes no recall output at all -- but `lookup ... give <col>` resolves the
/// names by position (`adj-lang/src/lower.rs`), so the consumer's query flips,
/// still carrying the genuine citation.
///
/// This is deliberately the ONLY lexical check left. Counting declaration
/// keywords was tried twice and broken twice -- `row(week, 604800)` past a
/// prefix count, then `86400rule {` past a whole-word count, because ADJ's
/// NUMBER and IDENT are different tokens and may abut. What an `.adj` file is
/// allowed to CONTAIN is a property of the language, enforceable only by
/// lexing with the real grammar; that is #14822, not this test's job. A
/// `columns` clause, by contrast, is bounded and checkable as text.
fn columns_clauses(adj: &str) -> Vec<String> {
    adj.lines()
        .map(|l| match l.find('%') {
            Some(i) => &l[..i],
            None => l,
        })
        .map(str::trim)
        .filter(|l| {
            l.strip_prefix("columns")
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn metrology_time_units_recall_binds_seconds_with_citation() {
    let dir = scratch("timeunits");
    // Copy the shipped metrology table beside the entry program and import it.
    let src = facts_stdlib().join("metrology/time-units.adj");
    std::fs::copy(&src, dir.join("time-units.adj")).expect("copy shipped time-units.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-units.adj\"\n\
         ? time_unit_seconds($U, $S)\n\
         ? time_unit_seconds(fortnight, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A minute is 60 seconds; an hour is 3600; a day is 86400 — the recalled
    // lengths that feed duration arithmetic.
    //
    // The first query is deliberately UNBOUND: `time_unit_seconds($U, $S)` makes
    // the engine enumerate every row the library can serve, so the answer set
    // below is the whole table rather than the rows a per-unit query happened to
    // ask for. Two security-review rounds landed here. One shipped
    // `row (minute, 7)` green while only hour and day were asked for; the next
    // shipped `row(week, 604800)` green — one space deleted — past a guard that
    // counted rows by line prefix. A row hides from a lexical count; it cannot
    // hide from an enumeration, because it either answers or it does not exist.
    assert!(out.contains("\"S\":\"60\""), "minute -> 60 s: {out}");
    assert!(out.contains("\"S\":\"3600\""), "hour -> 3600 s: {out}");
    assert!(out.contains("\"S\":\"86400\""), "day -> 86400 s: {out}");
    // The answer carries the NIST citation as its proof. Pinned as the WHOLE
    // serialised citation object, closing on the corroborations `]`, not as a
    // hostname plus a trust tier: the weaker pin cannot see the `source` field,
    // so it would stay green if the quoted cells were altered or replaced. The
    // object is bounded by the next delimiter, never by a character count.
    assert!(
        out.contains(
            "\"citations\":[{\"source\":\"1 min = 60 s | 1 h = 60 min = 3600 s | 1 d = 24 h = 86 400s\",\"locator\":\"https://www.nist.gov/pml/special-publication-811/nist-guide-si-chapter-5-units-outside-si\",\"trust\":\"authoritative\",\"corroborations\":[]}"
        ),
        "carries the whole NIST citation object, source cells included: {out}"
    );
    // NIST writes the day cell `86 400s`, with NO space before the s. This file
    // quoted it with one until #13934. Named here so the repair is load-bearing:
    // restore the space and this assertion is what reddens.
    assert!(
        !out.contains("86 400 s"),
        "the day cell keeps the page's spelling, `86 400s`: {out}"
    );

    // EXCLUSIVITY, not just presence. The pin above proves the NIST citation IS
    // in the output; it does not prove it is the ONLY one. Append a second
    // `table` plus a `rule` whose head aliases this relation and the library
    // emits a SIBLING answer carrying its own citation, under any address the
    // author likes, while a presence pin stays green throughout.
    //
    // Every needle below is KEY-ANCHORED -- `"locator":"<url>"`, not the bare
    // URL. A first draft counted the bare value and a security review broke it
    // by embedding the NIST URL and the shipped span INSIDE the fabricated
    // clause's own `source` string: both sides of the equality then moved
    // together -- both counts rose in step -- and the test stayed green
    // while the output carried a fabricated answer citing evil.example at
    // `trust authoritative`. A free substring search is not an exclusivity check.
    let loc_pair = "\"locator\":\"https://www.nist.gov/pml/special-publication-811/nist-guide-si-chapter-5-units-outside-si\"";
    let src_pair = "\"source\":\"1 min = 60 s | 1 h = 60 min = 3600 s | 1 d = 24 h = 86 400s\"";
    assert_eq!(
        out.matches("\"locator\":\"").count(),
        out.matches(loc_pair).count(),
        "every locator THIS QUERY emits is the NIST one -- no second address: {out}"
    );
    assert_eq!(
        out.matches("\"source\":\"").count(),
        out.matches(src_pair).count(),
        "every source THIS QUERY emits is the shipped span -- none fabricated: {out}"
    );
    assert_eq!(
        out.matches("\"trust\":\"").count(),
        out.matches("\"trust\":\"authoritative\"").count(),
        "every trust tier THIS QUERY emits is the shipped one: {out}"
    );

    // ...and the ANSWER SET, because those three equalities are satisfied BY
    // CONSTRUCTION if an injected clause re-uses the genuine citation triple
    // verbatim -- the fabrication then wears the real NIST citation and the
    // counts stay balanced. What they close is an answer citing a SECOND
    // source, locator or trust tier; what closes a fabricated BINDING under the
    // genuine one is naming the whole answer set and its size.
    assert_eq!(
        out.matches("\"term\":\"").count(),
        3,
        "exactly 3 facts answer -- every shipped row, and nothing else: {out}"
    );
    assert!(
        out.contains("\"term\":\"time_unit_seconds(minute, 60)\""),
        "time_unit_seconds answers with the shipped row: {out}"
    );
    assert!(
        out.contains("\"term\":\"time_unit_seconds(hour, 3600)\""),
        "time_unit_seconds answers with the shipped row: {out}"
    );
    assert!(
        out.contains("\"term\":\"time_unit_seconds(day, 86400)\""),
        "time_unit_seconds answers with the shipped row: {out}"
    );

    // ...and the `columns` clause of the shipped file, which nothing above can
    // see: the names never reach the recall output, and `lookup ... give <col>`
    // resolves against them POSITIONALLY, so a permutation hands a consumer the
    // right number under the wrong label while every assertion here stays green.
    //
    // This is the only check here that reads the file rather than the answer.
    // A wider one -- "and the file contains nothing else" -- was tried twice and
    // broken twice, and is NOT claimed: an injected `rule`, `relate` or `quote`
    // pin is invisible to this test. That gap is #14822.
    let lib = std::fs::read_to_string(dir.join("time-units.adj"))
        .expect("read back the copied library");
    assert_eq!(
        columns_clauses(&lib),
        vec!["columns unit, seconds".to_string()],
        "one columns clause, exactly as shipped and in order: {lib}"
    );

    // A "fortnight" is not in this table — honest abstention, never a fabricated
    // length.
    assert!(out.contains("\"abstained\":true"), "fortnight abstains: {out}");
}
