//! # `cost`: which rows the spot check may be asked to finish, decided by the PLAN
//!
//! `check`'s sampled re-run (VOS30 section 9.12) has a fixed time budget, so it must not try a
//! row that takes minutes.  Before this module the test was "recorded `solve_ms + check_ms` over
//! 20 s", which reads two fields of the very file under review: an editor who wanted a row left
//! alone had only to write a large number there (finding M4 of the VOS-028c2 security review).
//!
//! The class is now a property of the **plan**: the row's `(experiment, mode, knob)`, never of
//! anything the file says about the row.  [`CHEAP`] lists the knob groups a spot check may
//! re-run; every other planned group is expensive and is skipped (counted and printed).
//!
//! ## How the table is derived, and what it is not
//!
//! The entries are exactly the knob groups of the full tier whose *slowest seed*, in the committed
//! recording, took at most [`CHEAP_MAX_MS`] of solve plus check.  The recording is the CI one
//! (run 45486250172, 2,225 rows on GitHub `ubuntu-24.04` runners, committed as
//! `code/specs/data/vos30-capacity-results.json`).  [`slowest_seeds`] and [`derive_cheap`] are the
//! rule, written once: a group is every non-E full-tier row sharing `(experiment, mode, knob)`;
//! its figure is the slowest seed's `solve_ms + check_ms`, and a seed that ended `killed` or `oom`
//! has no finished time and counts as infinitely slow (so does `crashed`, and any non-finite time).  The figure after each entry below is that
//! slowest seed in the CI recording.
//!
//! History (VOS-D143): the first table was derived from the Mac recording of VOS-D095 and so
//! lacked the groups registered afterwards (B3 `every2`/`every4`, D5 and B5), and its exact test
//! had to be loosened to "at most 4x the line" when the recording moved to CI.  The VOS-028c3c-3b-1c
//! re-derivation from the CI recording added 36 groups (B3 `every2`/`every4` and four D5 groups)
//! and removed 14 whose slowest seed is over the line on the CI hosts (B2 25 instructions 25%;
//! B3 certified 25/1.0/`every8`, 500/0.25/`every1`; B3 incremental 100/0.05/`every8`,
//! 100/1.0/`every1`, 25/0.05/`every32`; D1 T=4 k=4; D2 T=256 and T=64 k=16; D3 split T=16 k=4;
//! D4 certified T=4 k=8 and T=8 k=4; D4 incremental T=16 k=2 and T=8 k=4).  No B5 group is
//! cheap.
//!
//! It is a reviewed constant: a re-recording that moves a group across the line is a change to
//! this table in the same commit, and the test `cost_table_is_exactly_the_committed_recordings_cheap_set`
//! fails until it is.  It is **not** read from the results file at check time, and it is not a
//! security boundary by itself: it only decides which rows the (budget-bounded) spot check may be
//! asked to finish.  A row in a cheap group that nevertheless does not finish inside the budget
//! is a `check` failure, not a skip (finding M5), so a wrong entry fails loudly.

/// The slowest seed, in milliseconds of solve plus check, that a group may have in the
/// committed CI recording to be listed in [`CHEAP`].
pub const CHEAP_MAX_MS: f64 = 3_000.0;

/// `(experiment, mode, knob key)` of the knob groups the spot check may re-run.
pub const CHEAP: &[(&str, &str, &str)] = &[
    ("A1", "monolithic", "{\"n\":16}"), // 2 ms
    ("A1", "monolithic", "{\"n\":32}"), // 4 ms
    ("A1", "monolithic", "{\"n\":64}"), // 10 ms
    ("A2", "monolithic", "{\"n\":16}"), // 4 ms
    ("A2", "monolithic", "{\"n\":32}"), // 5 ms
    ("A2", "monolithic", "{\"n\":64}"), // 13 ms
    ("B1", "monolithic", "{\"length\":1000,\"divergence\":0.0}"), // 0 ms
    ("B1", "monolithic", "{\"length\":500,\"divergence\":0.0}"), // 0 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 578 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 1164 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every4\"}",
    ), // 1593 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 726 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 1155 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 1664 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":1.0,\"cut\":\"every2\"}",
    ), // 2241 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 1149 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 1834 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 1421 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 2293 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 2480 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 352 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every4\"}",
    ), // 993 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 1576 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 211 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 511 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every4\"}",
    ), // 788 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 2366 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 350 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every2\"}",
    ), // 940 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every4\"}",
    ), // 1205 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 419 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 697 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every4\"}",
    ), // 1028 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 2654 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 389 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 699 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every4\"}",
    ), // 1143 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 646 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every2\"}",
    ), // 1921 ms
    (
        "B3",
        "certified",
        "{\"length\":500,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 2893 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 1946 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 1979 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 1789 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 1919 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every4\"}",
    ), // 2479 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 262 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every4\"}",
    ), // 474 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 649 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 273 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 367 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every4\"}",
    ), // 401 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 1039 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 429 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every2\"}",
    ), // 412 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every4\"}",
    ), // 719 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every8\"}",
    ), // 1740 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 600 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every2\"}",
    ), // 676 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every4\"}",
    ), // 869 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 1539 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 898 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every2\"}",
    ), // 698 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every4\"}",
    ), // 955 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 2360 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 724 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every2\"}",
    ), // 1061 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every4\"}",
    ), // 1916 ms
    ("B4", "monolithic", "{\"shape\":\"assoc\",\"n\":4,\"w\":32}"), // 2055 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":16}"), // 55 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":32}"), // 204 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":8}"), // 5 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":16}"), // 14 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":32}"), // 54 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":8}"), // 2 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":16}"), // 5 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":32}"), // 21 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":64}"), // 52 ms
    ("D1", "monolithic", "{\"t\":16,\"k\":1}"), // 1588 ms
    ("D1", "monolithic", "{\"t\":4,\"k\":2}"), // 1333 ms
    ("D1", "monolithic", "{\"t\":8,\"k\":1}"), // 486 ms
    ("D1", "monolithic", "{\"t\":8,\"k\":2}"), // 2263 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":1}"), // 17 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":2}"), // 75 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":4}"), // 278 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":8}"), // 993 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":1}"), // 16 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":2}"), // 62 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":4}"), // 276 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":8}"), // 840 ms
    ("D2", "monolithic", "{\"t\":4,\"k\":2}"), // 100 ms
    ("D2", "monolithic", "{\"t\":4,\"k\":4}"), // 252 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":1}"), // 16 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":2}"), // 97 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":4}"), // 410 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":8}"), // 956 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":1}"), // 25 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":2}"), // 75 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":4}"), // 274 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":8}"), // 910 ms
    ("D3", "split", "{\"t\":16,\"k\":1}"), // 255 ms
    ("D3", "split", "{\"t\":16,\"k\":2}"), // 1081 ms
    ("D3", "split", "{\"t\":4,\"k\":2}"), // 366 ms
    ("D3", "split", "{\"t\":4,\"k\":4}"), // 774 ms
    ("D3", "split", "{\"t\":64,\"k\":1}"), // 1337 ms
    ("D3", "split", "{\"t\":8,\"k\":1}"), // 158 ms
    ("D3", "split", "{\"t\":8,\"k\":2}"), // 709 ms
    ("D3", "split", "{\"t\":8,\"k\":4}"), // 1851 ms
    ("D4", "certified", "{\"t\":16,\"k\":1}"), // 2945 ms
    ("D4", "certified", "{\"t\":4,\"k\":2}"), // 745 ms
    ("D4", "certified", "{\"t\":4,\"k\":4}"), // 1551 ms
    ("D4", "certified", "{\"t\":8,\"k\":1}"), // 796 ms
    ("D4", "certified", "{\"t\":8,\"k\":2}"), // 2440 ms
    ("D4", "incremental", "{\"t\":16,\"k\":1}"), // 1009 ms
    ("D4", "incremental", "{\"t\":4,\"k\":2}"), // 333 ms
    ("D4", "incremental", "{\"t\":4,\"k\":4}"), // 1011 ms
    ("D4", "incremental", "{\"t\":4,\"k\":8}"), // 2027 ms
    ("D4", "incremental", "{\"t\":8,\"k\":1}"), // 329 ms
    ("D4", "incremental", "{\"t\":8,\"k\":2}"), // 868 ms
    ("D5", "certified", "{\"t\":16,\"k\":1,\"split\":\"index\"}"), // 618 ms
    ("D5", "certified", "{\"t\":4,\"k\":4,\"split\":\"index\"}"), // 1281 ms
    ("D5", "certified", "{\"t\":4,\"k\":4,\"split\":\"none\"}"), // 1969 ms
    ("D5", "certified", "{\"t\":64,\"k\":1,\"split\":\"index\"}"), // 2833 ms
];

/// `(experiment, mode, knob key)` of the knob groups whose instance takes too long to merely
/// BUILD (no solve) for `check`'s structural rebuild (`spot::structure_check`, VOS-028c2 round 2).
/// These are the five D3 groups at T = 256: the split builds 256 obligations, each a
/// T-parameterised self-composition, and one seed takes 50 s (k = 1) to 92 s (k = 16) on the
/// reference host (Apple silicon, release build, measured when the structural rebuild was added),
/// where every other full-tier non-E group builds in at most 4.4 s.  They are rebuilt only by
/// `check --full`.  Like [`CHEAP`] this is a reviewed constant keyed by the plan, never read from
/// the file; the test `structure_heavy_is_exactly_the_t_256_splits` pins its extent.
pub const STRUCT_HEAVY: &[(&str, &str, &str)] = &[
    ("D3", "split", "{\"t\":256,\"k\":1}"),  // 52 s
    ("D3", "split", "{\"t\":256,\"k\":2}"),  // 51 s
    ("D3", "split", "{\"t\":256,\"k\":4}"),  // 57 s
    ("D3", "split", "{\"t\":256,\"k\":8}"),  // 68 s
    ("D3", "split", "{\"t\":256,\"k\":16}"), // 92 s
];

/// Is the group of this row too slow to rebuild in CI, by the plan alone?
pub fn structure_heavy(experiment: &str, mode: &str, knob_key: &str) -> bool {
    STRUCT_HEAVY
        .iter()
        .any(|(e, m, k)| *e == experiment && *m == mode && *k == knob_key)
}

/// The slowest seed of every full-tier non-E knob group of `rows`, in milliseconds of solve plus
/// check, keyed `(experiment, mode, knob key)`.  A seed that ended `killed`, `oom` or
/// `crashed`, or whose time is not finite, has no finished time, so it counts as infinitely slow:
/// the group is never cheap.  This is the one
/// place the derivation of [`CHEAP`] is written down; the test
/// `cost_table_is_exactly_the_committed_recordings_cheap_set` applies [`derive_cheap`] to the
/// committed file and demands the constant equal it.
pub fn slowest_seeds(
    rows: &[crate::row::Row],
) -> std::collections::BTreeMap<(String, String, String), f64> {
    let mut worst = std::collections::BTreeMap::new();
    for r in rows {
        if r.tier != "full" || r.experiment.starts_with('E') {
            continue;
        }
        // `killed`, `oom` and `crashed` are the result words that mean "no finished run" (the
        // verdict words are `sat`, `unsat`, `unknown` and `collapsed`, each with a measured
        // time), and a time that is not a finite number is no measurement either: all four are
        // infinitely slow, so such a group is never cheap.
        let t = r.solve_ms + r.check_ms.unwrap_or(0.0);
        let c = if matches!(r.result.as_str(), "killed" | "oom" | "crashed") || !t.is_finite() {
            f64::INFINITY
        } else {
            t
        };
        let e = worst
            .entry((r.experiment.clone(), r.mode.clone(), r.knob.key()))
            .or_insert(0.0_f64);
        *e = e.max(c);
    }
    worst
}

/// The groups of `rows` whose slowest seed is at most [`CHEAP_MAX_MS`], with that slowest seed:
/// exactly the set [`CHEAP`] must list when `rows` is the committed recording.
pub fn derive_cheap(rows: &[crate::row::Row]) -> Vec<((String, String, String), f64)> {
    slowest_seeds(rows)
        .into_iter()
        .filter(|(_, c)| *c <= CHEAP_MAX_MS)
        .collect()
}

/// Is the group of this row cheap, by the plan alone?
pub fn is_cheap(experiment: &str, mode: &str, knob_key: &str) -> bool {
    CHEAP
        .iter()
        .any(|(e, m, k)| *e == experiment && *m == mode && *k == knob_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::planned_identities;

    #[test]
    fn every_cheap_entry_is_a_planned_full_tier_group_and_listed_once() {
        let planned = planned_identities();
        let mut seen = std::collections::BTreeSet::new();
        for (e, m, k) in CHEAP {
            assert!(
                planned.contains(&(e.to_string(), m.to_string(), k.to_string())),
                "{e} {m} {k} is not in the plan"
            );
            assert!(seen.insert((e, m, k)), "{e} {m} {k} is listed twice");
            assert!(!e.starts_with('E'), "enumeration rows are never sampled");
        }
    }

    #[test]
    fn structure_heavy_is_exactly_the_t_256_splits() {
        let planned = planned_identities();
        assert_eq!(STRUCT_HEAVY.len(), 5);
        for (e, m, k) in STRUCT_HEAVY {
            assert!(planned.contains(&(e.to_string(), m.to_string(), k.to_string())));
            assert!(structure_heavy(e, m, k));
            assert_eq!((*e, *m), ("D3", "split"));
            assert!(k.starts_with("{\"t\":256,"));
        }
        assert!(!structure_heavy("D3", "split", r#"{"t":64,"k":16}"#));
        // Exactly the plan's D3 groups at T = 256 and nothing else.
        let t256 = planned
            .iter()
            .filter(|(e, _, k)| e == "D3" && k.starts_with("{\"t\":256,"))
            .count();
        assert_eq!(t256, STRUCT_HEAVY.len());
    }

    #[test]
    fn a_group_is_cheap_by_name_only() {
        assert!(is_cheap("A1", "monolithic", r#"{"n":16}"#));
        assert!(!is_cheap("A1", "monolithic", r#"{"n":1000000000}"#));
        assert!(!is_cheap("B2", "monolithic", r#"{"length":25}"#));
        assert!(!is_cheap("A1", "certified", r#"{"n":16}"#));
    }
}
