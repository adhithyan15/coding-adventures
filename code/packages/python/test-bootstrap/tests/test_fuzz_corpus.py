"""fuzz-v1 conformance for the Python engine, with negative controls.

The committed corpus ``code/specs/fixtures/fuzz-v1/cases.json`` is loaded in
place (no copy) by walking up from this file to the repository root.  A runner
that cannot fail proves nothing, so most of this file corrupts a real case and
checks that ``run_fuzz_corpus`` notices.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pytest

from coding_adventures_test_bootstrap import fuzz, fuzz_corpus, prng
from coding_adventures_test_bootstrap.fuzz_corpus import (
    FUZZ_V1_CASE_COUNT,
    SHAPES,
    evaluate_fuzz_corpus,
    run_fuzz_corpus,
)


def _repo_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "code/specs/fixtures/fuzz-v1/cases.json").is_file():
            return parent
    raise AssertionError("repository root with the fuzz-v1 fixture not found")


FIXTURE = _repo_root() / "code/specs/fixtures/fuzz-v1/cases.json"
DOCUMENT = json.loads(FIXTURE.read_text(encoding="utf-8"))
CASES: list[dict[str, Any]] = DOCUMENT["cases"]


def _first(op: str, predicate=lambda case: True) -> dict[str, Any]:
    for case in CASES:
        if case["op"] == op and predicate(case):
            return copy.deepcopy(case)
    raise AssertionError(f"no {op} case in the corpus")


def _int(n: int) -> dict[str, str]:
    return {"kind": "int", "value": str(n)}


def _bytes(hexed: str) -> dict[str, str]:
    return {"kind": "bytes", "hex": hexed}


def _problems(*cases: dict[str, Any], expected: int | None = None) -> list[str]:
    return run_fuzz_corpus(list(cases), expected_cases=expected)


# --- the positive result -----------------------------------------------------


def test_fixture_is_the_committed_25_case_corpus() -> None:
    assert DOCUMENT["schema_version"] == 1
    assert len(CASES) == 25 == FUZZ_V1_CASE_COUNT
    assert {case["op"] for case in CASES} == set(SHAPES)


def test_engine_conforms_to_every_fuzz_v1_case() -> None:
    report = evaluate_fuzz_corpus(CASES)
    assert report.problems == []
    assert report.evaluated == len(CASES) == 25
    assert run_fuzz_corpus(CASES) == []


# --- unknown / malformed input is a problem, never a skip --------------------


def test_unknown_op_is_a_problem_not_a_skip() -> None:
    bogus = {"id": "fuzz/bogus", "op": "mutate_quantum"}
    assert any("unknown op" in p for p in _problems(bogus))
    report = evaluate_fuzz_corpus(CASES + [bogus])
    assert report.evaluated == 25 and report.problems
    assert any("unknown op" in p for p in _problems({"id": "x", "op": ["a"]}))


@pytest.mark.parametrize("field", ["input", "bit_offset", "count", "output"])
def test_missing_field_is_a_problem(field: str) -> None:
    case = _first("mutate_bit_flip")
    del case[field]
    assert any("fields" in p for p in _problems(case))


def test_extra_field_is_a_problem() -> None:
    case = _first("mutate_bit_flip")
    case["bonus"] = True
    assert any("fields" in p for p in _problems(case))


@pytest.mark.parametrize("bad", ["007", "+5", "-0", " 5", "1_0", "", "0x10"])
def test_non_canonical_int_text_is_a_problem(bad: str) -> None:
    case = _first("mutate_bit_flip")
    case["bit_offset"] = {"kind": "int", "value": bad}
    assert any("could not be evaluated" in p for p in _problems(case))


@pytest.mark.parametrize("bad", ["FF", "f", "ff ", "0G"])
def test_non_canonical_bytes_hex_is_a_problem(bad: str) -> None:
    case = _first("mutate_bit_flip")
    case["output"] = _bytes(bad)
    assert any("could not be evaluated" in p for p in _problems(case))


def test_wrongly_typed_fields_are_problems() -> None:
    cases = []
    c = _first("mutate_bit_flip")
    c["input"] = _int(5)
    cases.append((c, "must be bytes"))
    c = _first("mutate_arithmetic")
    c["endian"] = _int(1)
    cases.append((c, "must be a string"))
    c = _first("mutate_bit_flip")
    c["count"] = True
    cases.append((c, "must be an int"))
    c = _first("corpus_admit")
    c["trace_edges"] = "oops"
    cases.append((c, "list of ints"))
    c = _first("corpus_admit")
    c["admitted"] = _int(1)
    cases.append((c, "must be a bool"))
    c = _first("crash_dedup")
    c["crash_a_frames"] = [_int(1)]
    cases.append((c, "list of strings"))
    c = _first("crash_dedup")
    c["crash_a_kind"] = _int(1)
    cases.append((c, "must be a string"))
    for case, needle in cases:
        assert any(needle in p for p in _problems(case)), (case["id"], needle)


def test_unsupported_endian_in_the_fixture_is_a_problem() -> None:
    case = _first("mutate_arithmetic")
    case["endian"] = "middle"
    assert any("endian" in p for p in _problems(case))


def test_non_object_and_idless_cases_are_problems() -> None:
    assert _problems("nope")  # type: ignore[arg-type]
    assert _problems({"op": "mutate_bit_flip"})
    dup = _first("failure_id_valid")
    assert any("duplicate" in p for p in _problems(dup, dup))


def test_unhashable_or_non_string_ids_are_problems() -> None:
    for bad in ([1], {"a": 1}, 5, None, 1.5):
        case = _first("failure_id_valid")
        case["id"] = bad
        problems = _problems(case)
        assert problems and any("id" in p for p in problems), bad
    a, b = _first("failure_id_valid"), _first("failure_id_valid")
    a["id"] = b["id"] = ["same"]
    assert _problems(a, b)


# --- truncation --------------------------------------------------------------


def test_truncated_case_list_is_a_problem() -> None:
    for cut in (1, 5, 24):
        assert run_fuzz_corpus(CASES[:cut]), f"truncation to {cut} not detected"
    assert any("expected 25 cases" in p for p in run_fuzz_corpus(CASES[:-1]))


def test_dropping_every_case_of_an_op_is_a_problem() -> None:
    for op in SHAPES:
        kept = [c for c in CASES if c["op"] != op]
        problems = run_fuzz_corpus(kept, expected_cases=None)
        assert any("never exercised" in p and op in p for p in problems), op


def test_failure_id_acceptance_and_rejection_are_both_required() -> None:
    kept = [c for c in CASES if c.get("input") != "fuzz-crash"]
    assert any("acceptance" in p for p in run_fuzz_corpus(kept, None))
    kept = [c for c in CASES if c.get("input") != "fuzz-arbitrary-error"]
    assert any("rejection" in p for p in run_fuzz_corpus(kept, None))


# --- negative controls: a corrupted expectation must be reported -------------

CORRUPTIONS: list[tuple[str, Any]] = []


def _register(name: str):
    def deco(fn):
        CORRUPTIONS.append((name, fn))
        return fn

    return deco


def _flip_output(op: str, name: str, predicate=lambda c: True) -> None:
    def build(_: None) -> dict[str, Any]:
        case = _first(op, predicate)
        hexed = case["output"]["hex"]
        case["output"]["hex"] = ("ff" if hexed[:2] != "ff" else "00") + hexed[2:]
        return case

    _register(f"{op}/{name}")(build)


for _op in (
    "mutate_bit_flip",
    "mutate_byte_flip",
    "mutate_arithmetic",
    "mutate_boundary",
    "mutate_block_delete",
    "mutate_block_insert",
    "mutate_block_duplicate",
    "mutate_splice",
    "mutate_sequence",
):
    _flip_output(_op, "flip an output byte")


@_register("mutate_bit_flip/lengthened output")
def _c_len(_: None) -> dict[str, Any]:
    case = _first("mutate_bit_flip")
    case["output"]["hex"] += "00"
    return case


@_register("mutate_bit_flip/wrong offset")
def _c_bit_off(_: None) -> dict[str, Any]:
    case = _first("mutate_bit_flip", lambda c: c["bit_offset"]["value"] == "9")
    case["bit_offset"] = _int(8)
    return case


@_register("mutate_byte_flip/wrong count")
def _c_byte_count(_: None) -> dict[str, Any]:
    case = _first("mutate_byte_flip", lambda c: c["count"]["value"] == "2")
    case["count"] = _int(1)
    return case


@_register("mutate_arithmetic/wrong endian")
def _c_endian(_: None) -> dict[str, Any]:
    case = _first("mutate_arithmetic", lambda c: c["endian"] == "big")
    case["endian"] = "little"
    return case


@_register("mutate_arithmetic/wrong delta")
def _c_delta(_: None) -> dict[str, Any]:
    case = _first("mutate_arithmetic")
    case["delta"] = _int(int(case["delta"]["value"]) + 1)
    return case


@_register("mutate_boundary/wrong value")
def _c_val(_: None) -> dict[str, Any]:
    case = _first("mutate_boundary", lambda c: c["width"]["value"] == "4")
    case["val"] = _int(int(case["val"]["value"]) - 1)
    return case


@_register("mutate_sequence/wrong seed")
def _c_seed(_: None) -> dict[str, Any]:
    case = _first("mutate_sequence")
    case["seed"] = _int(int(case["seed"]["value"]) + 1)
    return case


@_register("mutate_sequence/wrong steps")
def _c_steps(_: None) -> dict[str, Any]:
    case = _first("mutate_sequence")
    case["steps"] = _int(int(case["steps"]["value"]) + 1)
    return case


@_register("corpus_admit/flip admitted")
def _c_admit(_: None) -> dict[str, Any]:
    case = _first("corpus_admit")
    case["admitted"] = not case["admitted"]
    return case


@_register("corpus_admit/wrong new_edges")
def _c_new(_: None) -> dict[str, Any]:
    case = _first("corpus_admit", lambda c: c["admitted"] is True)
    case["new_edges"] = _int(int(case["new_edges"]["value"]) + 1)
    return case


@_register("crash_dedup/duplicate claimed distinct")
def _c_dup(_: None) -> dict[str, Any]:
    case = _first("crash_dedup", lambda c: c["is_duplicate"] is True)
    case["is_duplicate"] = False
    return case


@_register("crash_dedup/distinct claimed duplicate")
def _c_distinct(_: None) -> dict[str, Any]:
    case = _first("crash_dedup", lambda c: c["is_duplicate"] is False)
    case["is_duplicate"] = True
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


@_register("private_reporting/wrong tap line")
def _c_tap(_: None) -> dict[str, Any]:
    case = _first("private_reporting")
    case["tap_line"] = case["tap_line"] + " raw=deadbeef"
    return case


@_register("private_reporting/claims a leak")
def _c_leak(_: None) -> dict[str, Any]:
    case = _first("private_reporting")
    case["contains_raw_payload"] = True
    return case


@_register("private_reporting/wrong hash")
def _c_hash(_: None) -> dict[str, Any]:
    case = _first("private_reporting")
    case["sig_hash"] = "00000000"
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
    assert run_fuzz_corpus([corrupted], expected_cases=None) != [], "undetected"
    swapped = [corrupted if c["id"] == corrupted["id"] else c for c in CASES]
    assert run_fuzz_corpus(swapped) != []


def test_every_op_has_at_least_one_negative_control() -> None:
    covered = {name.split("/")[0] for name, _ in CORRUPTIONS}
    assert set(SHAPES) <= covered, set(SHAPES) - covered


# --- engine sabotage: the runner must catch a broken engine ------------------

_SABOTAGES = {
    "bit_flip": lambda d, o, c=1: fuzz.bit_flip(d, o + 1, c),
    "byte_flip": lambda d, o, c=1: fuzz.byte_flip(d, o, c + 1),
    "byte_arithmetic": lambda d, o, dl, w=1, e="little": fuzz.byte_arithmetic(
        d, o, dl, w, "big" if e == "little" else "little"
    ),
    "boundary_insert": lambda d, o, w, v, e="little": fuzz.boundary_insert(
        d, o, w, v ^ 1, e
    ),
    "block_delete": lambda d, o, ln: fuzz.block_delete(d, o, ln + 1),
    "block_insert": lambda d, o, ins: fuzz.block_insert(d, o + 1, ins),
    "block_duplicate": lambda d, s, ln, ds: fuzz.block_duplicate(d, s, ln, ds + 1),
    "splice": lambda t, dn, ts, ds: fuzz.splice(t, dn, ts + 1, ds),
    "mutate_sequence": lambda d, s, n: fuzz.mutate_sequence(d, s + 1, n),
    "corpus_admit": lambda e, t: (True, 1),
    "is_duplicate_crash": lambda *a: True,
    "is_fuzz_failure_id": lambda _x: True,
    "format_tap_failure": lambda n, t, f, s: f"not ok {n} - {t} [{f}:{s}] {t}",
    "parse_tfv": lambda text: [],
}


@pytest.mark.parametrize("name", sorted(_SABOTAGES))
def test_runner_catches_a_sabotaged_engine(
    name: str, monkeypatch: pytest.MonkeyPatch
) -> None:
    assert run_fuzz_corpus(CASES) == []
    monkeypatch.setattr(fuzz_corpus, name, _SABOTAGES[name])
    assert run_fuzz_corpus(CASES), f"sabotaged {name} was not caught"


def test_prng_sabotage_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    real = prng.SplitMix64.bounded
    monkeypatch.setattr(
        prng.SplitMix64, "bounded", lambda self, n: real(self, n) ^ (1 if n == 7 else 0)
    )
    assert run_fuzz_corpus(CASES)


def test_engine_invariants_are_checked_independently_of_expected_values(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # An engine that changes the length of a flip, or whose splice is the wrong
    # size, is reported even when the corpus bytes are rewritten to agree.
    case = _first("mutate_bit_flip")
    monkeypatch.setattr(fuzz_corpus, "bit_flip", lambda d, o, c=1: d + b"\x00")
    case["output"] = _bytes(case["input"]["hex"] + "00")
    assert any("changed the length" in p for p in _problems(case))
    monkeypatch.undo()
    case = _first("mutate_splice")
    monkeypatch.setattr(fuzz_corpus, "splice", lambda t, d, a, b: b"\x01")
    case["output"] = _bytes("01")
    assert any("prefix + suffix" in p for p in _problems(case))
    monkeypatch.undo()
    case = _first("corpus_admit", lambda c: c["admitted"] is True)
    monkeypatch.setattr(fuzz_corpus, "corpus_admit", lambda e, t: (False, 0))
    case["admitted"], case["new_edges"] = False, _int(0)
    assert any("set difference" in p for p in _problems(case))
    monkeypatch.undo()
    case = _first("private_reporting")
    monkeypatch.setattr(
        fuzz_corpus, "format_tap_failure", lambda n, t, f, s: case["tap_line"] + "\n"
    )
    assert any("more than one line" in p for p in _problems(case))


def test_registered_ids_the_engine_rejects_are_reported(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(fuzz_corpus, "is_fuzz_failure_id", lambda x: x == "fuzz-crash")
    assert any("rejects registered ids" in p for p in run_fuzz_corpus(CASES))


# --- CWE-400: fixture-controlled sizes are capped ----------------------------


def test_oversized_steps_are_a_problem_not_a_hang() -> None:
    case = _first("mutate_sequence")
    case["steps"] = _int(10**9)
    assert any("outside [0," in p for p in _problems(case))
    case["steps"] = _int(-1)
    assert any("outside [0," in p for p in _problems(case))


def test_oversized_values_are_problems() -> None:
    case = _first("corpus_admit")
    case["existing_edges"] = [_int(0)] * 10_001
    assert any("list exceeds" in p for p in _problems(case))
    case = _first("mutate_bit_flip")
    case["input"] = _bytes("00" * 10_001)
    assert any("bytes exceed" in p for p in _problems(case))
    case = _first("mutate_boundary")
    case["val"] = _int(10**1001)
    assert any("digits" in p for p in _problems(case))


def test_huge_but_legal_integers_stay_fast() -> None:
    case = _first("mutate_bit_flip")
    case["bit_offset"] = _int(10**900)
    case["count"] = _int(10**900)
    case["input"] = _bytes("00" * 5000)
    case["output"] = _bytes("00" * 5000)
    problems = _problems(case)  # a mismatch is fine; the point is it returns
    assert isinstance(problems, list)
