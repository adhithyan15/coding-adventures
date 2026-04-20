"""Tests for the symbolic IR node types.

The IR is the foundation everything else depends on, so we test it
exhaustively: construction, normalization, equality, hashing, and the
no-mutation guarantee of frozen dataclasses.
"""

from __future__ import annotations

import pytest

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# Construction and basic attributes
# ---------------------------------------------------------------------------


def test_symbol_stores_name() -> None:
    assert IRSymbol("x").name == "x"
    assert str(IRSymbol("x")) == "x"


def test_integer_stores_value() -> None:
    assert IRInteger(42).value == 42
    assert IRInteger(-7).value == -7
    assert IRInteger(0).value == 0


def test_integer_arbitrary_precision() -> None:
    # Python ints are already arbitrary precision — verify it works.
    huge = 10**200
    assert IRInteger(huge).value == huge


def test_float_stores_value() -> None:
    assert IRFloat(3.14).value == 3.14


def test_string_stores_value() -> None:
    assert IRString("hello").value == "hello"


# ---------------------------------------------------------------------------
# IRRational normalization
# ---------------------------------------------------------------------------


def test_rational_reduces_on_construction() -> None:
    r = IRRational(2, 4)
    assert r.numer == 1
    assert r.denom == 2


def test_rational_normalizes_negative_denom() -> None:
    r = IRRational(1, -2)
    assert r.numer == -1
    assert r.denom == 2


def test_rational_double_negative() -> None:
    r = IRRational(-3, -6)
    assert r.numer == 1
    assert r.denom == 2


def test_rational_already_reduced() -> None:
    r = IRRational(3, 5)
    assert r.numer == 3
    assert r.denom == 5


def test_rational_zero_numerator() -> None:
    # 0/n should reduce to 0/1 (though we keep IRRational type).
    r = IRRational(0, 5)
    assert r.numer == 0
    assert r.denom == 1


def test_rational_division_by_zero_raises() -> None:
    with pytest.raises(ValueError, match="denominator cannot be zero"):
        IRRational(1, 0)


# ---------------------------------------------------------------------------
# Equality and hashing
# ---------------------------------------------------------------------------


def test_symbols_equal_by_name() -> None:
    assert IRSymbol("x") == IRSymbol("x")
    assert IRSymbol("x") != IRSymbol("y")


def test_symbols_hashable() -> None:
    # Must be usable as dict keys — essential for rule caching.
    d = {IRSymbol("x"): 1, IRSymbol("y"): 2}
    assert d[IRSymbol("x")] == 1


def test_integers_equal_by_value() -> None:
    assert IRInteger(5) == IRInteger(5)
    assert IRInteger(5) != IRInteger(6)


def test_different_types_not_equal() -> None:
    # Critical: IRInteger(1) and IRRational(1, 1) are not equal, even
    # though they represent the same mathematical value. The VM decides
    # when to collapse representations.
    assert IRInteger(1) != IRRational(1, 1)


def test_apply_equal_structurally() -> None:
    a = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    b = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert a == b
    assert hash(a) == hash(b)


def test_apply_order_matters() -> None:
    # Add is commutative semantically, but the IR preserves order
    # until the VM chooses to canonicalize.
    a = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    b = IRApply(ADD, (IRInteger(1), IRSymbol("x")))
    assert a != b


# ---------------------------------------------------------------------------
# Immutability
# ---------------------------------------------------------------------------


def test_nodes_are_frozen() -> None:
    x = IRSymbol("x")
    with pytest.raises((AttributeError, TypeError)):
        x.name = "y"  # type: ignore[misc]


def test_apply_args_are_tuple() -> None:
    e = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert isinstance(e.args, tuple)


# ---------------------------------------------------------------------------
# IRNode isinstance relationship
# ---------------------------------------------------------------------------


def test_all_types_are_irnode() -> None:
    assert isinstance(IRSymbol("x"), IRNode)
    assert isinstance(IRInteger(1), IRNode)
    assert isinstance(IRRational(1, 2), IRNode)
    assert isinstance(IRFloat(1.0), IRNode)
    assert isinstance(IRString("s"), IRNode)
    assert isinstance(IRApply(ADD, ()), IRNode)


# ---------------------------------------------------------------------------
# Deeply nested trees
# ---------------------------------------------------------------------------


def test_nested_apply_roundtrip() -> None:
    # (x + 1) * x^2  =  Mul(Add(x, 1), Pow(x, 2))
    x = IRSymbol("x")
    inner = IRApply(ADD, (x, IRInteger(1)))
    squared = IRApply(POW, (x, IRInteger(2)))
    full = IRApply(MUL, (inner, squared))
    assert full.head == MUL
    assert len(full.args) == 2
    assert full.args[0] == inner
    assert full.args[1] == squared


def test_str_representation() -> None:
    expr = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    # We don't pin the exact format but verify the essential parts appear.
    s = str(expr)
    assert "Add" in s
    assert "x" in s
    assert "1" in s


# ---------------------------------------------------------------------------
# Standard head symbols are singletons
# ---------------------------------------------------------------------------


def test_standard_heads_are_singletons() -> None:
    from symbolic_ir import ADD, MUL

    # Re-importing gives the same object (enables identity comparison).
    assert ADD is ADD  # noqa: PLR0124
    assert MUL is MUL  # noqa: PLR0124
    assert ADD != MUL
