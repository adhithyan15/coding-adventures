"""Follow-ups folded into S10a (VER04r section 16.9, "follow-ups from the S3e review").

(a) ``lower_op_admit`` refuses an op ``admit`` that calls a subject op **even when no claim or
    example calls that op**, as the generator's front end does with ``W0301``: the refusal is the
    one the oracle already gives when such an op *is* called.
(b) The compilation of a callee ``admit`` is memoised per ``(op, widths)``: no output changes.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import (
    admit_check_cost,
    admit_fn,
    compile_admit_check,
)
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

GEN = {"name": "test", "version": "0"}
FORBIDDEN = "a subject call is forbidden here"


def lower(text: str):
    return lower_document(parse_source(text))


def refused(text: str) -> OracleError:
    with pytest.raises(OracleError) as err:
        lower(text)
    assert str(err.value) == f"ir: {FORBIDDEN}", err.value
    return err.value


# -- (a) fail-closed parity with the generator's W0301 -------------------------------------------


def test_an_unused_op_whose_admit_calls_a_subject_op_is_refused():
    refused("warrant a v1\n  op f(a: U8) -> U8\n  op g(a: U8 | f(a) == a) -> U8\n")


def test_the_same_refusal_when_a_claim_or_an_example_calls_it():
    refused(
        "warrant a v1\n  op f(a: U8) -> U8\n  op g(a: U8 | f(a) == a) -> U8\n"
        "  claim c:\n    forall a: U8. g(a) == a\n"
    )
    refused(
        "warrant a v1\n  op f(a: U8) -> U8\n  op g(a: U8 | f(a) == a) -> U8\n"
        "  example e: g(1u8) == 1u8\n"
    )


def test_the_refusal_is_the_one_a_call_gives_at_plan_time():
    # before S10a the refusal came from the typed compiler when a claim reached the op
    from coding_adventures_warrant_oracle.ir_eval import Program

    with pytest.raises(OracleError) as err:
        Program({}, {"f": None}).compile(["subject_op", "f", []], {}, allow_subject=False)
    assert str(err.value) == f"ir: {FORBIDDEN}"


@pytest.mark.parametrize(
    "refinement",
    [
        "f(a) == a",
        "a < 10u8 and f(a) == a",  # inside a boolean operator
        "if a < 10u8 then f(a) == a else true",  # inside an if
        "k(f(a)) == a",  # nested in the argument of another call
        "f(a) == a and match a with x -> true",  # next to a construct of a higher level
        "(f(a), a) == (a, a)",  # next to a tuple
        "unwrap_or(div(f(a), 2u8), 0u8) == a",  # inside a built-in
    ],
)
def test_the_refusal_finds_a_subject_call_wherever_it_sits(refinement):
    refused(
        "warrant a v1\n  op f(a: U8) -> U8\n  op k(a: U8) -> U8\n"
        f"  op g(a: U8 | {refinement}) -> U8\n"
    )


def test_an_op_may_call_a_model_in_its_admit_and_ops_without_a_call_are_unaffected():
    spec = lower(
        "warrant a v1\n  model small(a: U8) -> Bool = a < 10u8\n  op f(a: U8) -> U8\n"
        "  op g(a: U8 | small(a)) -> U8\n  op h(a: U8 | a < 4u8 and small(a)) -> U8\n"
    )
    assert spec.ops["g"].admit == ["model_call", "small", [["var", "a"]]]
    assert spec.ops["f"].admit is None and spec.ops["h"].admit[0] == "and"


def test_an_op_named_like_a_parameter_is_not_a_call():
    # ``f`` here is a parameter name used as a value, not a call of the op ``f``
    spec = lower("warrant a v1\n  op f(a: U8) -> U8\n  op g(f: U8 | f < 3u8) -> U8\n")
    assert spec.ops["g"].admit == ["lt", ["var", "f"], ["lit", {"type": "U8", "value": "3"}]]


# -- (b) memoised callee admit compilation --------------------------------------------------------

DOC = (
    "warrant m v1\n  model small(a: U8) -> Bool = a < 10u8\n"
    "  op g(a: U8 | small(a) and a < 9u8, b: U8) -> U8\n"
    "  op h<N: Nat>(b: Bytes<N> | len(b) < 100) -> Bool\n"
    "  claim many:\n"
    "    forall a: U8 | a < 3u8, b: U8. g(a, g(a, b)) == g(a, g(a, b))\n"
    "  claim widths:\n"
    "    forall x: Bytes<2>, y: Bytes<3>. (h<2>(x) == h<3>(y)) == h<2>(x)\n"
)


class Counter:
    """Counts the compilations of one op's ``admit`` (by identity of its IR)."""

    def __init__(self, prog, ir):
        self.n = 0
        self._orig = prog.compile
        self._ir = ir
        prog.compile = self

    def __call__(self, ir, tenv, allow_subject=True):
        if ir is self._ir:
            self.n += 1
        return self._orig(ir, tenv, allow_subject)


def test_a_callee_admit_is_compiled_once_per_op_and_widths_not_once_per_call_site():
    spec = lower(DOC)
    prog = build_program(spec)
    g_count = Counter(prog, spec.ops["g"].admit)
    body = spec.suites[0].body
    tenv = dict(spec.suites[0].binders)
    # four call sites of g in the body, each asked for twice (the check and its cost)
    compile_admit_check(prog, body, tenv)
    admit_check_cost(prog, body, tenv)
    assert g_count.n == 1
    # the closure is shared too, and the function entry point agrees
    assert admit_fn(prog, spec.ops["g"], None) is admit_fn(prog, spec.ops["g"], None)
    assert g_count.n == 1


def test_distinct_static_widths_compile_separately_equal_widths_share():
    spec = lower(DOC)
    prog = build_program(spec)
    h_count = Counter(prog, spec.ops["h"].admit)
    body = spec.suites[1].body
    tenv = dict(spec.suites[1].binders)
    compile_admit_check(prog, body, tenv)
    admit_check_cost(prog, body, tenv)
    assert h_count.n == 2  # widths (2,) and (3,); the third call site reuses (2,)
    assert admit_fn(prog, spec.ops["h"], [2]) is admit_fn(prog, spec.ops["h"], [2])
    assert admit_fn(prog, spec.ops["h"], [2]) is not admit_fn(prog, spec.ops["h"], [3])
    assert h_count.n == 2
    assert admit_fn(prog, spec.ops["h"], None) is not None  # an absent width list is its own key


def test_an_op_without_an_admit_has_nothing_to_compile():
    spec = lower("warrant n v1\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == f(a)\n")
    prog = build_program(spec)
    assert admit_fn(prog, spec.ops["f"], None) is None
    assert admit_check_cost(prog, spec.suites[0].body, dict(spec.suites[0].binders)) == 0


def test_memoisation_changes_no_output():
    """The same document derives to the same bytes however often a call site asks."""
    first = derive(DOC.encode(), b"", spec_ref="s.warrant", generator=GEN)
    second = derive(DOC.encode(), b"", spec_ref="s.warrant", generator=GEN)
    assert first.chunk_files == second.chunk_files and first.tfv == second.tfv
    assert first.corpus == second.corpus
    row = {s["claim"]: s for s in first.corpus["suites"]}["many"]
    assert row["cases"] == 3 * 256 and row["expected_calls"] == 3 * 256 * 4
