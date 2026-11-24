"""Conformance runner for the shared ``fuzz-v1`` corpus (VER06 section 8).

``code/specs/fixtures/fuzz-v1/cases.json`` pins what every language's fuzz
engine must reproduce.  This module is the Python lane's *production* runner:
it takes the already-parsed JSON (``json.load(...)["cases"]``), evaluates every
case with **this package's own public engine functions**, and returns a list of
human-readable problems.  An empty list means "conforms".

It does no file I/O (the package is capability-free); the test that loads the
committed fixture is ``tests/test_fuzz_corpus.py``.

A runner that cannot report a problem is worse than none, so this one is strict
(the same rules as ``pbt_corpus``):

* a case has exactly the fields of its op, in canonical encodings (ints are
  decimal strings, bytes are even-length lowercase hex);
* an unknown op or a malformed value is a *problem*, never a silent skip;
* every op appears, a failure id is accepted and one is rejected, and the case
  count equals ``FUZZ_V1_CASE_COUNT``, so deleting cases is itself a problem;
* fixture-controlled sizes are capped (CWE-400): a hostile ``steps = 10**9``
  is a reported problem, not a hang.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from coding_adventures_test_bootstrap.fuzz import (
    FUZZ_FAILURE_IDS,
    bit_flip,
    block_delete,
    block_duplicate,
    block_insert,
    boundary_insert,
    byte_arithmetic,
    byte_flip,
    corpus_admit,
    format_tap_failure,
    is_duplicate_crash,
    is_fuzz_failure_id,
    mutate_sequence,
    splice,
)
from coding_adventures_test_bootstrap.pbt_corpus import (
    CORPUS_MAX_SIZE,
    CorpusReport,
    _CaseError,
    _decode,
    _int,
    _same,
    _size,
)
from coding_adventures_test_bootstrap.tfv import parse_tfv

#: ``fuzz-v1`` holds exactly this many cases.
FUZZ_V1_CASE_COUNT = 25

#: The closed field set of each op (besides ``id`` and ``op``).
SHAPES: dict[str, frozenset[str]] = {
    "mutate_bit_flip": frozenset({"input", "bit_offset", "count", "output"}),
    "mutate_byte_flip": frozenset({"input", "byte_offset", "count", "output"}),
    "mutate_arithmetic": frozenset(
        {"input", "byte_offset", "delta", "width", "endian", "output"}
    ),
    "mutate_boundary": frozenset(
        {"input", "byte_offset", "width", "val", "endian", "output"}
    ),
    "mutate_block_delete": frozenset({"input", "offset", "length", "output"}),
    "mutate_block_insert": frozenset({"input", "offset", "insert_bytes", "output"}),
    "mutate_block_duplicate": frozenset(
        {"input", "src_offset", "length", "dst_offset", "output"}
    ),
    "mutate_splice": frozenset(
        {"target", "donor", "target_split", "donor_split", "output"}
    ),
    "mutate_sequence": frozenset({"input", "seed", "steps", "output"}),
    "corpus_admit": frozenset(
        {"existing_edges", "trace_edges", "admitted", "new_edges"}
    ),
    "crash_dedup": frozenset(
        {
            "crash_a_kind",
            "crash_a_frames",
            "crash_b_kind",
            "crash_b_frames",
            "is_duplicate",
        }
    ),
    "failure_id_valid": frozenset({"input", "valid"}),
    "private_reporting": frozenset(
        {
            "target",
            "failure_id",
            "raw_payload",
            "sig_hash",
            "tap_line",
            "contains_raw_payload",
        }
    ),
    "corpus_reject": frozenset({"text"}),
}


def _bytes(value: Any, what: str) -> bytes:
    if not isinstance(value, bytes):
        raise _CaseError(f"{what} must be bytes")
    return value


def _text(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise _CaseError(f"{what} must be a string")
    return value


def _texts(value: Any, what: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        raise _CaseError(f"{what} must be a list of strings")
    return value


def _ints(value: Any, what: str) -> list[int]:
    if not isinstance(value, list):
        raise _CaseError(f"{what} must be a list of ints")
    return [_int(v, what) for v in value]


def _bool(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise _CaseError(f"{what} must be a bool")
    return value


def _compare(cid: str, op: str, got: bytes, v: dict[str, Any]) -> list[str]:
    want = _bytes(v["output"], "output")
    if _same(got, want):
        return []
    return [f"{cid}: {op} expected {want.hex()}, got {got.hex()}"]


def _check_mutator(cid: str, op: str, v: dict[str, Any]) -> list[str]:
    """The nine byte-in, byte-out mutators."""
    if op == "mutate_bit_flip":
        data = _bytes(v["input"], "input")
        got = bit_flip(
            data, _int(v["bit_offset"], "bit_offset"), _int(v["count"], "count")
        )
    elif op == "mutate_byte_flip":
        data = _bytes(v["input"], "input")
        got = byte_flip(
            data, _int(v["byte_offset"], "byte_offset"), _int(v["count"], "count")
        )
    elif op == "mutate_arithmetic":
        data = _bytes(v["input"], "input")
        got = byte_arithmetic(
            data,
            _int(v["byte_offset"], "byte_offset"),
            _int(v["delta"], "delta"),
            _int(v["width"], "width"),
            _text(v["endian"], "endian"),
        )
    elif op == "mutate_boundary":
        data = _bytes(v["input"], "input")
        got = boundary_insert(
            data,
            _int(v["byte_offset"], "byte_offset"),
            _int(v["width"], "width"),
            _int(v["val"], "val"),
            _text(v["endian"], "endian"),
        )
    elif op == "mutate_block_delete":
        data = _bytes(v["input"], "input")
        got = block_delete(
            data, _int(v["offset"], "offset"), _int(v["length"], "length")
        )
    elif op == "mutate_block_insert":
        data = _bytes(v["input"], "input")
        got = block_insert(
            data, _int(v["offset"], "offset"), _bytes(v["insert_bytes"], "insert_bytes")
        )
    elif op == "mutate_block_duplicate":
        data = _bytes(v["input"], "input")
        got = block_duplicate(
            data,
            _int(v["src_offset"], "src_offset"),
            _int(v["length"], "length"),
            _int(v["dst_offset"], "dst_offset"),
        )
    elif op == "mutate_splice":
        data = _bytes(v["target"], "target")
        donor = _bytes(v["donor"], "donor")
        ts, ds = (
            _int(v["target_split"], "target_split"),
            _int(v["donor_split"], "donor_split"),
        )
        got = splice(data, donor, ts, ds)
    else:  # mutate_sequence
        data = _bytes(v["input"], "input")
        got = mutate_sequence(data, _int(v["seed"], "seed"), _size(v["steps"], "steps"))
    problems = _compare(cid, op, got, v)
    # Engine invariants, independent of the corpus's expected bytes.
    if op in (
        "mutate_bit_flip",
        "mutate_byte_flip",
        "mutate_arithmetic",
        "mutate_boundary",
    ) and len(got) != len(data):
        problems.append(f"{cid}: {op} changed the length of its input")
    if op == "mutate_splice":
        ds_clamped = max(0, min(ds, len(donor)))
        ts_clamped = max(0, min(ts, len(data)))
        if len(got) != ts_clamped + len(donor) - ds_clamped:
            problems.append(f"{cid}: splice length is not prefix + suffix")
    return problems


def _check_corpus_admit(cid: str, v: dict[str, Any]) -> list[str]:
    existing, trace = (
        _ints(v["existing_edges"], "existing_edges"),
        _ints(v["trace_edges"], "trace_edges"),
    )
    admitted, new = corpus_admit(existing, trace)
    problems = []
    if not _same(admitted, _bool(v["admitted"], "admitted")):
        problems.append(
            f"{cid}: corpus_admit admitted {admitted}, corpus {v['admitted']}"
        )
    if not _same(new, _int(v["new_edges"], "new_edges")):
        problems.append(f"{cid}: corpus_admit new_edges {new}, corpus {v['new_edges']}")
    # Invariant: admission is exactly "some trace edge is unseen".
    if admitted != bool(set(trace) - set(existing)):
        problems.append(f"{cid}: corpus_admit disagrees with set difference")
    return problems


def _check_crash_dedup(cid: str, v: dict[str, Any]) -> list[str]:
    got = is_duplicate_crash(
        _text(v["crash_a_kind"], "crash_a_kind"),
        _texts(v["crash_a_frames"], "crash_a_frames"),
        _text(v["crash_b_kind"], "crash_b_kind"),
        _texts(v["crash_b_frames"], "crash_b_frames"),
    )
    if _same(got, _bool(v["is_duplicate"], "is_duplicate")):
        return []
    return [f"{cid}: crash_dedup is_duplicate {got}, corpus {v['is_duplicate']}"]


def _check_failure_id(cid: str, v: dict[str, Any]) -> tuple[list[str], bool]:
    candidate = v["input"]
    got = is_fuzz_failure_id(candidate)
    ok = _same(got, _bool(v["valid"], "valid"))
    problems = (
        [] if ok else [f"{cid}: failure id membership disagrees for {candidate!r}"]
    )
    return problems, got


def _check_private_reporting(cid: str, v: dict[str, Any]) -> list[str]:
    raw = _bytes(v["raw_payload"], "raw_payload")
    line = format_tap_failure(
        1,
        _text(v["target"], "target"),
        _text(v["failure_id"], "failure_id"),
        _text(v["sig_hash"], "sig_hash"),
    )
    problems = []
    if line != _text(v["tap_line"], "tap_line"):
        problems.append(f"{cid}: tap_line expected {v['tap_line']!r}, got {line!r}")
    leaked = bool(raw) and raw.hex() in line
    if not _same(leaked, _bool(v["contains_raw_payload"], "contains_raw_payload")):
        problems.append(f"{cid}: privacy verdict wrong, payload leaked: {leaked}")
    if "\n" in line or "\r" in line:
        problems.append(f"{cid}: failure line spans more than one line")
    return problems


def evaluate_fuzz_corpus(
    cases: Sequence[Mapping[str, Any]],
    expected_cases: int | None = FUZZ_V1_CASE_COUNT,
) -> CorpusReport:
    """Evaluate every case; report how many ran and every problem found."""
    problems: list[str] = []
    evaluated = 0
    # Ids are compared as strings only: an unhashable id must be a reported
    # problem, not a TypeError out of ``set()``.
    ids = [
        case.get("id")
        if isinstance(case, Mapping) and isinstance(case.get("id"), str)
        else None
        for case in cases
    ]
    if len(ids) != len(set(ids)):
        problems.append("duplicate or missing case id")
    if expected_cases is not None and len(cases) != expected_cases:
        problems.append(f"fuzz-v1: expected {expected_cases} cases, got {len(cases)}")

    ops_seen: set[str] = set()
    accepted_id_seen = rejected_id_seen = False

    for case in cases:
        if not isinstance(case, Mapping):
            problems.append(f"case is not an object: {case!r}")
            continue
        cid, op = case.get("id"), case.get("op")
        if not isinstance(cid, str) or not cid:
            problems.append(f"case missing a string id: {dict(case)!r}")
            continue
        if not isinstance(op, str) or op not in SHAPES:
            problems.append(f"{cid}: unknown op {op!r}")
            continue
        wanted = SHAPES[op] | {"id", "op"}
        if set(case) != wanted:
            problems.append(f"{cid}: fields {sorted(case)} != {sorted(wanted)}")
            continue
        try:
            v = {k: _decode(val) for k, val in case.items() if k not in ("id", "op")}
            if op.startswith("mutate_"):
                found = _check_mutator(cid, op, v)
            elif op == "corpus_admit":
                found = _check_corpus_admit(cid, v)
            elif op == "crash_dedup":
                found = _check_crash_dedup(cid, v)
            elif op == "failure_id_valid":
                found, accepted = _check_failure_id(cid, v)
                accepted_id_seen |= accepted
                rejected_id_seen |= not accepted
            elif op == "private_reporting":
                found = _check_private_reporting(cid, v)
            else:  # corpus_reject
                try:
                    parse_tfv(_text(v["text"], "text"))
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
        problems.append(f"fuzz-v1: operations never exercised: {sorted(missing_ops)}")
    if not accepted_id_seen:
        problems.append("fuzz-v1: failure_id_valid needs a registered-id acceptance")
    if not rejected_id_seen:
        problems.append("fuzz-v1: failure_id_valid needs an unregistered-id rejection")
    unregistered = [fid for fid in FUZZ_FAILURE_IDS if not is_fuzz_failure_id(fid)]
    if unregistered:
        problems.append(f"fuzz-v1: engine rejects registered ids {unregistered}")
    return CorpusReport(evaluated=evaluated, problems=problems)


def run_fuzz_corpus(
    cases: Sequence[Mapping[str, Any]],
    expected_cases: int | None = FUZZ_V1_CASE_COUNT,
) -> list[str]:
    """Problems found in ``cases`` (empty means the engine conforms)."""
    return evaluate_fuzz_corpus(cases, expected_cases).problems


__all__ = [
    "CORPUS_MAX_SIZE",
    "FUZZ_V1_CASE_COUNT",
    "SHAPES",
    "evaluate_fuzz_corpus",
    "run_fuzz_corpus",
]
