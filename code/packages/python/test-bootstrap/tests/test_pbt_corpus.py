"""pbt-v1 conformance for the Python engine, with negative controls.

The committed corpus ``code/specs/fixtures/pbt-v1/cases.json`` is loaded in
place (no copy) by walking up from this file to the repository root.  A runner
that cannot fail proves nothing, so most of this file corrupts a real case and
checks that ``run_pbt_corpus`` notices.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pytest

from coding_adventures_test_bootstrap import pbt
from coding_adventures_test_bootstrap.pbt_corpus import (
    PBT_V1_CASE_COUNT,
    PROPERTIES,
    SHAPES,
    evaluate_pbt_corpus,
    run_pbt_corpus,
)


def _repo_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "code/specs/fixtures/pbt-v1/cases.json").is_file():
            return parent
    raise AssertionError("repository root with the pbt-v1 fixture not found")


FIXTURE = _repo_root() / "code/specs/fixtures/pbt-v1/cases.json"
DOCUMENT = json.loads(FIXTURE.read_text(encoding="utf-8"))
CASES: list[dict[str, Any]] = DOCUMENT["cases"]


def _first(op: str, predicate=lambda case: True) -> dict[str, Any]:
    for case in CASES:
        if case["op"] == op and predicate(case):
            return copy.deepcopy(case)
    raise AssertionError(f"no {op} case in the corpus")


def _int(n: int) -> dict[str, str]:
    return {"kind": "int", "value": str(n)}


def _problems(*cases: dict[str, Any], expected: int | None = None) -> list[str]:
    return run_pbt_corpus(list(cases), expected_cases=expected)


# --- the positive result -----------------------------------------------------


def test_fixture_is_the_committed_25_case_corpus() -> None:
    assert DOCUMENT["schema_version"] == 1
    assert len(CASES) == 25 == PBT_V1_CASE_COUNT
    assert {case["op"] for case in CASES} == set(SHAPES)


def test_engine_conforms_to_every_pbt_v1_case() -> None:
    report = evaluate_pbt_corpus(CASES)
    assert report.problems == []
    assert report.evaluated == len(CASES) == 25
    assert run_pbt_corpus(CASES) == []


def test_every_property_name_in_the_corpus_is_registered() -> None:
    names = {c["property"] for c in CASES if "property" in c}
    assert names <= set(PROPERTIES)


# --- unknown / malformed input is a problem, never a skip --------------------


def test_unknown_op_is_a_problem_not_a_skip() -> None:
    bogus = {"id": "pbt/bogus", "op": "gen_quantum"}
    problems = _problems(bogus, expected=None)
    assert any("unknown op" in p for p in problems)
    report = evaluate_pbt_corpus(CASES + [bogus])
    assert report.evaluated == 25  # the unknown op was not counted as evaluated
    assert report.problems


@pytest.mark.parametrize("field", ["seed", "count", "outputs"])
def test_missing_field_is_a_problem(field: str) -> None:
    case = _first("gen_primitive")
    del case[field]
    assert any("fields" in p for p in _problems(case))


def test_extra_field_is_a_problem() -> None:
    case = _first("gen_primitive")
    case["bonus"] = True
    assert any("fields" in p for p in _problems(case))


def test_unknown_property_name_is_a_problem() -> None:
    case = _first("shrink_int")
    case["property"] = "int_is_prime"
    assert any("unknown property" in p for p in _problems(case))


@pytest.mark.parametrize("bad", ["007", "+5", "-0", " 5", "1_0", "", "0x10"])
def test_non_canonical_int_text_is_a_problem(bad: str) -> None:
    case = _first("gen_primitive", lambda c: c["type"] == "u8")
    case["seed"] = {"kind": "int", "value": bad}
    assert any("could not be evaluated" in p for p in _problems(case))


@pytest.mark.parametrize("bad", ["FF", "f", "ff ", "0G"])
def test_non_canonical_bytes_hex_is_a_problem(bad: str) -> None:
    case = _first("gen_bytes")
    case["outputs"] = [{"kind": "bytes", "hex": bad}]
    assert any("could not be evaluated" in p for p in _problems(case))


def test_malformed_value_nodes_are_problems() -> None:
    case = _first("gen_primitive", lambda c: c["type"] == "u8")
    for bad in (5, 1.5, None, {"kind": "float", "value": "1"}, {"kind": "int"}):
        corrupted = copy.deepcopy(case)
        corrupted["seed"] = bad
        assert any("could not be evaluated" in p for p in _problems(corrupted))


def test_non_object_and_idless_cases_are_problems() -> None:
    assert _problems("nope", expected=None)  # type: ignore[arg-type]
    assert _problems({"op": "gen_primitive"}, expected=None)
    dup = _first("failure_id_valid")
    assert any("duplicate" in p for p in _problems(dup, dup, expected=None))


# --- truncation --------------------------------------------------------------


def test_truncated_case_list_is_a_problem() -> None:
    for cut in (1, 5, 24):
        assert run_pbt_corpus(CASES[:cut]), f"truncation to {cut} not detected"
    assert any("expected 25 cases" in p for p in run_pbt_corpus(CASES[:-1]))


def test_dropping_every_case_of_an_op_is_a_problem() -> None:
    for op in SHAPES:
        kept = [c for c in CASES if c["op"] != op]
        problems = run_pbt_corpus(kept, expected_cases=None)
        assert any("never exercised" in p and op in p for p in problems), op


def test_missing_failure_ids_are_problems() -> None:
    kept = [c for c in CASES if c.get("input") != "property-timeout"]
    assert any("failure ids missing" in p for p in run_pbt_corpus(kept, None))
    kept = [c for c in CASES if c.get("input") != "unregistered-id"]
    assert any("unregistered-id rejection" in p for p in run_pbt_corpus(kept, None))


# --- negative controls: a corrupted expectation must be reported -------------

CORRUPTIONS: list[tuple[str, Any]] = []


def _register(name: str):
    def deco(fn):
        CORRUPTIONS.append((name, fn))
        return fn

    return deco


@_register("gen_primitive/flip a bool")
def _c_bool(_: None) -> dict[str, Any]:
    case = _first("gen_primitive", lambda c: c["type"] == "bool")
    case["outputs"][0] = not case["outputs"][0]
    return case


@_register("gen_primitive/off-by-one u64")
def _c_u64(_: None) -> dict[str, Any]:
    case = _first("gen_primitive", lambda c: c["type"] == "u64")
    case["outputs"][0]["value"] = str(int(case["outputs"][0]["value"]) + 1)
    return case


@_register("gen_primitive/bool encoded as int")
def _c_bool_as_int(_: None) -> dict[str, Any]:
    case = _first("gen_primitive", lambda c: c["type"] == "bool")
    case["outputs"][0] = _int(1 if case["outputs"][0] else 0)
    return case


@_register("gen_primitive/i128 sign")
def _c_i128(_: None) -> dict[str, Any]:
    case = _first("gen_primitive", lambda c: c["type"] == "i128")
    case["outputs"][0]["value"] = str(-int(case["outputs"][0]["value"]))
    return case


@_register("gen_primitive/dropped output")
def _c_prim_drop(_: None) -> dict[str, Any]:
    case = _first("gen_primitive", lambda c: c["type"] == "u8")
    case["outputs"].pop()
    return case


@_register("gen_bounded/off-by-one")
def _c_bounded(_: None) -> dict[str, Any]:
    case = _first("gen_bounded")
    case["outputs"][0]["value"] = str(int(case["outputs"][0]["value"]) + 1)
    return case


@_register("gen_bytes/flip an output byte")
def _c_bytes(_: None) -> dict[str, Any]:
    case = _first("gen_bytes", lambda c: c["outputs"][0]["hex"] != "")
    hexed = case["outputs"][0]["hex"]
    case["outputs"][0]["hex"] = ("ff" if hexed[:2] != "ff" else "00") + hexed[2:]
    return case


@_register("gen_bytes/lengthen")
def _c_bytes_len(_: None) -> dict[str, Any]:
    case = _first("gen_bytes")
    case["outputs"][0]["hex"] += "00"
    return case


@_register("shrink_int/steps+1")
def _c_si_steps(_: None) -> dict[str, Any]:
    case = _first("shrink_int")
    case["expected_steps"] = _int(int(case["expected_steps"]["value"]) + 1)
    return case


@_register("shrink_int/wrong minimal")
def _c_si_min(_: None) -> dict[str, Any]:
    case = _first("shrink_int")
    case["expected_minimal"] = _int(int(case["expected_minimal"]["value"]) + 1)
    return case


@_register("shrink_int/wrong property")
def _c_si_prop(_: None) -> dict[str, Any]:
    case = _first("shrink_int", lambda c: c["property"] == "int_le_100")
    case["property"] = "int_abs_le_10"
    return case


@_register("shrink_refinement/steps+1")
def _c_sr_steps(_: None) -> dict[str, Any]:
    case = _first("shrink_refinement")
    case["expected_steps"] = _int(int(case["expected_steps"]["value"]) + 1)
    return case


@_register("shrink_refinement/crossing the lower bound")
def _c_sr_bound(_: None) -> dict[str, Any]:
    case = _first("shrink_refinement")
    case["expected_minimal"] = _int(int(case["target"]["value"]) - 1)
    return case


@_register("shrink_list/wrong minimal")
def _c_sl_min(_: None) -> dict[str, Any]:
    case = _first("shrink_list")
    case["expected_minimal"] = case["expected_minimal"] + [_int(0)]
    return case


@_register("shrink_list/steps+1")
def _c_sl_steps(_: None) -> dict[str, Any]:
    case = _first("shrink_list")
    case["expected_steps"] = _int(int(case["expected_steps"]["value"]) + 1)
    return case


@_register("shrink_bytes/wrong minimal")
def _c_sb_min(_: None) -> dict[str, Any]:
    case = _first("shrink_bytes")
    case["expected_minimal"]["hex"] = "0000000001"
    return case


@_register("shrink_bytes/steps+1")
def _c_sb_steps(_: None) -> dict[str, Any]:
    case = _first("shrink_bytes")
    case["expected_steps"] = _int(int(case["expected_steps"]["value"]) + 1)
    return case


@_register("replay/wrong initial_fail")
def _c_rp_init(_: None) -> dict[str, Any]:
    case = _first("replay_consistency")
    case["initial_fail"] = _int(int(case["initial_fail"]["value"]) + 1)
    return case


@_register("replay/wrong minimal")
def _c_rp_min(_: None) -> dict[str, Any]:
    case = _first("replay_consistency")
    case["expected_minimal"] = _int(int(case["expected_minimal"]["value"]) + 1)
    return case


@_register("filter_exhausted/wrong failure id")
def _c_fe_id(_: None) -> dict[str, Any]:
    case = _first("filter_exhausted")
    case["failure_id"] = "property-counterexample"
    return case


@_register("filter_exhausted/claims a pass")
def _c_fe_verdict(_: None) -> dict[str, Any]:
    case = _first("filter_exhausted")
    case["verdict"] = "pass"
    return case


@_register("failure_id_valid/registered id claimed invalid")
def _c_fi_valid(_: None) -> dict[str, Any]:
    case = _first("failure_id_valid", lambda c: c["valid"] is True)
    case["valid"] = False
    return case


@_register("failure_id_valid/unregistered id claimed valid")
def _c_fi_invalid(_: None) -> dict[str, Any]:
    case = _first("failure_id_valid", lambda c: c["valid"] is False)
    case["valid"] = True
    return case


@_register("corpus_reject/accepted text")
def _c_cr(_: None) -> dict[str, Any]:
    case = _first("corpus_reject")
    case["text"] = "tfv 1\nr1 R1 Kop S706274\n"
    return case


@pytest.mark.parametrize(
    "build", [fn for _, fn in CORRUPTIONS], ids=[name for name, _ in CORRUPTIONS]
)
def test_corrupted_expectation_is_reported(build) -> None:
    corrupted = build(None)
    assert run_pbt_corpus([corrupted], expected_cases=None) != [], "undetected"
    # and inside the whole corpus the same corruption fails the whole run
    swapped = [corrupted if c["id"] == corrupted["id"] else c for c in CASES]
    assert run_pbt_corpus(swapped) != []


def test_every_op_has_at_least_one_negative_control() -> None:
    covered = {name.split("/")[0] for name, _ in CORRUPTIONS}
    ops = set(SHAPES)
    # shrink_refinement/replay are named by their short form above
    covered |= {"replay_consistency"} if "replay" in covered else set()
    assert ops <= covered, ops - covered


# --- engine sabotage: the runner must catch a broken engine ------------------


def test_runner_catches_a_sabotaged_engine(monkeypatch: pytest.MonkeyPatch) -> None:
    from coding_adventures_test_bootstrap import pbt_corpus

    real_shrink_int = pbt.shrink_int

    def lazy_shrink_int(initial, target, lo, hi, prop, max_steps=1000):
        minimal, steps = real_shrink_int(initial, target, lo, hi, prop, max_steps)
        return minimal, steps + 1  # off-by-one step accounting

    monkeypatch.setattr(pbt_corpus, "shrink_int", lazy_shrink_int)
    assert run_pbt_corpus(CASES)

    monkeypatch.undo()
    monkeypatch.setattr(pbt_corpus, "is_pbt_failure_id", lambda _x: True)
    assert run_pbt_corpus(CASES)

    monkeypatch.undo()
    monkeypatch.setattr(pbt_corpus, "gen_primitive", lambda *a: [])
    assert run_pbt_corpus(CASES)

    monkeypatch.undo()
    monkeypatch.setattr(pbt_corpus, "parse_tfv", lambda text: [])
    assert run_pbt_corpus(CASES)

    monkeypatch.undo()
    # a filter that stops early (wrong retry count) is caught end to end
    real_filter = pbt.Gen.filter
    monkeypatch.setattr(
        pbt.Gen, "filter", lambda self, p, max_retries=100: real_filter(self, p, 1)
    )
    assert run_pbt_corpus(CASES)

    monkeypatch.undo()
    assert run_pbt_corpus(CASES) == []  # and the unsabotaged engine is clean


def test_engine_seed_sabotage_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    from coding_adventures_test_bootstrap import prng

    real_bounded = prng.SplitMix64.bounded

    def biased(self, n):
        return real_bounded(self, n) ^ (1 if n == 256 else 0)

    monkeypatch.setattr(prng.SplitMix64, "bounded", biased)
    assert run_pbt_corpus(CASES)


def test_wrongly_typed_fields_are_problems() -> None:
    case = _first("gen_primitive", lambda c: c["type"] == "u8")
    case["count"] = True  # a bool is not an int
    assert any("must be an int" in p for p in _problems(case))
    case = _first("shrink_list")
    case["initial"] = [True, _int(1)]
    assert any("list of ints" in p for p in _problems(case))
    case = _first("shrink_list")
    case["initial"] = "oops"
    assert any("list of ints" in p for p in _problems(case))
    case = _first("shrink_bytes")
    case["initial"] = [_int(1)]
    assert any("must be bytes" in p for p in _problems(case))


def test_engine_invariants_are_checked_independently_of_expected_values(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from coding_adventures_test_bootstrap import pbt_corpus

    # An engine whose shrink_int returns a value that does not falsify, or that
    # left the bounds, is reported even when the corpus numbers are rewritten
    # to agree with it.
    case = _first("shrink_int", lambda c: c["property"] == "int_le_100")
    monkeypatch.setattr(pbt_corpus, "shrink_int", lambda *a, **k: (3, 5))
    case["expected_minimal"], case["expected_steps"] = _int(3), _int(5)
    problems = _problems(case)
    assert any("does not falsify" in p for p in problems)
    case = _first("shrink_int", lambda c: c["property"] == "int_le_100")
    monkeypatch.setattr(pbt_corpus, "shrink_int", lambda *a, **k: (10**9, 5))
    case["expected_minimal"], case["expected_steps"] = _int(10**9), _int(5)
    assert any("left [" in p for p in _problems(case))
    monkeypatch.undo()
    seq = _first("shrink_list")
    monkeypatch.setattr(pbt_corpus, "shrink_list", lambda *a, **k: ([0], 1))
    seq["expected_minimal"], seq["expected_steps"] = [_int(0)], _int(1)
    assert any("does not falsify" in p for p in _problems(seq))


# --- CWE-400: fixture-controlled sizes are capped ----------------------------


@pytest.mark.parametrize(
    ("op", "field", "huge"),
    [
        ("gen_primitive", "count", 10**9),
        ("gen_bounded", "count", 10**9),
        ("gen_bytes", "count", 10**9),
        ("gen_bytes", "max_len", 10**9),
        ("filter_exhausted", "max_retries", 10**12),
    ],
)
def test_oversized_loop_drivers_are_problems_not_hangs(
    op: str, field: str, huge: int
) -> None:
    case = _first(op)
    case[field] = _int(huge)
    problems = _problems(case)
    assert any("outside [0," in p for p in problems), problems


def test_negative_sizes_and_oversized_values_are_problems() -> None:
    case = _first("gen_primitive")
    case["count"] = _int(-1)
    assert any("outside [0," in p for p in _problems(case))
    case = _first("gen_bounded")
    case["outputs"] = [_int(0)] * 10_001
    assert any("list exceeds" in p for p in _problems(case))
    case = _first("shrink_bytes")
    case["initial"] = {"kind": "bytes", "hex": "00" * 10_001}
    assert any("bytes exceed" in p for p in _problems(case))
    case = _first("gen_bounded")
    case["max"] = _int(10**1001)
    assert any("digits" in p for p in _problems(case))


def test_unhashable_or_non_string_ids_are_problems() -> None:
    for bad in ([1], {"a": 1}, 5, None, 1.5):
        case = _first("failure_id_valid")
        case["id"] = bad
        problems = _problems(case, expected=None)
        assert problems and any("id" in p for p in problems), bad
    a, b = _first("failure_id_valid"), _first("failure_id_valid")
    a["id"] = b["id"] = ["same"]
    assert _problems(a, b, expected=None)
