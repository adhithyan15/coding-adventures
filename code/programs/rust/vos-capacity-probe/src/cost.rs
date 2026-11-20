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
//! ## How the table was derived, and what it is not
//!
//! The entries are the knob groups of the full tier whose *slowest seed*, in the committed
//! recording (after the VOS-D095 re-record, one release build on one Apple-silicon host), took at
//! most [`CHEAP_MAX_MS`] of solve plus check.  The figure after each entry is that slowest seed.
//! It is a reviewed constant: a re-recording that moves a group across the line is a change to
//! this table in the same commit, and the test `cost_table_matches_the_committed_recording`
//! fails until it is.  It is **not** read from the results file at check time, and it is not a
//! security boundary by itself: it only decides which rows the (budget-bounded) spot check may be
//! asked to finish.  A row in a cheap group that nevertheless does not finish inside the budget
//! is a `check` failure, not a skip (finding M5), so a wrong entry fails loudly.

/// The slowest seed, in milliseconds of solve plus check, that a group may have in the
/// committed recording to be listed in [`CHEAP`].
pub const CHEAP_MAX_MS: f64 = 3_000.0;

/// `(experiment, mode, knob key)` of the knob groups the spot check may re-run.
pub const CHEAP: &[(&str, &str, &str)] = &[
    ("A1", "monolithic", "{\"n\":16}"), // 2 ms
    ("A1", "monolithic", "{\"n\":32}"), // 5 ms
    ("A1", "monolithic", "{\"n\":64}"), // 39 ms
    ("A2", "monolithic", "{\"n\":16}"), // 3 ms
    ("A2", "monolithic", "{\"n\":32}"), // 7 ms
    ("A2", "monolithic", "{\"n\":64}"), // 24 ms
    ("B1", "monolithic", "{\"length\":1000,\"divergence\":0.0}"), // 0 ms
    ("B1", "monolithic", "{\"length\":500,\"divergence\":0.0}"), // 0 ms
    ("B2", "monolithic", "{\"length\":25,\"divergence\":0.25}"), // 2209 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 430 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 529 ms
    (
        "B3",
        "certified",
        "{\"length\":100,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 2035 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 828 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 1011 ms
    (
        "B3",
        "certified",
        "{\"length\":200,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 1731 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 1074 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 143 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 1655 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 239 ms
    (
        "B3",
        "certified",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every8\"}",
    ), // 2580 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 224 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 1852 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 481 ms
    (
        "B3",
        "certified",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 475 ms
    (
        "B3",
        "certified",
        "{\"length\":500,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 2068 ms
    (
        "B3",
        "certified",
        "{\"length\":500,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 2749 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 1268 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 2918 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 1222 ms
    (
        "B3",
        "incremental",
        "{\"length\":100,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 2342 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every32\"}",
    ), // 2648 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 436 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 133 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 710 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 199 ms
    (
        "B3",
        "incremental",
        "{\"length\":25,\"divergence\":1.0,\"cut\":\"every8\"}",
    ), // 1155 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every1\"}",
    ), // 384 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.05,\"cut\":\"every8\"}",
    ), // 1054 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every1\"}",
    ), // 434 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":0.25,\"cut\":\"every8\"}",
    ), // 1773 ms
    (
        "B3",
        "incremental",
        "{\"length\":50,\"divergence\":1.0,\"cut\":\"every1\"}",
    ), // 508 ms
    ("B4", "monolithic", "{\"shape\":\"assoc\",\"n\":4,\"w\":32}"), // 1436 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":16}"), // 33 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":32}"), // 141 ms
    ("B4", "monolithic", "{\"shape\":\"mul12\",\"w\":8}"), // 3 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":16}"), // 10 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":32}"), // 37 ms
    ("B4", "monolithic", "{\"shape\":\"mul5\",\"w\":8}"), // 1 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":16}"), // 2 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":32}"), // 11 ms
    ("C1", "monolithic", "{\"base\":\"a1\",\"n\":64}"), // 29 ms
    ("D1", "monolithic", "{\"t\":16,\"k\":1}"), // 1555 ms
    ("D1", "monolithic", "{\"t\":4,\"k\":2}"), // 564 ms
    ("D1", "monolithic", "{\"t\":4,\"k\":4}"), // 2734 ms
    ("D1", "monolithic", "{\"t\":8,\"k\":1}"), // 362 ms
    ("D1", "monolithic", "{\"t\":8,\"k\":2}"), // 1633 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":1}"), // 13 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":2}"), // 54 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":4}"), // 213 ms
    ("D2", "monolithic", "{\"t\":16,\"k\":8}"), // 706 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":16}"), // 2808 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":1}"), // 13 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":2}"), // 49 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":4}"), // 217 ms
    ("D2", "monolithic", "{\"t\":256,\"k\":8}"), // 634 ms
    ("D2", "monolithic", "{\"t\":4,\"k\":2}"), // 54 ms
    ("D2", "monolithic", "{\"t\":4,\"k\":4}"), // 190 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":16}"), // 2694 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":1}"), // 12 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":2}"), // 55 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":4}"), // 246 ms
    ("D2", "monolithic", "{\"t\":64,\"k\":8}"), // 749 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":1}"), // 13 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":2}"), // 57 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":4}"), // 210 ms
    ("D2", "monolithic", "{\"t\":8,\"k\":8}"), // 707 ms
    ("D3", "split", "{\"t\":16,\"k\":1}"), // 187 ms
    ("D3", "split", "{\"t\":16,\"k\":2}"), // 787 ms
    ("D3", "split", "{\"t\":16,\"k\":4}"), // 2664 ms
    ("D3", "split", "{\"t\":4,\"k\":2}"), // 211 ms
    ("D3", "split", "{\"t\":4,\"k\":4}"), // 568 ms
    ("D3", "split", "{\"t\":64,\"k\":1}"), // 745 ms
    ("D3", "split", "{\"t\":8,\"k\":1}"), // 93 ms
    ("D3", "split", "{\"t\":8,\"k\":2}"), // 391 ms
    ("D3", "split", "{\"t\":8,\"k\":4}"), // 1318 ms
    ("D4", "certified", "{\"t\":16,\"k\":1}"), // 1858 ms
    ("D4", "certified", "{\"t\":4,\"k\":2}"), // 546 ms
    ("D4", "certified", "{\"t\":4,\"k\":4}"), // 1120 ms
    ("D4", "certified", "{\"t\":4,\"k\":8}"), // 2267 ms
    ("D4", "certified", "{\"t\":8,\"k\":1}"), // 572 ms
    ("D4", "certified", "{\"t\":8,\"k\":2}"), // 1356 ms
    ("D4", "certified", "{\"t\":8,\"k\":4}"), // 2902 ms
    ("D4", "incremental", "{\"t\":16,\"k\":1}"), // 667 ms
    ("D4", "incremental", "{\"t\":16,\"k\":2}"), // 1808 ms
    ("D4", "incremental", "{\"t\":4,\"k\":2}"), // 229 ms
    ("D4", "incremental", "{\"t\":4,\"k\":4}"), // 518 ms
    ("D4", "incremental", "{\"t\":4,\"k\":8}"), // 1217 ms
    ("D4", "incremental", "{\"t\":8,\"k\":1}"), // 220 ms
    ("D4", "incremental", "{\"t\":8,\"k\":2}"), // 569 ms
    ("D4", "incremental", "{\"t\":8,\"k\":4}"), // 1405 ms
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
