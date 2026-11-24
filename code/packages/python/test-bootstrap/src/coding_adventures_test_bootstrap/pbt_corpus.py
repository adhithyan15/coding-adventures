"""Conformance runner for the shared ``pbt-v1`` corpus (VER05 section 6).

``code/specs/fixtures/pbt-v1/cases.json`` pins what every language's property
engine must reproduce.  This module is the Python lane's *production* runner:
it takes the already-parsed JSON (``json.load`` of ``cases.json``["cases"]),
evaluates every case with **this package's own public engine functions**, and
returns a list of human-readable problems.  An empty list means "conforms".

It does no file I/O (the package is capability-free); the test that loads the
committed fixture lives in ``tests/test_pbt_corpus.py``.

Why a runner that can fail matters: a conformance check that cannot report a
problem is worse than none.  So the runner is strict:

* a case must have exactly the fields of its op, with canonical encodings
  (ints are decimal strings, bytes are even-length lowercase hex);
* an unknown op, unknown property name or malformed value is a *problem*, never
  a silent skip;
* every op and all four failure ids must appear, at least one unregistered id
  must be rejected, and the number of cases must equal ``PBT_V1_CASE_COUNT``,
  so deleting cases from the list is itself a problem.

Case encoding (JSON)::

    {"kind": "int",   "value": "-12"}      an integer (any width, decimal text)
    {"kind": "bytes", "hex": "00ff"}       a byte string
    true / false                           a bool
    [ ... ]                                a list of the above
    "text"                                 a string (types, property names)
"""

from __future__ import annotations

import re
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

from coding_adventures_test_bootstrap.pbt import (
    PBT_FAILURE_IDS,
    PbtOptions,
    check_property,
    gen_bounded,
    gen_bytes_corpus,
    gen_int,
    gen_primitive,
    is_pbt_failure_id,
    shrink_bytes,
    shrink_int,
    shrink_list,
    simulate_replay,
)
from coding_adventures_test_bootstrap.tfv import parse_tfv

#: ``pbt-v1`` holds exactly this many cases; a different count is a problem
#: (a truncated list, or a corpus that grew without this lane being reviewed).
PBT_V1_CASE_COUNT = 25

#: Predicates for the shrink and replay cases.  ``True`` means "holds".
PROPERTIES: dict[str, Callable[[Any], bool]] = {
    "int_le_100": lambda x: x <= 100,
    "int_in_range_neg10_50": lambda x: -10 <= x <= 50,
    "int_is_even": lambda x: x % 2 == 0,
    "int_abs_le_10": lambda x: abs(x) <= 10,
    "list_no_consecutive_duplicates": lambda lst: all(
        lst[i] != lst[i + 1] for i in range(len(lst) - 1)
    ),
    "list_sum_le_20": lambda lst: sum(lst) <= 20,
    "bytes_len_lt_5": lambda b: len(b) < 5,
    "bytes_no_zero_byte": lambda b: 0 not in b,
    "refinement_x_le_50": lambda x: x <= 50,
}

#: The closed field set of each op (besides ``id`` and ``op``).
SHAPES: dict[str, frozenset[str]] = {
    "gen_primitive": frozenset({"type", "seed", "count", "outputs"}),
    "gen_bounded": frozenset({"min", "max", "seed", "count", "outputs"}),
    "gen_bytes": frozenset({"min_len", "max_len", "seed", "count", "outputs"}),
    "shrink_int": frozenset(
        {
            "initial",
            "target",
            "min",
            "max",
            "property",
            "expected_minimal",
            "expected_steps",
        }
    ),
    "shrink_refinement": frozenset(
        {
            "initial",
            "target",
            "min",
            "max",
            "property",
            "expected_minimal",
            "expected_steps",
        }
    ),
    "shrink_list": frozenset(
        {"initial", "elem_target", "property", "expected_minimal", "expected_steps"}
    ),
    "shrink_bytes": frozenset(
        {"initial", "property", "expected_minimal", "expected_steps"}
    ),
    "replay_consistency": frozenset(
        {"seed", "property", "initial_fail", "expected_minimal"}
    ),
    "filter_exhausted": frozenset({"max_retries", "verdict", "failure_id"}),
    "failure_id_valid": frozenset({"input", "valid"}),
    "corpus_reject": frozenset({"text"}),
}


@dataclass(frozen=True)
class CorpusReport:
    """``evaluated`` counts cases that were fully evaluated (match or not)."""

    evaluated: int
    problems: list[str] = field(default_factory=list)


class _CaseError(Exception):
    """A malformed case (not an engine mismatch)."""


_CANONICAL_INT = re.compile(r"-?(?:0|[1-9][0-9]*)")
_LOWER_HEX = re.compile(r"(?:[0-9a-f]{2})*")


def _decode(node: Any) -> Any:
    """JSON node -> Python value, strictly."""
    if isinstance(node, (bool, str)):
        return node
    if isinstance(node, list):
        return [_decode(item) for item in node]
    if isinstance(node, Mapping):
        kind = node.get("kind")
        if kind == "int" and set(node) == {"kind", "value"}:
            text = node["value"]
            if not isinstance(text, str) or not _CANONICAL_INT.fullmatch(text):
                raise _CaseError(f"non-canonical int {text!r}")
            if text == "-0":
                raise _CaseError("non-canonical int '-0'")
            return int(text)
        if kind == "bytes" and set(node) == {"kind", "hex"}:
            text = node["hex"]
            if not isinstance(text, str) or not _LOWER_HEX.fullmatch(text):
                raise _CaseError(f"bytes hex must be even-length lowercase: {text!r}")
            return bytes.fromhex(text)
        raise _CaseError(f"unrecognised value node {dict(node)!r}")
    raise _CaseError(f"unsupported JSON value {node!r}")


def _same(a: Any, b: Any) -> bool:
    """Equality that does not conflate ``True`` with ``1`` or lists with bytes."""
    if type(a) is not type(b):
        return False
    if isinstance(a, list):
        return len(a) == len(b) and all(_same(x, y) for x, y in zip(a, b, strict=True))
    return bool(a == b)


def _prop(name: Any) -> Callable[[Any], bool]:
    if not isinstance(name, str) or name not in PROPERTIES:
        raise _CaseError(f"unknown property {name!r}")
    return PROPERTIES[name]


def _int(value: Any, what: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise _CaseError(f"{what} must be an int")
    return value


def _check_gen(cid: str, op: str, v: dict[str, Any]) -> list[str]:
    if op == "gen_primitive":
        got = gen_primitive(
            _int(v["seed"], "seed"), v["type"], _int(v["count"], "count")
        )
    elif op == "gen_bounded":
        got = gen_bounded(
            _int(v["seed"], "seed"),
            _int(v["min"], "min"),
            _int(v["max"], "max"),
            _int(v["count"], "count"),
        )
    else:
        got = gen_bytes_corpus(
            _int(v["seed"], "seed"),
            _int(v["min_len"], "min_len"),
            _int(v["max_len"], "max_len"),
            _int(v["count"], "count"),
        )
    return [] if _same(got, v["outputs"]) else [f"{cid}: {op} outputs mismatch"]


def _check_shrink_int(cid: str, op: str, v: dict[str, Any]) -> list[str]:
    prop = _prop(v["property"])
    lo, hi = _int(v["min"], "min"), _int(v["max"], "max")
    minimal, steps = shrink_int(
        _int(v["initial"], "initial"), _int(v["target"], "target"), lo, hi, prop
    )
    problems = []
    if not _same(minimal, v["expected_minimal"]):
        problems.append(
            f"{cid}: {op} minimal mismatch: got {minimal}, "
            f"corpus {v['expected_minimal']}"
        )
    if not _same(steps, v["expected_steps"]):
        problems.append(
            f"{cid}: {op} step count mismatch: got {steps}, "
            f"corpus {v['expected_steps']}"
        )
    # Engine invariants, independent of the corpus's expected values.
    if prop(minimal):
        problems.append(f"{cid}: {op} minimal {minimal} does not falsify the property")
    if not lo <= minimal <= hi:
        problems.append(f"{cid}: {op} minimal {minimal} left [{lo}, {hi}]")
    return problems


def _check_shrink_seq(cid: str, op: str, v: dict[str, Any]) -> list[str]:
    prop = _prop(v["property"])
    if op == "shrink_list":
        items = v["initial"]
        if not isinstance(items, list) or not all(
            isinstance(i, int) and not isinstance(i, bool) for i in items
        ):
            raise _CaseError("initial must be a list of ints")
        minimal, steps = shrink_list(items, _int(v["elem_target"], "elem_target"), prop)
    else:
        if not isinstance(v["initial"], bytes):
            raise _CaseError("initial must be bytes")
        minimal, steps = shrink_bytes(v["initial"], prop)
    problems = []
    if not _same(minimal, v["expected_minimal"]):
        problems.append(
            f"{cid}: {op} minimal mismatch: got {minimal!r}, corpus "
            f"{v['expected_minimal']!r}"
        )
    if not _same(steps, v["expected_steps"]):
        problems.append(
            f"{cid}: {op} steps mismatch: got {steps}, corpus {v['expected_steps']}"
        )
    if prop(minimal):
        problems.append(
            f"{cid}: {op} minimal {minimal!r} does not falsify the property"
        )
    return problems


def _check_replay(cid: str, v: dict[str, Any]) -> list[str]:
    initial_fail, minimal = simulate_replay(
        _int(v["seed"], "seed"), _prop(v["property"])
    )
    problems = []
    if not _same(initial_fail, v["initial_fail"]):
        problems.append(
            f"{cid}: replay initial_fail mismatch: got {initial_fail}, "
            f"corpus {v['initial_fail']}"
        )
    if not _same(minimal, v["expected_minimal"]):
        problems.append(
            f"{cid}: replay minimal mismatch: got {minimal}, "
            f"corpus {v['expected_minimal']}"
        )
    return problems


def _check_filter_exhausted(cid: str, v: dict[str, Any]) -> list[str]:
    max_retries = _int(v["max_retries"], "max_retries")
    calls = 0

    def never(_value: int) -> bool:
        nonlocal calls
        calls += 1
        return False

    verdict = check_property(
        0,
        gen_int(0, 10).filter(never, max_retries),
        lambda _x: True,
        PbtOptions(num_tests=1, ci=True),
    )
    problems = []
    if v["verdict"] != "fail" or verdict.passed:
        problems.append(f"{cid}: an unsatisfiable filter must be a fail verdict")
    if verdict.failure_id != v["failure_id"]:
        problems.append(
            f"{cid}: failure id {verdict.failure_id!r}, corpus {v['failure_id']!r}"
        )
    if calls != max_retries:
        problems.append(f"{cid}: filter sampled {calls} times, expected {max_retries}")
    return problems


def evaluate_pbt_corpus(
    cases: Sequence[Mapping[str, Any]],
    expected_cases: int | None = PBT_V1_CASE_COUNT,
) -> CorpusReport:
    """Evaluate every case; report how many ran and every problem found."""
    problems: list[str] = []
    evaluated = 0
    ids = [case.get("id") if isinstance(case, Mapping) else None for case in cases]
    if len(ids) != len(set(ids)):
        problems.append("duplicate or missing case id")
    if expected_cases is not None and len(cases) != expected_cases:
        problems.append(f"pbt-v1: expected {expected_cases} cases, got {len(cases)}")

    ops_seen: set[str] = set()
    registered_seen: set[str] = set()
    rejected_id_seen = False

    for case in cases:
        if not isinstance(case, Mapping):
            problems.append(f"case is not an object: {case!r}")
            continue
        cid, op = case.get("id"), case.get("op")
        if not isinstance(cid, str) or not cid:
            problems.append(f"case missing a string id: {dict(case)!r}")
            continue
        if op not in SHAPES:
            problems.append(f"{cid}: unknown op {op!r}")
            continue
        wanted = SHAPES[op] | {"id", "op"}
        if set(case) != wanted:
            problems.append(f"{cid}: fields {sorted(case)} != {sorted(wanted)}")
            continue
        try:
            v = {k: _decode(val) for k, val in case.items() if k not in ("id", "op")}
            if op in ("gen_primitive", "gen_bounded", "gen_bytes"):
                found = _check_gen(cid, op, v)
            elif op in ("shrink_int", "shrink_refinement"):
                found = _check_shrink_int(cid, op, v)
            elif op in ("shrink_list", "shrink_bytes"):
                found = _check_shrink_seq(cid, op, v)
            elif op == "replay_consistency":
                found = _check_replay(cid, v)
            elif op == "filter_exhausted":
                found = _check_filter_exhausted(cid, v)
            elif op == "failure_id_valid":
                candidate = v["input"]
                got = is_pbt_failure_id(candidate)
                found = (
                    []
                    if _same(got, v["valid"])
                    else [f"{cid}: failure id membership disagrees for {candidate!r}"]
                )
                if got:
                    registered_seen.add(candidate)
                else:
                    rejected_id_seen = True
            else:  # corpus_reject
                try:
                    parse_tfv(v["text"])
                except ValueError:
                    found = []
                else:
                    found = [
                        f"{cid}: corpus_reject text was accepted by the strict parser"
                    ]
        except Exception as err:  # malformed case or an engine that raised
            problems.append(
                f"{cid}: {op} could not be evaluated: {type(err).__name__}: {err}"
            )
            continue
        problems.extend(found)
        ops_seen.add(op)
        evaluated += 1

    missing_ops = set(SHAPES) - ops_seen
    if missing_ops:
        problems.append(f"pbt-v1: operations never exercised: {sorted(missing_ops)}")
    missing_ids = set(PBT_FAILURE_IDS) - registered_seen
    if missing_ids:
        problems.append(
            f"pbt-v1: registered failure ids missing: {sorted(missing_ids)}"
        )
    if not rejected_id_seen:
        problems.append("pbt-v1: failure_id_valid needs an unregistered-id rejection")
    return CorpusReport(evaluated=evaluated, problems=problems)


def run_pbt_corpus(
    cases: Sequence[Mapping[str, Any]],
    expected_cases: int | None = PBT_V1_CASE_COUNT,
) -> list[str]:
    """Problems found in ``cases`` (empty means the engine conforms)."""
    return evaluate_pbt_corpus(cases, expected_cases).problems
