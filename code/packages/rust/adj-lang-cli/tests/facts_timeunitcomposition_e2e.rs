//! End-to-end test for the metrology FACTS library
//! (`adj-facts-stdlib/metrology/time-unit-composition.adj`) driven through
//! the built CLI: a native `table` naming the unit-to-unit composition the
//! SAME NIST source span already states for two time units -- a sibling to
//! the already-shipped `time-units.adj` (which only carries each unit's
//! length in seconds, not a unit-to-unit relation), decoding the
//! composition half of a span already sitting unused inside that table's
//! own `source` field. Resolves binding-query recall (both directions)
//! with the source's citation, and abstains on a unit (minute) the cited
//! span gives no unit-to-unit composition for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_timeunitcomposition_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("metrology/time-unit-composition.adj");
    std::fs::copy(&src, dir.join("time-unit-composition.adj"))
        .expect("copy shipped time-unit-composition.adj");
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
fn time_unit_composition_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition($U, $SubUnit, $Count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The query above is deliberately UNBOUND, so the engine enumerates every
    // row the library can serve and the answer set below is the whole table.
    // Two security-review rounds landed here: one shipped `row (day, hour, 99)`
    // green while only the hour row was asked for, under the genuine NIST
    // citation; the next shipped `row(week, day, 7)` green — one space deleted —
    // past a guard that counted rows by line prefix. A row hides from a lexical
    // count; it cannot hide from an enumeration.
    assert!(
        out.contains("\"term\":\"time_unit_composition(hour, minute, 60)\""),
        "an hour is 60 minutes: {out}"
    );
    assert!(
        out.contains("\"term\":\"time_unit_composition(day, hour, 24)\""),
        "a day is 24 hours: {out}"
    );
    // The NIST citation, pinned as the WHOLE serialised object closing on the
    // corroborations `]`. A hostname-plus-trust-tier pin cannot see the `source`
    // field, so it would stay green if the quoted cells were altered.
    assert!(
        out.contains(
            "\"citations\":[{\"source\":\"1 h = 60 min = 3600 s | 1 d = 24 h = 86 400s\",\"locator\":\"https://www.nist.gov/pml/special-publication-811/nist-guide-si-chapter-5-units-outside-si\",\"trust\":\"authoritative\",\"corroborations\":[]}"
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
    let src_pair = "\"source\":\"1 h = 60 min = 3600 s | 1 d = 24 h = 86 400s\"";
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
        2,
        "exactly 2 facts answer -- every shipped row, and nothing else: {out}"
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
    let lib = std::fs::read_to_string(dir.join("time-unit-composition.adj"))
        .expect("read back the copied library");
    assert_eq!(
        columns_clauses(&lib),
        vec!["columns unit, sub_unit, count".to_string()],
        "one columns clause, exactly as shipped and in order: {lib}"
    );

}

#[test]
fn time_unit_composition_recalls_backward_from_a_bound_sub_unit_and_count() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition($Unit, minute, 60)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"time_unit_composition(hour, minute, 60)\""),
        "60 minutes composes an hour: {out}"
    );
}

#[test]
fn time_unit_composition_abstains_honestly_on_minute() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-unit-composition.adj\"\n\
         ? time_unit_composition(minute, $SubUnit, $Count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "minute has no unit-to-unit composition in the cited span -- honest abstention: {out}"
    );
}
