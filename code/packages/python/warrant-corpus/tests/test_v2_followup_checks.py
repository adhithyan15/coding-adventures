"""VER04r section 16.21: [D73] (chunk ``calls``), R107 (d) (tags before levels).

R108, the ``types`` table checks, is in ``test_v2_lane_checks.py`` with the other
bad tables, where every context that reads a table is run over it.
"""

from __future__ import annotations

import copy
import hashlib

import pytest
from test_v2_chunks import (
    A_AND_B,
    REFINED,
    SUBJECT,
    bare,
    lit_u8,
    refuse_file,
    sealed,
)

from coding_adventures_warrant_corpus.v2 import chunks as C
from coding_adventures_warrant_corpus.v2.errors import Refusal


# -- [D73] -------------------------------------------------------------------------------
def with_calls(data: bytes, line: int, delta: int) -> bytes:
    lines = data.decode().split("\n")[:-1]
    fields = lines[line].split(" ")
    fields[3] = str(int(fields[3]) + delta)
    lines[line] = " ".join(fields)
    return ("\n".join(lines) + "\n").encode()


@pytest.mark.parametrize(("line", "delta"), [(0, 1), (0, -1), (-1, 1)])
def test_chunk_calls_that_do_not_sum_to_expected_calls_are_chunk_file(line, delta):
    corpus, row, data = sealed(A_AND_B, **REFINED)
    assert row["expected_calls"] > 0
    altered = with_calls(data, line if line >= 0 else row["chunks"] - 1, delta)
    # the digest is recomputed, so the sum is the only check that can fail
    row = row | {"chunks_sha256": hashlib.sha256(altered).hexdigest()}
    suite = C.build_suite(corpus, row)
    with pytest.raises(Refusal) as caught:
        C.check_chunk_file(altered, suite)
    assert caught.value.code == "chunk-file"
    assert "expected_calls" in caught.value.message


def test_a_predicate_suite_is_held_to_the_calls_sum_too():
    corpus, row, data = sealed(BINDER_U8)
    assert "expected_sha256" not in row and row["expected_calls"] > 0
    altered = with_calls(data, 0, 1)
    row = row | {"chunks_sha256": hashlib.sha256(altered).hexdigest()}
    with pytest.raises(Refusal) as caught:
        C.check_chunk_file(altered, C.build_suite(corpus, row))
    assert caught.value.code == "chunk-file"
    assert "expected_calls" in caught.value.message


@pytest.mark.parametrize("field", ["expected_calls", "cases", "discarded"])
def test_a_count_that_is_a_float_or_a_bool_is_chunk_file(field):
    corpus, row, data = sealed(BINDER_U8)
    for fake in (float(row[field]), bool(row[field]) if row[field] in (0, 1) else None):
        if fake is None:
            continue
        edited = row | {field: fake}
        with pytest.raises(Refusal) as caught:
            C.check_chunk_file(data, C.build_suite(corpus, edited))
        assert caught.value.code == "chunk-file"


def test_a_row_whose_expected_calls_moved_is_chunk_file_too():
    refuse_file(sealed(A_AND_B, **REFINED), expected_calls=1)


def test_a_file_whose_calls_sum_to_the_row_is_accepted():
    corpus, row, data = sealed(A_AND_B, **REFINED)
    assert len(C.check_chunk_file(data, C.build_suite(corpus, row))) == row["chunks"]


# -- R107 (d) ---------------------------------------------------------------------------
FIXED_TAG = [
    "some",
    ["var", "b"],
]  # ir.data: a level this runner lacks (it implements ir.fixed since S10e)
BINDER = [("b", "U8", {"d": "u8_all", "size": 256})]
BINDER_U8 = BINDER
LIVE_ELSEWHERE = ["eq", FIXED_TAG, lit_u8(0)]
NEEDS_FIXED = ["ir.core", "ir.data"]


def build(**edits):
    corpus, row = bare(BINDER)
    row = copy.deepcopy(row) | edits
    return C.build_suite(corpus, row)


def test_a_level_missing_tag_alone_is_still_level_missing():
    # the row says it needs ir.data, so the lane records it not-live
    with pytest.raises(Refusal) as caught:
        build(admit=["eq", FIXED_TAG, lit_u8(0)], requires=NEEDS_FIXED)
    assert caught.value.code == "level-missing"


@pytest.mark.parametrize(
    ("edits", "why"),
    [
        (
            {"admit": ["eq", FIXED_TAG, lit_u8(0)], "body": ["wibble"]},
            "unknown in body",
        ),
        (
            {"admit": ["eq", FIXED_TAG, SUBJECT]},
            "subject_op beside a missing level in admit",
        ),
        (
            {"admit": ["eq", FIXED_TAG, lit_u8(0)], "body": ["eq", FIXED_TAG, ["x"]]},
            "unknown in body after a missing level in both",
        ),
    ],
)
def test_a_tag_refusal_takes_precedence_over_level_missing(edits, why):
    with pytest.raises(Refusal) as caught:
        build(**edits)
    assert caught.value.code == "corpus", why


def test_the_oracle_sides_are_scanned_too():
    corpus, row = bare(
        BINDER,
        expected=["model_call", "m", [["var", "b"]]],
        result_type="U8",
    )
    row = copy.deepcopy(row)
    row["admit"] = ["eq", FIXED_TAG, lit_u8(0)]
    for side, node in (
        ("expected", ["wibble"]),
        ("expected", SUBJECT),
        ("subject", ["wibble"]),
    ):
        edited = copy.deepcopy(row)
        edited["oracle"][side] = node
        edited["body"] = [
            "eq",
            edited["oracle"]["subject"],
            edited["oracle"]["expected"],
        ]
        with pytest.raises(Refusal) as caught:
            C.build_suite(corpus, edited)
        assert caught.value.code == "corpus", (side, node)


def test_a_malformed_oracle_is_still_refused_not_crashed():
    corpus, row = bare(BINDER)
    row = copy.deepcopy(row) | {"oracle": 7}
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row)
    assert caught.value.code == "corpus"


def test_the_scan_is_a_no_op_on_a_good_suite():
    assert build().m == 1


def nested_not(times):
    node = ["lit", True]
    for _ in range(times):
        node = ["not", node]
    return node


@pytest.mark.parametrize(
    "body",
    [nested_not(70), ["eq", ["model_call", "nope", [["var", "b"]]], lit_u8(0)]],
    ids=["depth over 64", "model_call to an unknown model"],
)
def test_depth_and_model_order_are_checked_before_level_missing(body):
    with pytest.raises(Refusal) as caught:
        build(admit=LIVE_ELSEWHERE, body=body, requires=NEEDS_FIXED)
    assert caught.value.code == "corpus"


def test_a_not_live_reference_suite_with_a_legal_subject_stays_level_missing():
    corpus, row = bare(
        BINDER, expected=["model_call", "m", [["var", "b"]]], result_type="U8"
    )
    row = copy.deepcopy(row) | {"admit": LIVE_ELSEWHERE, "requires": NEEDS_FIXED}
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row)
    assert caught.value.code == "level-missing"
