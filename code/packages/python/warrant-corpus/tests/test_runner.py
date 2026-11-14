"""Executing suites against a real (reference) library."""

from __future__ import annotations

import pytest
from helpers import CORPUS_DIR, REPO_ROOT, copy_corpus, new_library

from coding_adventures_warrant_corpus import load_corpus
from coding_adventures_warrant_corpus.binding import (
    Subject,
    parse_binding,
    resolve_exports,
)
from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.faults import FAULTS, Fault
from coding_adventures_warrant_corpus.runner import (
    ControlResult,
    run_control,
    run_examples,
    run_suite,
    shuffled,
)


def subject_for(tmp_path, monkeypatch, overrides=None, corpus=None, root=None):
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch, overrides)
    binding = parse_binding(binding_path.read_bytes())
    exports = resolve_exports(binding, package_dir)
    return binding, exports, Subject(binding, exports, corpus.ops)


def suite(corpus, claim: str):
    return next(s for s in corpus.suites if s.claim == claim)


def test_a_correct_library_matches_every_digest_for_a_small_suite(
    tmp_path, monkeypatch, corpus
) -> None:
    _b, _e, subject = subject_for(tmp_path, monkeypatch, corpus=corpus)
    target = suite(corpus, "eq_fixed_agrees_w2")
    result = run_suite(corpus, target, subject, fail_fast=False)
    assert result.passed
    assert (result.cases, result.discarded, result.calls) == (256, 0, 256)
    assert result.inputs_sha256 == target.inputs_sha256
    assert result.expected_sha256 == target.expected_sha256


@pytest.mark.parametrize(
    ("override", "failure"),
    [
        ("def ct_eq_fixed(a, b):\n    return 1 if a == b else 0\n", "type-mismatch"),
        ("def ct_eq_fixed(a, b):\n    return True\n", "result-mismatch"),
        ("def ct_eq_fixed(a, b):\n    raise KeyError('boom')\n", "unexpected-error"),
        ("def ct_eq_fixed(a, b):\n    return None\n", "type-mismatch"),
    ],
)
def test_a_wrong_library_fails_with_the_right_failure_id(
    tmp_path, monkeypatch, corpus, override, failure
) -> None:
    _b, _e, subject = subject_for(
        tmp_path, monkeypatch, {"ct_eq_fixed": override}, corpus=corpus
    )
    result = run_suite(
        corpus, suite(corpus, "eq_fixed_agrees_w2"), subject, fail_fast=False
    )
    assert not result.passed
    assert result.failures > 0 and result.failed_cases[0][1] == failure
    # The oracle never moved: the digests are still the corpus's.
    assert result.inputs_sha256 == suite(corpus, "eq_fixed_agrees_w2").inputs_sha256


def test_a_single_wrong_case_is_found_and_named_by_index(
    tmp_path, monkeypatch, corpus
) -> None:
    buggy = (
        "def ct_eq_fixed(a, b):\n"
        "    if a == b'\\xff\\xff' and b == b'\\xff\\xff':\n"
        "        return False\n"
        "    return a == b\n"
    )
    _b, _e, subject = subject_for(
        tmp_path, monkeypatch, {"ct_eq_fixed": buggy}, corpus=corpus
    )
    result = run_suite(
        corpus, suite(corpus, "eq_fixed_agrees_w2"), subject, fail_fast=False
    )
    assert result.failures == 1
    assert result.failed_cases == [(255, "result-mismatch")]  # the last of 256


def test_fail_fast_stops_at_the_first_failure(tmp_path, monkeypatch, corpus) -> None:
    _b, _e, subject = subject_for(
        tmp_path,
        monkeypatch,
        {"ct_eq": "def ct_eq(a, b):\n    return True\n"},
        corpus=corpus,
    )
    result = run_suite(corpus, suite(corpus, "eq_agrees"), subject, fail_fast=True)
    assert result.failures == 1 and result.cases < 1000 and not result.passed


def test_many_failures_are_counted_but_only_a_few_are_named(
    tmp_path, monkeypatch, corpus
) -> None:
    _b, _e, subject = subject_for(
        tmp_path,
        monkeypatch,
        {"ct_eq_fixed": "def ct_eq_fixed(a, b):\n    return True\n"},
        corpus=corpus,
    )
    result = run_suite(
        corpus, suite(corpus, "eq_fixed_agrees_w2"), subject, fail_fast=False
    )
    assert result.failures > len(result.failed_cases) == 10


def test_a_missing_error_is_classified_separately(
    tmp_path, monkeypatch, corpus
) -> None:
    lenient = (
        "def ct_select_bytes(a, b, choice):\n"
        "    return bytes(a) if choice else bytes(b)\n"  # never raises on mismatch
    )
    _b, _e, subject = subject_for(
        tmp_path, monkeypatch, {"ct_select_bytes": lenient}, corpus=corpus
    )
    result = run_suite(
        corpus, suite(corpus, "select_rejects_mismatch"), subject, fail_fast=True
    )
    assert result.failed_cases[0][1] == "missing-error"


def test_a_mutable_result_is_a_type_mismatch(tmp_path, monkeypatch, corpus) -> None:
    mutable = (
        "def ct_select_bytes(a, b, choice):\n"
        "    if len(a) != len(b):\n"
        "        raise ValueError('x')\n"
        "    return bytearray(a if choice else b)\n"
    )
    _b, _e, subject = subject_for(
        tmp_path, monkeypatch, {"ct_select_bytes": mutable}, corpus=corpus
    )
    result = run_suite(corpus, suite(corpus, "select_picks"), subject, fail_fast=True)
    assert result.failed_cases[0][1] == "type-mismatch"


def test_examples_run_and_fail(tmp_path, monkeypatch, corpus) -> None:
    _b, _e, subject = subject_for(tmp_path, monkeypatch, corpus=corpus)
    results = run_examples(corpus, subject, lambda _id: True)
    assert len(results) == 12 and all(r.failure is None for r in results)
    only_eq = run_examples(corpus, subject, lambda i: i.endswith("/eq_empty"))
    assert [r.id for r in only_eq] == ["warrant/ct_compare/example/eq_empty"]

    _b, _e, bad = subject_for(
        tmp_path,
        monkeypatch,
        {"ct_eq": "def ct_eq(a, b):\n    raise KeyError(1)\n"},
        corpus=corpus,
    )
    failed = run_examples(corpus, bad, lambda i: "/eq_" in i)
    assert {r.failure for r in failed} == {"unexpected-error"}


def test_tampered_digests_and_counts_fail_closed(tmp_path, monkeypatch, corpus) -> None:
    def tamper(field: str, value):
        def change(document: dict) -> None:
            for row in document["suites"]:
                if row["claim"] == "eq_fixed_agrees_w2":
                    row[field] = value

        root, corpus_dir = copy_corpus(tmp_path, corpus=change)
        tampered = load_corpus(root, corpus_dir)
        _b, _e, subject = subject_for(tmp_path, monkeypatch, corpus=tampered)
        return run_suite(
            tampered, suite(tampered, "eq_fixed_agrees_w2"), subject, fail_fast=False
        )

    assert tamper("inputs_sha256", "0" * 64).suite_failures == ["stream-mismatch"]
    assert tamper("expected_sha256", "f" * 64).suite_failures == ["stream-mismatch"]
    assert tamper("expected_calls", 255).suite_failures == ["call-count-mismatch"]
    # Declared counts that disagree with the domains are refused before any
    # case runs (test_hardening): the count-mismatch verdict is a backstop.
    with pytest.raises(CorpusError):
        tamper("cases", 257)


def test_ir_problems_are_refused_before_any_case_runs(tmp_path, monkeypatch) -> None:
    def break_with(side: str, node):
        def change(document: dict) -> None:
            for row in document["suites"]:
                if row["claim"] == "eq_fixed_agrees_w2":
                    row[side] = node

        root, corpus_dir = copy_corpus(tmp_path, corpus=change)
        broken = load_corpus(root, corpus_dir)
        _b, _e, subject = subject_for(tmp_path, monkeypatch, corpus=broken)
        with pytest.raises(CorpusError):
            run_suite(
                broken, suite(broken, "eq_fixed_agrees_w2"), subject, fail_fast=False
            )

    call = ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [2]]
    break_with("rhs", call)  # subject on both sides
    break_with("lhs", ["lit", True])  # no subject call on either side
    break_with("lhs", ["mystery", 1])  # unknown tag
    break_with("admit", call)  # a subject call in an admission expression


def test_a_model_may_not_call_the_subject(tmp_path, monkeypatch) -> None:
    def change(document: dict) -> None:
        document["models"]["same"]["body"] = [
            "subject_op",
            "eq",
            [["var", "a"], ["var", "b"]],
        ]

    root, corpus_dir = copy_corpus(tmp_path, corpus=change)
    broken = load_corpus(root, corpus_dir)
    _b, _e, subject = subject_for(tmp_path, monkeypatch, corpus=broken)
    with pytest.raises(CorpusError):
        run_suite(broken, suite(broken, "eq_agrees"), subject, fail_fast=False)


def test_a_control_catches_a_real_fault(tmp_path, monkeypatch, corpus) -> None:
    binding, exports, _s = subject_for(tmp_path, monkeypatch, corpus=corpus)
    index = next(
        i
        for i, c in enumerate(corpus.controls)
        if c.fault == "ignore-last-byte" and c.op == "eq_fixed"
    )
    result = run_control(corpus, binding, exports, index)
    assert result.passed and result.reached_export
    # width 0 has no last byte to ignore, so that suite cannot notice the fault
    assert [c.rsplit("_", 1)[1] for c in result.caught_by] == ["w1", "w2", "w4", "w16"]


def test_a_control_that_nothing_notices_fails(tmp_path, monkeypatch, corpus) -> None:
    binding, exports, _s = subject_for(tmp_path, monkeypatch, corpus=corpus)
    harmless = Fault(
        "invert",
        FAULTS["invert"].applies,
        lambda args, _sig: args,
        lambda result, _a, _s2: result,  # does nothing: the suites cannot fail
    )
    monkeypatch.setitem(FAULTS, "invert", harmless)
    index = next(
        i
        for i, c in enumerate(corpus.controls)
        if c.fault == "invert" and c.op == "eq_fixed"
    )
    result = run_control(corpus, binding, exports, index)
    assert result.caught_by == [] and not result.passed


def test_a_control_must_reach_the_export() -> None:
    assert ControlResult("c", "f", "op", "exhaustive", ["claim"], True).passed
    assert not ControlResult("c", "f", "op", "exhaustive", ["claim"], False).passed
    assert not ControlResult("c", "f", "op", "exhaustive", [], True).passed


def test_shuffle_is_a_deterministic_permutation(corpus) -> None:
    first = shuffled(corpus.suites, 1234)
    assert first == shuffled(corpus.suites, 1234)
    assert sorted(s.id for s in first) == sorted(s.id for s in corpus.suites)
    assert any(shuffled(corpus.suites, seed) != first for seed in range(2, 12))
    assert shuffled([], 1) == []


def test_corpus_dir_constants_exist() -> None:
    assert CORPUS_DIR.is_dir() and REPO_ROOT.is_dir()
