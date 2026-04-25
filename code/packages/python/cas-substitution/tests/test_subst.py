"""subst — structural substitution."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, POW, SIN, IRApply, IRInteger, IRSymbol

from cas_substitution import subst, subst_many


def test_subst_at_root() -> None:
    """subst(2, x, x) -> 2."""
    assert subst(IRInteger(2), IRSymbol("x"), IRSymbol("x")) == IRInteger(2)


def test_subst_no_occurrence() -> None:
    """subst(2, x, y) -> y."""
    assert subst(IRInteger(2), IRSymbol("x"), IRSymbol("y")) == IRSymbol("y")


def test_subst_in_compound() -> None:
    """subst(2, x, x^2) -> 2^2 (un-simplified)."""
    expr = IRApply(POW, (IRSymbol("x"), IRInteger(2)))
    expected = IRApply(POW, (IRInteger(2), IRInteger(2)))
    assert subst(IRInteger(2), IRSymbol("x"), expr) == expected


def test_subst_multiple_occurrences() -> None:
    """subst(2, x, x*x) -> 2*2."""
    expr = IRApply(MUL, (IRSymbol("x"), IRSymbol("x")))
    expected = IRApply(MUL, (IRInteger(2), IRInteger(2)))
    assert subst(IRInteger(2), IRSymbol("x"), expr) == expected


def test_subst_with_compound_value() -> None:
    """subst(a+b, x, x*x) -> (a+b)*(a+b)."""
    a, b, x = IRSymbol("a"), IRSymbol("b"), IRSymbol("x")
    expr = IRApply(MUL, (x, x))
    value = IRApply(ADD, (a, b))
    expected = IRApply(MUL, (value, value))
    assert subst(value, x, expr) == expected


def test_subst_nested_expression() -> None:
    """subst(2, x, sin(x^2 + 1)) -> sin(2^2 + 1) (un-simplified)."""
    x = IRSymbol("x")
    inner = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    expr = IRApply(SIN, (inner,))
    expected_inner = IRApply(
        ADD, (IRApply(POW, (IRInteger(2), IRInteger(2))), IRInteger(1))
    )
    expected = IRApply(SIN, (expected_inner,))
    assert subst(IRInteger(2), x, expr) == expected


def test_subst_searches_for_compound_target() -> None:
    """subst(z, x+1, (x+1)*(x+1)) -> z*z."""
    target = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    expr = IRApply(MUL, (target, target))
    expected = IRApply(MUL, (IRSymbol("z"), IRSymbol("z")))
    assert subst(IRSymbol("z"), target, expr) == expected


def test_subst_many_sequential() -> None:
    """subst_many([(x, 2), (y, 3)], x + y) -> 2 + 3."""
    expr = IRApply(ADD, (IRSymbol("x"), IRSymbol("y")))
    expected = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    rules = [(IRSymbol("x"), IRInteger(2)), (IRSymbol("y"), IRInteger(3))]
    assert subst_many(rules, expr) == expected


def test_subst_many_order_matters() -> None:
    """First rule's result is visible to the second rule."""
    expr = IRSymbol("x")
    # x -> y, then y -> z. Result: z.
    rules = [(IRSymbol("x"), IRSymbol("y")), (IRSymbol("y"), IRSymbol("z"))]
    assert subst_many(rules, expr) == IRSymbol("z")
