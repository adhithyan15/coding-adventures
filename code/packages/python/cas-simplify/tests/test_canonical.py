"""Canonical-form pass."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRSymbol

from cas_simplify import canonical


def test_flatten_add() -> None:
    """Add(a, Add(b, c)) → Add(a, b, c)."""
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    inner = IRApply(ADD, (b, c))
    expr = IRApply(ADD, (a, inner))
    out = canonical(expr)
    assert isinstance(out, IRApply)
    assert out.head == IRSymbol("Add")
    # After flatten + sort, args should be (a, b, c) by alphabetical key.
    assert out.args == (a, b, c)


def test_sort_args_alphabetical() -> None:
    """Add(c, a, b) → Add(a, b, c)."""
    a, b, c = IRSymbol("a"), IRSymbol("b"), IRSymbol("c")
    expr = IRApply(ADD, (c, a, b))
    out = canonical(expr)
    assert isinstance(out, IRApply)
    assert out.args == (a, b, c)


def test_integer_sorts_before_symbol() -> None:
    """Add(x, 2) → Add(2, x) — integers come first in canonical order."""
    expr = IRApply(ADD, (IRSymbol("x"), IRInteger(2)))
    out = canonical(expr)
    assert isinstance(out, IRApply)
    assert out.args == (IRInteger(2), IRSymbol("x"))


def test_singleton_add_drops_to_inner() -> None:
    """Add(x) → x."""
    assert canonical(IRApply(ADD, (IRSymbol("x"),))) == IRSymbol("x")


def test_singleton_mul_drops_to_inner() -> None:
    assert canonical(IRApply(MUL, (IRSymbol("x"),))) == IRSymbol("x")


def test_empty_add_is_zero() -> None:
    assert canonical(IRApply(ADD, ())) == IRInteger(0)


def test_empty_mul_is_one() -> None:
    assert canonical(IRApply(MUL, ())) == IRInteger(1)


def test_canonical_is_idempotent() -> None:
    """canonical(canonical(x)) == canonical(x)."""
    expr = IRApply(ADD, (IRSymbol("c"), IRSymbol("a"), IRSymbol("b")))
    once = canonical(expr)
    twice = canonical(once)
    assert once == twice


def test_non_commutative_head_is_not_sorted() -> None:
    """A non-commutative head (Sub) keeps its arg order."""
    from symbolic_ir import SUB

    expr = IRApply(SUB, (IRSymbol("b"), IRSymbol("a")))
    assert canonical(expr) == expr
