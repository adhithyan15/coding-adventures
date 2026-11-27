"""Falsification of the v2 conformance harness (slice S6a, first acceptance item, [D42]).

    "the runner's conformance harness, given deliberately wrong evaluators (at
    least: no wrap, eager ``and``, first-binder-fastest enumeration), reports
    failures naming case ids"

This is the proof that the harness is not a rubber stamp.  Each mutant of
``v2_mutants.py`` is one realistic mistake; the harness is run against it and
must report failures **by case id**, including the ids that pin that mistake.
The control is the standard semantics, which must report none.
"""

from __future__ import annotations

import pytest
from helpers import REPO_ROOT
from v2_mutants import MUTANTS

from coding_adventures_warrant_corpus.v2 import STANDARD, run_kit

FIXTURE = REPO_ROOT / "code" / "specs" / "fixtures" / "warrant-corpus-conformance-v1"


@pytest.mark.parametrize("name", sorted(MUTANTS))
def test_the_harness_reports_the_mutant_by_case_id(name):
    mutant, categories, pinned = MUTANTS[name]
    report = run_kit(FIXTURE, sem=mutant, categories=categories, sequential=False)
    assert not report.ok, f"{name}: the harness passed a deliberately wrong evaluator"
    failed = set(report.failed_ids)
    missing = [case for case in pinned if case not in failed]
    assert not missing, f"{name}: these pinning cases did not fail: {missing}"
    # a failure line names the case id and the outcome, never the input
    lines = [line for line in report.lines() if line.startswith("not ok ")]
    assert lines and all(line.split(":")[0].split()[-1] in failed for line in lines)


@pytest.mark.parametrize("name", sorted(MUTANTS))
def test_the_standard_semantics_passes_what_the_mutant_fails(name):
    """The same categories, the same cases: the control group is green."""
    _mutant, categories, _pinned = MUTANTS[name]
    report = run_kit(FIXTURE, sem=STANDARD, categories=categories, sequential=False)
    assert report.ok, report.lines()


def test_a_mutant_is_reported_in_every_category_it_was_built_to_break():
    """The three mutants the spec names (and two of S10e), and the category each must be caught in."""
    expectations = {
        "no-wrap": {"v2-arith", "v2-ir", "v2-fault"},
        "eager-and": {"v2-ir"},
        "first-binder-fastest": {"v2-chunk"},
        # S10e: the mistakes of the two new levels are caught where each one lives
        "shift-count-masked": {"v2-ir"},
        "tuple-first-component-fastest": {"v2-domain", "v2-chunk"},
    }
    for name, wanted in expectations.items():
        mutant, categories, _pinned = MUTANTS[name]
        report = run_kit(FIXTURE, sem=mutant, categories=categories, sequential=False)
        broken = {c.category for c in report.categories.values() if c.failed}
        assert broken == wanted, (name, broken)
