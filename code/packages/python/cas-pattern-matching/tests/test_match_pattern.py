"""Pattern — named bindings."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRSymbol

from cas_pattern_matching import Blank, Pattern, match


def test_pattern_captures_value() -> None:
    p = Pattern("x", Blank())
    bindings = match(p, IRInteger(5))
    assert bindings is not None
    assert bindings["x"] == IRInteger(5)


def test_pattern_with_compound_target() -> None:
    p = Pattern("expr", Blank())
    target = IRApply(MUL, (IRInteger(2), IRSymbol("y")))
    bindings = match(p, target)
    assert bindings is not None
    assert bindings["expr"] == target


def test_repeated_name_same_value_matches() -> None:
    """``Add(x_, x_)`` matches ``Add(a, a)`` but not ``Add(a, b)``."""
    p = IRApply(ADD, (Pattern("x", Blank()), Pattern("x", Blank())))
    target = IRApply(ADD, (IRSymbol("a"), IRSymbol("a")))
    assert match(p, target) is not None


def test_repeated_name_different_values_fails() -> None:
    p = IRApply(ADD, (Pattern("x", Blank()), Pattern("x", Blank())))
    target = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
    assert match(p, target) is None


def test_pattern_with_head_constraint() -> None:
    """``Pattern("n", Blank("Integer"))`` only binds integer literals."""
    p = Pattern("n", Blank("Integer"))
    assert match(p, IRInteger(7)) is not None
    assert match(p, IRSymbol("x")) is None


def test_two_patterns_in_one_expr() -> None:
    """``Add(x_, y_)`` against ``Add(2, 3)`` binds x=2, y=3."""
    p = IRApply(ADD, (Pattern("x", Blank()), Pattern("y", Blank())))
    target = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    bindings = match(p, target)
    assert bindings is not None
    assert bindings["x"] == IRInteger(2)
    assert bindings["y"] == IRInteger(3)
