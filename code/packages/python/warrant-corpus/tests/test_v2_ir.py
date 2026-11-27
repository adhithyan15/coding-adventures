"""The ``ir.core`` evaluator of VER04r section 4, beyond what the kit pins.

The conformance kit fixes the vocabulary of outcomes; these tests are about the
*mechanism*: that call counts are measured and traced and agree, that an
overflow is classed by where it happened, that a hostile tree is refused rather
than crashing the runner, that the fast arithmetic path and the hook path are
the same rule, and that a ``Bytes`` value is bounded **before** it is allocated.
"""

from __future__ import annotations

import sys
import tracemalloc

import pytest

from coding_adventures_warrant_corpus.v2 import STANDARD, Program, Semantics
from coding_adventures_warrant_corpus.v2.arith import WindowExceeded, wrap
from coding_adventures_warrant_corpus.v2.errors import IrOverflow, KitError, Refusal
from coding_adventures_warrant_corpus.v2.ir import (
    MAX_IR_VALUE,
    Compiled,
    SubjectTable,
    evaluate,
    recursion_guard,
    tag_level,
    type_level,
    walk,
)
from coding_adventures_warrant_corpus.v2.kit import outcome_ir
from coding_adventures_warrant_corpus.v2.wtypes import Ty, fixed_range

U8 = {"type": "U8", "value": "{}"}


def u8(n):
    return ["lit", {"type": "U8", "value": str(n)}]


def nat(n):
    return ["lit", {"type": "Nat", "value": str(n)}]


def var(name):
    return ["var", name]


BUMP = {"bump": {"params": [["a", "U8"]], "requires": ["ir.core"], "ret": "U8"}}
INC = {"inc": {"params": [["a", "U8"]], "body": ["add", var("a"), u8(1)]}}


def case(ir, **fields):
    """A ``v2-ir`` case with the usual stand-in: ``bump`` is played by ``inc``."""
    base = {
        "ir": ir,
        "env": [
            ["a", {"type": "U8", "value": "5"}],
            ["t", {"type": "Bool", "value": True}],
        ],
        "levels": ["ir.core"],
        "types": {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}},
        "ops": BUMP,
        "models": {"inc": INC["inc"]},
        "model_order": ["inc"],
        "subjects": {"bump": "inc"},
    }
    return base | fields


def bump(arg="a"):
    return ["subject_op", "bump", [var(arg)]]


def is_six(expr):
    return ["eq", expr, u8(6)]


# -- call counting: measured and traced ----------------------------------------
@pytest.mark.parametrize(
    ("name", "ir", "calls"),
    [
        ("a call", bump(), 1),
        ("nested calls", ["subject_op", "bump", [bump()]], 2),
        ("two calls in one expression", ["add", bump(), bump()], 2),
        ("and: a pure false guard skips the call", ["and", ["lit", False], is_six(bump())], 0),
        ("and: a pure true guard runs it", ["and", ["lit", True], is_six(bump())], 1),
        ("or: a pure true guard skips the call", ["or", ["lit", True], is_six(bump())], 0),
        ("or: a pure false guard runs it", ["or", ["lit", False], is_six(bump())], 1),
        ("implies: a false antecedent skips the call", ["implies", ["lit", False], is_six(bump())], 0),
        ("if: the taken branch counts", ["if", var("t"), is_six(bump()), ["lit", True]], 1),
        ("if: the untaken branch does not", ["if", ["not", var("t")], is_six(bump()), ["lit", True]], 0),
        ("if: equal calls on both branches are static", ["if", var("t"), bump(), bump()], 1),
        ("an impure guard guarding nothing", ["and", is_six(bump()), ["lit", True]], 1),
        ("an impure condition with pure branches", ["if", is_six(bump()), ["lit", True], ["lit", False]], 1),
        ("a call inside a model argument", ["model_call", "inc", [bump()]], 1),
        ("a dynamic count added to a constant", ["add", ["if", ["eq", var("a"), u8(5)], bump(), u8(0)], bump()], 2),
        ("a dynamic count inside a call", ["subject_op", "bump", [["if", ["eq", var("a"), u8(6)], bump(), u8(1)]]], 1),
    ],
)  # fmt: skip
def test_calls_are_measured_and_equal_to_the_pure_trace(name, ir, calls):
    outcome = outcome_ir(case(ir), STANDARD)
    assert outcome["calls"] == calls, name
    assert "traced_calls" not in outcome, name


def test_an_eager_evaluator_makes_the_two_counts_disagree():
    class Eager(Semantics):
        def and_(self, left, right):
            a, b = left(), right()
            return a and b

    outcome = outcome_ir(case(["and", ["lit", False], is_six(bump())]), Eager())
    assert (outcome["calls"], outcome["traced_calls"]) == (1, 0)


# -- overflow classes -----------------------------------------------------------------
BIG = (1 << 64) - 1
SQUARE = {"square": {"params": [["n", "Nat"]], "body": ["mul", var("n"), var("n")]}}
IDN = {"idn": {"params": [["n", "Nat"]], "requires": ["ir.core"], "ret": "Nat"}}
KEEP = {"keep": {"params": [["n", "Nat"]], "body": var("n")}}


def overflow_case(ir):
    return case(
        ir,
        env=[["n", {"type": "Nat", "value": str(BIG)}]],
        ops=IDN,
        models=SQUARE | KEEP,
        model_order=["square", "keep"],
        subjects={"idn": "keep"},
    )


def test_an_overflow_below_a_subject_call_is_a_case_failure():
    ir = ["mul", ["subject_op", "idn", [var("n")]], nat(BIG)]
    assert outcome_ir(overflow_case(ir), STANDARD) == {
        "error": "ir-overflow",
        "class": "case",
    }


def test_an_overflow_in_pure_arithmetic_is_a_corpus_error():
    ir = ["mul", var("n"), nat(BIG)]
    assert outcome_ir(overflow_case(ir), STANDARD) == {
        "error": "ir-overflow",
        "class": "corpus",
    }


def test_an_overflow_inside_a_model_fed_by_a_subject_call_is_a_case_failure():
    """The model body has no subject beneath it, but its input came from one."""
    fed = ["model_call", "square", [["subject_op", "idn", [var("n")]]]]
    assert outcome_ir(overflow_case(fed), STANDARD)["class"] == "case"
    pure = ["model_call", "square", [var("n")]]
    assert outcome_ir(overflow_case(pure), STANDARD)["class"] == "corpus"


def test_negating_an_exact_value_is_checked_against_the_window_too():
    class Tight(Semantics):
        def exact(self, value):
            raise WindowExceeded

    ir = ["neg", nat(1)]
    assert outcome_ir(case(ir), Tight()) == {"error": "ir-overflow", "class": "corpus"}


def test_a_bytes_value_is_bounded_before_it_is_allocated():
    """Section 14: ``MAX_IR_VALUE`` is checked *before* the allocation.

    Concatenating two 65,536-byte values would allocate 131,072 bytes; the peak
    allocation while the refusal happens must stay below one such operand.
    """
    big = "ab" * MAX_IR_VALUE
    program = Program()
    compiled = program.compile(["concat", var("x"), var("x")], {"x": Ty("Bytes")})
    value = bytes.fromhex(big)
    tracemalloc.start()
    tracemalloc.reset_peak()
    try:
        with pytest.raises(IrOverflow) as caught:
            evaluate(compiled, {"x": value})
        _current, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    assert caught.value.klass == "corpus"
    assert peak < MAX_IR_VALUE // 2, peak
    exact = program.compile(
        ["concat", var("x"), var("y")], {"x": Ty("Bytes"), "y": Ty("Bytes")}
    )
    assert len(evaluate(exact, {"x": value[:-1], "y": b"\x00"})) == MAX_IR_VALUE


# -- hostile trees ----------------------------------------------------------------------
def test_a_tree_far_deeper_than_python_can_recurse_is_refused_not_crashed():
    node = ["lit", True]
    for _ in range(200_000):
        node = ["not", node]
    with pytest.raises(Refusal) as caught:
        Program().compile(node, {})
    assert caught.value.code == "depth"


def test_a_tree_of_nested_arrays_is_not_a_tree_and_is_skipped_iteratively():
    """F13: only the operand positions of a tag are followed; an array is no node."""
    nodes = [["not", ["lit", True]]]
    for _ in range(50_000):
        nodes = [nodes]
    assert list(walk(nodes)) == []
    assert [n[0] for n, _d in walk(["not", ["lit", True]])] == ["not", "lit"]


def test_a_stray_item_in_an_argument_array_is_ill_typed_not_a_tag():
    """F13: ``["model_call", "m", ["x"]]`` is a malformed argument, not a node ``x``."""
    models = {"m": {"params": [["a", "U8"]], "body": ["lit", True]}}
    got = outcome_ir(
        case(["model_call", "m", ["x"]], models=models, model_order=["m"]), STANDARD
    )
    assert got == {"error": "ill-typed"}
    got = outcome_ir(
        case(["model_call", "m", [["x"]]], models=models, model_order=["m"]), STANDARD
    )
    assert got == {"error": "unknown-tag"}


def test_the_deepest_legal_nesting_evaluates():
    """64 models each called through a 3-deep expression: far past the default recursion limit."""
    models = {
        "m0": {"params": [["a", "Bool"]], "body": ["not", ["not", ["not", var("a")]]]}
    }
    order = ["m0"]
    for i in range(1, 63):
        body = var("a")
        for _ in range(60):
            body = ["not", body]
        models[f"m{i}"] = {
            "params": [["a", "Bool"]],
            "body": ["model_call", f"m{i - 1}", [body]],
        }
        order.append(f"m{i}")
    ir = ["model_call", "m62", [var("t")]]
    out = outcome_ir(
        case(
            ir,
            models=models,
            model_order=order,
            ops={},
            subjects={},
            max_ir_cost=65_536,
        ),
        STANDARD,
    )
    assert out["type"] == "Bool" and out["calls"] == 0


def test_the_recursion_guard_raises_and_restores_the_limit():
    before = sys.getrecursionlimit()
    with recursion_guard():
        assert sys.getrecursionlimit() >= recursion_guard.LIMIT
    assert sys.getrecursionlimit() == before


def test_a_stack_overflow_while_evaluating_is_the_depth_refusal():
    def boom(_env):
        raise RecursionError

    with pytest.raises(Refusal) as caught:
        evaluate(Compiled(Ty("Bool"), boom, 1), {})
    assert caught.value.code == "depth"


# -- static refusals: the shape of every node -----------------------------------------------
ILL_TYPED = [
    ["not"], ["lit"], ["lit", 5], ["lit", {"type": "U8"}], ["lit", {"type": "U8", "value": "300"}],
    ["lit", {"type": "Result<U8,E>", "value": {"ok": "1"}}], ["lit", {"type": "Foo", "value": "1"}],
    ["var"], ["var", 3], ["err"], ["err", 3], ["eq", ["lit", True]], ["if", ["lit", True]],
    ["model_call", "inc"], ["model_call", "inc", "x"], ["subject_op", 3, []],
    ["subject_op", "bump"], ["subject_op", "bump", [var("a")], "w"], ["subject_op", "bump", [var("a")], [True]],
    ["len", ["lit", True]], ["neg", ["lit", True]], ["not", u8(1)], ["is_err", u8(1)],
    ["add", u8(1), ["lit", {"type": "I8", "value": "1"}]], ["add", u8(1), nat(1)],
    ["add", ["lit", True], ["lit", True]], ["lt", ["lit", True], ["lit", False]],
    ["lt", u8(1), nat(1)], ["concat", u8(1), u8(1)], ["concat", ["lit", {"type": "Bytes", "value": ""}], u8(1)],
    ["take", ["lit", {"type": "Bytes", "value": ""}], u8(1)], ["take", u8(1), nat(1)],
    ["and", u8(1), ["lit", True]], ["if", u8(1), u8(1), u8(1)], ["add", 5, 6],
    ["model_call", "inc", [["lit", True]]], ["model_call", "inc", []],
    ["eq", u8(1), ["lit", True]], ["ok"], ["or", ["lit", True]],
]  # fmt: skip


@pytest.mark.parametrize(
    "ir", ILL_TYPED, ids=[str(i)[:50] for i in range(len(ILL_TYPED))]
)
def test_a_malformed_node_is_ill_typed(ir):
    assert outcome_ir(case(ir), STANDARD) == {"error": "ill-typed"}


def test_a_node_that_is_not_a_node_is_ill_typed():
    assert outcome_ir(case(5), STANDARD) == {"error": "ill-typed"}
    assert outcome_ir(case([]), STANDARD) == {"error": "ill-typed"}


def test_model_order_refusals_found_before_compilation():
    assert outcome_ir(case(u8(1), model_order=["ghost"]), STANDARD) == {
        "error": "model-order"
    }
    assert outcome_ir(case(["model_call", 3, []]), STANDARD) == {"error": "model-order"}
    assert outcome_ir(case(["model_call"]), STANDARD) == {"error": "model-order"}


def test_a_bad_declared_type_in_a_signature_is_ill_typed():
    bad_param = {"m": {"params": [["a", "Nope"]], "body": ["lit", True]}}
    call_m = ["model_call", "m", [var("a")]]
    assert outcome_ir(case(call_m, models=bad_param, model_order=["m"]), STANDARD) == {
        "error": "ill-typed"
    }
    bad_ret = {"op": {"params": [], "ret": "Nope", "requires": ["ir.core"]}}
    assert outcome_ir(case(["subject_op", "op", []], ops=bad_ret), STANDARD) == {
        "error": "ill-typed"
    }
    # R69 (i): validated in full even when nothing reaches them
    assert outcome_ir(case(["lit", True], ops=bad_ret), STANDARD) == {
        "error": "ill-typed"
    }


def test_a_type_of_a_level_the_runner_lacks_is_level_missing():
    tuple_op = {
        "t": {"params": [["p", "(U8,Bool)"]], "ret": "Bool", "requires": ["ir.tuple"]}
    }
    reach = ["subject_op", "t", [["lit", True]]]
    assert outcome_ir(case(reach, ops=tuple_op), STANDARD) == {"error": "level-missing"}
    # an op of a missing level nothing reaches is only classified, not refused
    assert outcome_ir(case(["lit", True], ops=tuple_op), STANDARD)["value"] is True
    assert "t" in Program(ops=tuple_op, levels=frozenset({"ir.core"})).not_live_ops
    # ... and at the levels this lane implements (S10e) the tuple type is live
    assert "t" not in Program(ops=tuple_op).not_live_ops
    assert outcome_ir(case(["lit", True], ops={}, levels=[]), STANDARD) == {
        "error": "level-missing"
    }
    assert type_level(Ty("Option", (Ty("U8"),)), {}) == {"ir.data"}
    assert type_level(Ty("List", (Ty("U8"),)), {}) == {"ir.list"}
    assert type_level(Ty("()", (Ty("U8"), Ty("Bool"))), {}) == {"ir.tuple"}
    assert type_level(Ty("P"), {"P": {"kind": "record"}}) == {"ir.data"}
    assert type_level(Ty("U8"), {}) == set()


def test_every_tag_of_every_defined_level_is_known():
    assert tag_level("add") == "ir.core" and tag_level("bit_xor") == "ir.fixed"
    assert tag_level("tuple") == "ir.tuple" and tag_level("match") == "ir.tuple"
    assert tag_level("some") == "ir.data" and tag_level("list") == "ir.list"
    assert tag_level("fold") == "ir.seq" and tag_level("wibble") is None


def test_max_ir_cost_must_be_an_integer_in_range():
    for bad in (None, "2000", 2000.0, True, 1999, 65537):
        if bad is True:  # bool is an int in Python: True == 1, below the floor
            pass
        with pytest.raises(Refusal) as caught:
            Program(max_ir_cost=bad)
        assert caught.value.code == "ir-cost"


# -- subject dispatch --------------------------------------------------------------------------
def test_a_stand_in_must_have_the_signature_of_the_op_it_plays():
    program = Program(
        ops=BUMP,
        models={"wide": {"params": [["a", "U16"]], "body": ["lit", True]}},
        model_order=["wide"],
    )
    program.compile(bump(), {"a": Ty("U8")})  # the call is what reaches the op
    with pytest.raises(KitError):
        SubjectTable(program, {"bump": "wide"})
    SubjectTable(
        program, {"unused-op": "wide", "bump-too": "missing"}
    )  # entries nobody uses


def test_a_subject_call_with_nothing_bound_is_a_kit_error():
    program = Program(ops=BUMP)
    compiled = program.compile(bump(), {"a": Ty("U8")})
    with pytest.raises(KitError):
        evaluate(compiled, {"a": 1})


# -- the fast path is the same rule as the hook path --------------------------------------------
class ForcedSlowPath(Semantics):
    """Overrides ``wrap`` with the standard rule, so the evaluator takes the hook path."""

    def wrap(self, value, ty):
        return wrap(value, ty)


@pytest.mark.parametrize("name", ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"])
@pytest.mark.parametrize("tag", ["add", "sub", "mul"])
def test_inline_and_hook_arithmetic_agree(name, tag):
    ty = Ty(name)
    low, high = fixed_range(ty)
    edge = {low, low + 1, -1, 0, 1, 2, 3, 127, 128, 255, high - 1, high}
    samples = sorted(v for v in edge if low <= v <= high)
    node = [tag, var("x"), var("y")]
    scope = {"x": ty, "y": ty}
    fast = Program(sem=STANDARD).compile(node, scope)
    slow = Program(sem=ForcedSlowPath()).compile(node, scope)
    for x in samples:
        for y in samples:
            env = {"x": x, "y": y}
            assert fast.fn(env) == slow.fn(env), (name, tag, x, y)
            assert low <= fast.fn(env) <= high


CALL_INC = ["model_call", "inc", [var("a")]]
CALL_BUMP = ["subject_op", "bump", [var("a")]]


@pytest.mark.parametrize(
    ("ir", "edit"),
    [
        (CALL_INC, {"models": {"inc": {"body": ["lit", True]}}}),  # no params
        (CALL_INC, {"models": {"inc": {"params": 5, "body": ["lit", True]}}}),
        (CALL_INC, {"models": {"inc": "not a row"}}),
        (CALL_BUMP, {"ops": {"bump": {"params": [["a"]], "ret": "U8"}}}),
        (["err", "X"], {"types": {"E": {"kind": "error"}}}),  # no cases declared
    ],
)
def test_a_row_of_the_wrong_shape_is_refused_not_a_crash(ir, edit):
    got = outcome_ir(case(ir, **edit), STANDARD)
    assert got == {"error": "ill-typed"}
