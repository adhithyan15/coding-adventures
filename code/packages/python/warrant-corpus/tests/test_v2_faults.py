"""Fault wrappers (VER04r section 8): ``pre``, ``post`` and ``replace``, written once.

A fault is data in the corpus; the lane implements the three wrappers and nothing
per fault.  The kit pins the outcomes of 36 hand-picked faults; these tests pin the
*wrapper* contract with real callables: which arguments the subject receives, that a
``replace`` still makes (and counts) the call, and that every expression reads the
arguments as they were passed in.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.v2.errors import KitError, Refusal
from coding_adventures_warrant_corpus.v2.faults import apply_fault, compile_fault

OP = {"params": [["a", "U8"], ["b", "U8"]], "ret": "U8", "requires": ["ir.core"]}
TYPES: dict = {}
CORE = frozenset({"ir.core"})


def var(name):
    return ["var", name]


def lit(n):
    return ["lit", {"type": "U8", "value": str(n)}]


def post(expr):
    return {
        "fault": "f",
        "op": "op",
        "kind": "post",
        "expr": expr,
        "requires": ["ir.core"],
    }


def replace(expr):
    return {
        "fault": "f",
        "op": "op",
        "kind": "replace",
        "expr": expr,
        "requires": ["ir.core"],
    }


def pre(**rewrites):
    return {
        "fault": "f",
        "op": "op",
        "kind": "pre",
        "rewrites": [{"param": p, "expr": e} for p, e in rewrites.items()],
        "requires": ["ir.core"],
    }


class Subject:
    """A stand-in export that records every call and returns the sum of its arguments."""

    def __init__(self):
        self.calls = []

    def __call__(self, a, b):
        self.calls.append((a, b))
        return (a + b) % 256


def wrapped(fault, subject=None):
    compiled = compile_fault(fault, OP, TYPES, CORE)
    return compiled, compiled.wrap(subject or Subject())


# -- the three wrappers -----------------------------------------------------------------
def test_a_post_fault_rewrites_the_result_from_the_arguments_and_the_result():
    subject = Subject()
    _compiled, call = wrapped(post(["add", var("result"), var("a")]), subject)
    assert call(10, 20) == 40  # (10 + 20) + 10
    assert call(200, 100) == (300 % 256 + 200) % 256  # wraps: section 3
    assert subject.calls == [(10, 20), (200, 100)]


def test_a_replace_fault_still_makes_and_counts_the_call_but_distrusts_its_value():
    subject = Subject()
    _compiled, call = wrapped(replace(lit(99)), subject)
    assert call(1, 2) == 99 and call(3, 4) == 99
    assert subject.calls == [(1, 2), (3, 4)]


def test_a_pre_fault_rewrites_the_arguments_the_subject_receives():
    subject = Subject()
    _compiled, call = wrapped(pre(a=["add", var("a"), lit(1)]), subject)
    assert (
        call(10, 20) == 31
    )  # the subject saw (11, 20) and returns its result untouched
    assert subject.calls == [(11, 20)]


def test_pre_rewrites_are_simultaneous_so_a_swap_works():
    subject = Subject()
    _compiled, call = wrapped(pre(a=var("b"), b=var("a")), subject)
    call(3, 9)
    assert subject.calls == [(9, 3)]
    subject.calls.clear()
    _compiled, call = wrapped(
        pre(a=["add", var("a"), var("b")], b=["add", var("a"), var("b")]), subject
    )
    call(3, 9)
    assert subject.calls == [
        (12, 12)
    ]  # the second rewrite did not see the first's output


def test_a_pre_fault_leaves_unrewritten_arguments_alone():
    compiled = compile_fault(pre(b=lit(0)), OP, TYPES, CORE)
    assert compiled.rewrite_args([7, 8]) == [7, 0]


# -- the kit's outcome shapes --------------------------------------------------------------------
def test_apply_fault_reports_every_argument_for_pre_and_the_result_for_the_others():
    assert apply_fault(pre(b=lit(0)), OP, TYPES, CORE, ["7", "8"], "15") == {
        "args": ["7", "0"]
    }
    assert apply_fault(
        post(["add", var("result"), lit(1)]), OP, TYPES, CORE, ["1", "2"], "255"
    ) == {"result": "0"}
    assert apply_fault(replace(var("a")), OP, TYPES, CORE, ["1", "2"], "3") == {
        "result": "1"
    }


def test_a_result_is_written_as_the_ops_declared_type():
    nat_op = {"params": [["a", "Nat"]], "ret": "Nat", "requires": ["ir.core"]}
    fault = {
        "fault": "off-by-one",
        "op": "op",
        "kind": "post",
        "expr": ["add", var("result"), ["lit", {"type": "Nat", "value": "1"}]],
        "requires": ["ir.core"],
    }
    assert apply_fault(fault, nat_op, TYPES, CORE, ["5"], "6") == {"result": "7"}


def test_a_result_type_with_an_error_case_round_trips():
    types = {"E": {"kind": "error", "cases": ["BAD"]}}
    op = {"params": [["a", "Bytes"]], "ret": "Result<Bytes,E>", "requires": ["ir.core"]}
    fault = {
        "fault": "missing-error",
        "op": "op",
        "kind": "replace",
        "expr": ["if", ["is_err", var("result")], ["ok", var("a")], var("result")],
        "requires": ["ir.core"],
    }
    assert apply_fault(fault, op, types, CORE, ["0a"], {"err": "BAD"}) == {
        "result": {"ok": "0a"}
    }
    assert apply_fault(fault, op, types, CORE, ["0a"], {"ok": "ff"}) == {
        "result": {"ok": "ff"}
    }


# -- refusals -----------------------------------------------------------------------------------------
def refusal(fault, levels=CORE, op=OP):
    with pytest.raises(Refusal) as caught:
        compile_fault(fault, op, TYPES, levels)
    return caught.value.code


def test_a_row_that_is_not_a_fault_is_a_kit_error():
    # R126: a break of the case's own schema is no outcome.
    for row in ("fault", {"kind": "mystery"}):
        with pytest.raises(KitError):
            compile_fault(row, OP, TYPES, CORE)


def test_a_fault_that_needs_a_missing_level_is_level_missing_by_requires_or_by_tag():
    needs_fixed = post(["bit_xor", var("result"), lit(1)])
    assert (
        refusal(needs_fixed) == "level-missing"
    )  # the tag, though `requires` says core
    assert (
        refusal(needs_fixed | {"requires": ["ir.fixed"]}) == "level-missing"
    )  # the row's requires
    assert (
        refusal(post(lit(1)) | {"requires": ["ir.core", "ir.tuple"]}) == "level-missing"
    )
    assert refusal(post(lit(1)), levels=frozenset()) == "level-missing"


def test_a_fault_expression_never_calls_the_subject():
    call = ["subject_op", "op", [var("a"), var("b")]]
    assert refusal(post(call)) == "subject-forbidden"
    assert refusal(pre(a=call)) == "subject-forbidden"
    assert refusal(replace(["eq", call, lit(1)])) == "subject-forbidden"


def test_an_op_with_a_parameter_named_result_is_refused():
    op = {"params": [["result", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    assert refusal(post(lit(1)), op=op) == "ill-typed"


def test_a_fault_expression_is_type_checked_against_the_op():
    assert refusal(post(["add", var("result"), ["lit", True]])) == "ill-typed"
    assert refusal(post(var("nope"))) == "ill-typed"
