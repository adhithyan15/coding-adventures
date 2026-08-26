//! # Report shapes — per-directive-kind pass/fail tallies.
//!
//! The whole point of this crate is a *legible* answer to "where does
//! `wasm-execution` actually stand against the real testsuite" — not just a
//! single pass/fail number, but one broken down by directive kind, so a
//! reader can immediately tell "the interpreter got 40 wrong answers" apart
//! from "we haven't built the type-checker yet" (see
//! `code/specs/W05-wasm-conformance-harness.md` section 4.3 for why that
//! distinction matters: `assert_invalid` can't be graded honestly without a
//! type-checker `wasm-validator` doesn't have yet, and lumping "not
//! supported" in with "wrong" would make the number worse than useless —
//! a maintainer chasing it down would "fix" the wrong thing).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Every kind of top-level `.wast` script directive this crate grades.
///
/// Mirrors `wasm_wast_parser::script::Directive` one-to-one, but as a
/// unit-only enum (no payload) — reports group and tally *by kind*, not by
/// each directive's individual data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DirectiveKind {
    Module,
    Register,
    Action,
    AssertReturn,
    AssertTrap,
    AssertExhaustion,
    AssertInvalid,
    AssertMalformed,
    AssertUnlinkable,
    /// `assert_exception` (W21 -- the exceptions proposal): a genuinely
    /// different outcome from `AssertTrap` -- see `wasm_wast_parser::
    /// script::Directive::AssertException`'s own doc comment.
    AssertException,
}

impl DirectiveKind {
    /// Every variant, in a fixed, stable order — used to print report rows
    /// in a consistent sequence and to seed an aggregate's tally map with
    /// every kind present (even ones a given run saw zero of), so the
    /// report's shape doesn't silently change just because a file happened
    /// not to use a particular directive.
    pub const ALL: [DirectiveKind; 10] = [
        DirectiveKind::Module,
        DirectiveKind::Register,
        DirectiveKind::Action,
        DirectiveKind::AssertReturn,
        DirectiveKind::AssertTrap,
        DirectiveKind::AssertExhaustion,
        DirectiveKind::AssertInvalid,
        DirectiveKind::AssertMalformed,
        DirectiveKind::AssertUnlinkable,
        DirectiveKind::AssertException,
    ];

    /// The stable string key used as a JSON object key (both in the golden
    /// baseline manifest and the report CLI's human-readable table) — the
    /// official testsuite's own directive spelling, so the report reads
    /// naturally next to the `.wast` source it's grading.
    pub fn label(self) -> &'static str {
        match self {
            DirectiveKind::Module => "module",
            DirectiveKind::Register => "register",
            DirectiveKind::Action => "action",
            DirectiveKind::AssertReturn => "assert_return",
            DirectiveKind::AssertTrap => "assert_trap",
            DirectiveKind::AssertExhaustion => "assert_exhaustion",
            DirectiveKind::AssertInvalid => "assert_invalid",
            DirectiveKind::AssertMalformed => "assert_malformed",
            DirectiveKind::AssertUnlinkable => "assert_unlinkable",
            DirectiveKind::AssertException => "assert_exception",
        }
    }
}

/// The result of running exactly one directive.
///
/// `NotYetSupported` is not a euphemism for `Fail` — it means grading this
/// directive correctly needs a capability this repo's WASM stack doesn't
/// have yet (a type-checker for `assert_invalid`, a linking-failure path
/// for `assert_unlinkable`, text-level malformed-detection for
/// `assert_malformed`'s `quote` variant beyond what the hand-built parser
/// already catches). Every `NotYetSupported` case is expected to flip to a
/// real `Pass`/`Fail` once the missing capability ships, with zero changes
/// to this harness — see `code/specs/W05-wasm-conformance-harness.md`
/// section 4.3.
#[derive(Debug, Clone, PartialEq)]
pub enum DirectiveOutcome {
    Pass,
    Fail(String),
    Trap(String),
    NotYetSupported(String),
}

impl DirectiveOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, DirectiveOutcome::Pass)
    }

    /// The bucket this outcome tallies into — used both for `Tally::record`
    /// and for printing a one-line reason next to a failing case.
    pub fn category(&self) -> &'static str {
        match self {
            DirectiveOutcome::Pass => "pass",
            DirectiveOutcome::Fail(_) => "fail",
            DirectiveOutcome::Trap(_) => "trap",
            DirectiveOutcome::NotYetSupported(_) => "not_yet_supported",
        }
    }

    /// The human-readable detail carried by a non-`Pass` outcome, if any —
    /// `None` for `Pass`, since there's nothing to explain.
    pub fn detail(&self) -> Option<&str> {
        match self {
            DirectiveOutcome::Pass => None,
            DirectiveOutcome::Fail(m)
            | DirectiveOutcome::Trap(m)
            | DirectiveOutcome::NotYetSupported(m) => Some(m),
        }
    }
}

/// Pass/fail/trap/not-yet-supported counts for one `DirectiveKind`, in one
/// file or aggregated across the whole run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    pub pass: usize,
    pub fail: usize,
    pub trap: usize,
    pub not_yet_supported: usize,
}

impl Tally {
    pub fn total(&self) -> usize {
        self.pass + self.fail + self.trap + self.not_yet_supported
    }

    /// Directives this harness actually graded one way or the other —
    /// `total()` minus the ones it structurally couldn't judge yet. The
    /// pass-rate a maintainer cares about is `pass / graded()`, not
    /// `pass / total()`: diluting the rate with known-unsupported cases
    /// would make "we haven't built the type-checker yet" look identical
    /// to "the interpreter got these wrong."
    pub fn graded(&self) -> usize {
        self.pass + self.fail + self.trap
    }

    pub fn record(&mut self, outcome: &DirectiveOutcome) {
        match outcome {
            DirectiveOutcome::Pass => self.pass += 1,
            DirectiveOutcome::Fail(_) => self.fail += 1,
            DirectiveOutcome::Trap(_) => self.trap += 1,
            DirectiveOutcome::NotYetSupported(_) => self.not_yet_supported += 1,
        }
    }

    fn merge(&mut self, other: &Tally) {
        self.pass += other.pass;
        self.fail += other.fail;
        self.trap += other.trap;
        self.not_yet_supported += other.not_yet_supported;
    }
}

/// One file's tallies, one `Tally` per `DirectiveKind` seen (or zero-valued
/// for kinds the file didn't use — see `DirectiveKind::ALL`'s doc comment).
pub type KindTallies = BTreeMap<String, Tally>;

/// Fold a file's per-directive outcomes into a `KindTallies`, pre-seeded
/// with every `DirectiveKind` at zero so the JSON shape never quietly
/// varies file to file.
pub fn tally_results(results: &[(DirectiveKind, DirectiveOutcome)]) -> KindTallies {
    let mut tallies: KindTallies =
        DirectiveKind::ALL.iter().map(|k| (k.label().to_string(), Tally::default())).collect();
    for (kind, outcome) in results {
        tallies.entry(kind.label().to_string()).or_default().record(outcome);
    }
    tallies
}

/// The full conformance report: one `KindTallies` per vendored file, plus
/// the sum across all of them. This exact shape (`BTreeMap<String,
/// KindTallies>` for `files`, keyed by filename for deterministic JSON
/// ordering) is what gets serialized to the golden baseline manifest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConformanceReport {
    pub files: BTreeMap<String, KindTallies>,
    pub aggregate: KindTallies,
    /// Files whose `.wast` SCRIPT itself failed to parse -- a strictly
    /// different failure mode from any individual directive's outcome
    /// (nothing inside the file was gradeable at all), kept as its own
    /// field rather than folded into `files` as an all-zero `KindTallies`
    /// so the baseline stays self-explanatory and this class of gap can't
    /// be silently confused with "this file legitimately has zero
    /// `assert_return` directives."
    pub parse_failures: BTreeMap<String, String>,
}

impl ConformanceReport {
    pub fn add_file(&mut self, file: impl Into<String>, tallies: KindTallies) {
        for (kind, tally) in &tallies {
            self.aggregate.entry(kind.clone()).or_default().merge(tally);
        }
        self.files.insert(file.into(), tallies);
    }

    pub fn add_parse_failure(&mut self, file: impl Into<String>, error: impl Into<String>) {
        self.parse_failures.insert(file.into(), error.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tally_records_each_outcome_category_once() {
        let mut t = Tally::default();
        t.record(&DirectiveOutcome::Pass);
        t.record(&DirectiveOutcome::Fail("wrong result".into()));
        t.record(&DirectiveOutcome::Trap("unexpected trap".into()));
        t.record(&DirectiveOutcome::NotYetSupported("no type-checker".into()));
        assert_eq!(t, Tally { pass: 1, fail: 1, trap: 1, not_yet_supported: 1 });
        assert_eq!(t.total(), 4);
        assert_eq!(t.graded(), 3, "not_yet_supported must not count as graded");
    }

    #[test]
    fn tally_results_seeds_every_kind_at_zero() {
        let results = vec![(DirectiveKind::AssertReturn, DirectiveOutcome::Pass)];
        let tallies = tally_results(&results);
        assert_eq!(tallies.len(), DirectiveKind::ALL.len());
        assert_eq!(tallies["assert_return"].pass, 1);
        assert_eq!(tallies["module"], Tally::default());
    }

    #[test]
    fn conformance_report_aggregate_sums_across_files() {
        let mut report = ConformanceReport::default();
        report.add_file(
            "a.wast",
            tally_results(&[(DirectiveKind::AssertReturn, DirectiveOutcome::Pass)]),
        );
        report.add_file(
            "b.wast",
            tally_results(&[
                (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
                (DirectiveKind::AssertReturn, DirectiveOutcome::Fail("x".into())),
            ]),
        );
        assert_eq!(report.aggregate["assert_return"], Tally { pass: 2, fail: 1, trap: 0, not_yet_supported: 0 });
        assert_eq!(report.files.len(), 2);
    }

    #[test]
    fn conformance_report_round_trips_through_json() {
        let mut report = ConformanceReport::default();
        report.add_file(
            "a.wast",
            tally_results(&[(DirectiveKind::AssertTrap, DirectiveOutcome::Trap("oob".into()))]),
        );
        let json = serde_json::to_string(&report).unwrap();
        let restored: ConformanceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, restored);
    }

    #[test]
    fn parse_failures_are_tracked_separately_from_zero_tallies() {
        let mut report = ConformanceReport::default();
        report.add_parse_failure("broken.wast", "unexpected end of input");
        assert_eq!(report.parse_failures.len(), 1);
        assert_eq!(report.parse_failures["broken.wast"], "unexpected end of input");
        // A parse failure must NOT also show up as a (misleadingly
        // all-zero) entry in `files`.
        assert!(!report.files.contains_key("broken.wast"));
    }
}
