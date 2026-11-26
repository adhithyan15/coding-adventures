"""VER04r section 16.23: [D77] and R119 to R127 in the Python lane.

* R120: an ``env`` type the reader refuses is ``ill-typed`` in ``v2-ir``; an entry
  that is not ``{type: text, value: V}`` stays a kit error.
* R121: a ``v2-fault`` op row's parameter and result types are read with the
  table (order 1), before ``level-missing``.
* R119 / [D77]: ``unknown-tag``, ``depth`` and ``model-order`` come after
  ``level-missing`` and before ``subject-forbidden`` wherever they stand.
* R126: a malformed fault row, an undefined level name in ``requires``, is a kit
  error read before the table.
* R127: nothing in this lane today (the corpus reader is S6b); see the last test.
"""

from __future__ import annotations

import pytest
from test_v2_ir import case
from test_v2_lane_checks import fault_case, fault_row

from coding_adventures_warrant_corpus.v2 import STANDARD
from coding_adventures_warrant_corpus.v2.errors import KitError, Refusal
from coding_adventures_warrant_corpus.v2.ir import scan_tags
from coding_adventures_warrant_corpus.v2.kit import outcome_fault, outcome_ir

RECORD_Q = {"Q": {"kind": "record", "fields": [["a", "U8"]]}}


def ir_error(**fields):
    return outcome_ir(case(["lit", True], **fields), STANDARD)


# ---------------------------------------------------------------------------
# R120
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "text", ["Int", "Option<Int>", "Ghost", "Result<U8,Q>", "Bytes<07>", "U8<", ""]
)
def test_an_env_type_the_reader_refuses_is_ill_typed(text):
    got = ir_error(
        env=[["x", {"type": text, "value": "1"}]],
        types={"Q": RECORD_Q["Q"]},
    )
    assert got == {"error": "ill-typed"}


def test_an_env_entry_naming_an_error_type_is_still_ill_typed():
    got = ir_error(
        env=[["x", {"type": "E", "value": "1"}]],
        types={"E": {"kind": "error", "cases": ["A"]}},
    )
    assert got == {"error": "ill-typed"}


@pytest.mark.parametrize(
    "entry",
    [
        {"value": "1"},
        {"type": 7, "value": "1"},
        {"type": None, "value": "1"},
        {"type": "U8"},
        "U8",
    ],
)
def test_an_env_entry_that_is_not_type_and_value_stays_a_kit_error(entry):
    with pytest.raises(KitError):
        ir_error(env=[["x", entry]])


def test_an_env_value_that_does_not_decode_stays_a_kit_error():
    with pytest.raises(KitError):
        ir_error(env=[["x", {"type": "U8", "value": "999"}]])


# ---------------------------------------------------------------------------
# R121
# ---------------------------------------------------------------------------
def op_row(param="U8", ret="U8"):
    return {"params": [["a", param]], "ret": ret, "requires": ["ir.core"]}


@pytest.mark.parametrize("bad", ["Int", "Ghost", "E", "Result<U8,Q>"])
@pytest.mark.parametrize("where", ["param", "ret"])
def test_an_op_row_type_refusal_is_ill_typed(bad, where):
    types = {
        "E": {"kind": "error", "cases": ["A"]},
        "Q": RECORD_Q["Q"],
    }
    row = op_row(param=bad) if where == "param" else op_row(ret=bad)
    got = outcome_fault(fault_case(op=row, types=types), STANDARD)
    assert got == {"error": "ill-typed"}


@pytest.mark.parametrize(
    "edits",
    [
        {"levels": []},
        {"fault": fault_row(requires=["ir.fixed"])},
    ],
)
def test_an_op_row_type_refusal_comes_before_level_missing(edits):
    got = outcome_fault(fault_case(op=op_row(param="Int"), **edits), STANDARD)
    assert got == {"error": "ill-typed"}


def test_an_op_row_type_that_needs_a_level_is_level_missing():
    got = outcome_fault(fault_case(op=op_row(param="Option<U8>")), STANDARD)
    assert got == {"error": "level-missing"}


def test_an_op_row_refusal_comes_before_the_three_codes_and_subject_forbidden():
    for expr in (["bogus"], ["model_call", "m", []], ["subject_op", "bump", []]):
        row = fault_row(expr=expr)
        got = outcome_fault(fault_case(fault=row, op=op_row(ret="Int")), STANDARD)
        assert got == {"error": "ill-typed"}


# ---------------------------------------------------------------------------
# R119 / [D77]
# ---------------------------------------------------------------------------
def deep(n):
    """``n`` nested ``concat`` nodes over a ``var``: ``n + 1`` nodes deep."""
    node = ["var", "a"]
    for _ in range(n):
        node = ["concat", node, ["var", "a"]]
    return node


def code(expr, requires=("ir.core",), **edits):
    row = fault_row(expr=expr, requires=list(requires))
    return outcome_fault(fault_case(fault=row, **edits), STANDARD)["error"]


def both(*parts):
    """A core-only node holding every part (an ``eq`` chain)."""
    node = parts[0]
    for part in parts[1:]:
        node = ["eq", node, part]
    return node


SUBJECT = ["subject_op", "bump", [["var", "a"]]]


def test_the_three_codes_are_outcomes_of_v2_fault():
    assert code(["bogus", ["var", "a"]]) == "unknown-tag"
    assert code(["model_call", "m", [["var", "a"]]]) == "model-order"
    assert code(deep(64)) == "depth"


def test_a_nesting_of_64_is_accepted_by_the_scan():
    # 63 concats over a var are 64 nodes deep; 64 concats are 65 (R119 a).
    assert code(deep(63)) != "depth"
    assert code(deep(64)) == "depth"


def test_level_missing_comes_before_the_three_codes():
    for expr in (["bogus"], ["model_call", "m", []], deep(64)):
        assert code(expr, requires=["ir.fixed"]) == "level-missing"
        assert code(expr, levels=[]) == "level-missing"


@pytest.mark.parametrize("static", ["unknown-tag", "depth", "model-order"])
@pytest.mark.parametrize("subject_first", [True, False])
def test_the_three_codes_come_before_subject_forbidden_wherever_they_stand(
    static, subject_first
):
    bad = {
        "unknown-tag": ["bogus"],
        "depth": deep(64),
        "model-order": ["model_call", "m", []],
    }[static]
    parts = [SUBJECT, bad] if subject_first else [bad, SUBJECT]
    assert code(both(*parts)) == static


def test_subject_forbidden_comes_before_the_typing_of_the_row():
    # `not` of a U8 is ill-typed; the subject call beside it is found first.
    assert code(both(["not", ["var", "a"]], SUBJECT)) == "subject-forbidden"
    assert code(["not", ["var", "a"]]) == "ill-typed"


@pytest.mark.parametrize("static", ["unknown-tag", "depth", "model-order"])
def test_the_three_codes_come_before_the_typing_of_the_row(static):
    bad = {
        "unknown-tag": ["bogus"],
        "depth": deep(70),
        "model-order": ["model_call", "m", []],
    }[static]
    assert code(both(["not", ["var", "a"]], bad)) == static


def test_a_pre_rewrites_static_codes_come_before_another_rewrites_typing():
    op = {"params": [["a", "U8"], ["b", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    row = {
        "fault": "f",
        "op": "bump",
        "kind": "pre",
        "rewrites": [
            {"param": "a", "expr": ["not", ["var", "a"]]},
            {"param": "b", "expr": ["bogus"]},
        ],
        "requires": ["ir.core"],
    }
    case_ = fault_case(fault=row, op=op, args=["1", "2"])
    assert outcome_fault(case_, STANDARD) == {"error": "unknown-tag"}


def test_scan_tags_puts_model_order_before_subject_forbidden():
    tree = both(SUBJECT, ["model_call", "m", []])
    with pytest.raises(Refusal) as caught:
        scan_tags(tree, "fault", models=[])
    assert caught.value.code == "model-order"


# ---------------------------------------------------------------------------
# R126
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "row",
    [
        fault_row(extra=1),
        fault_row(kind="swap"),
        {k: v for k, v in fault_row().items() if k != "expr"},
        {k: v for k, v in fault_row().items() if k != "requires"},
        fault_row(requires=["ir.bogus"]),
        fault_row(requires="ir.core"),
        fault_row(op=3),
        "not a row",
    ],
)
def test_a_malformed_fault_row_is_a_kit_error(row):
    with pytest.raises(KitError):
        outcome_fault(fault_case(fault=row), STANDARD)


def test_a_malformed_row_is_read_before_the_table_and_the_op_row():
    bad_table = {"R": {"kind": "record", "fields": [["me", "R"]]}}
    with pytest.raises(KitError):
        outcome_fault(
            fault_case(fault=fault_row(extra=1), types=bad_table, op=op_row("Int")),
            STANDARD,
        )
    with pytest.raises(KitError):
        outcome_fault(
            fault_case(fault=fault_row(requires=["ir.bogus"]), types=bad_table),
            STANDARD,
        )


def test_a_defined_but_missing_level_is_still_an_outcome():
    got = outcome_fault(fault_case(fault=fault_row(requires=["ir.fixed"])), STANDARD)
    assert got == {"error": "level-missing"}
