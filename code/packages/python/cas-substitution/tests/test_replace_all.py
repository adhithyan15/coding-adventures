"""replace_all — pattern-aware substitution everywhere."""

from __future__ import annotations

from cas_pattern_matching import Blank, Pattern, Rule
from symbolic_ir import ADD, MUL, POW, IRApply, IRInteger, IRSymbol

from cas_substitution import replace_all, replace_all_many


def test_replace_all_simple_rule() -> None:
    """``Pow(x_, 2) -> Mul(x_, x_)`` rewrites every Pow-of-2 in an expression."""
    rule = Rule(
        IRApply(POW, (Pattern("a", Blank()), IRInteger(2))),
        IRApply(MUL, (Pattern("a", Blank()), Pattern("a", Blank()))),
    )
    expr = IRApply(POW, (IRSymbol("y"), IRInteger(2)))
    expected = IRApply(MUL, (IRSymbol("y"), IRSymbol("y")))
    assert replace_all(expr, rule) == expected


def test_replace_all_in_subtree() -> None:
    """A rule fires deep inside an expression."""
    rule = Rule(
        IRApply(ADD, (Pattern("x", Blank()), IRInteger(0))),
        Pattern("x", Blank()),
    )
    inner = IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
    expr = IRApply(MUL, (IRInteger(2), inner))
    expected = IRApply(MUL, (IRInteger(2), IRSymbol("z")))
    assert replace_all(expr, rule) == expected


def test_replace_all_no_match() -> None:
    """If the rule never fires, expr is returned unchanged."""
    rule = Rule(
        IRApply(POW, (Pattern("a", Blank()), IRInteger(0))),
        IRInteger(1),
    )
    expr = IRApply(ADD, (IRSymbol("x"), IRSymbol("y")))
    assert replace_all(expr, rule) == expr


def test_replace_all_many() -> None:
    """Two rules in sequence."""
    rule1 = Rule(
        IRApply(ADD, (Pattern("x", Blank()), IRInteger(0))),
        Pattern("x", Blank()),
    )
    rule2 = Rule(
        IRApply(MUL, (Pattern("x", Blank()), IRInteger(1))),
        Pattern("x", Blank()),
    )
    inner = IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
    expr = IRApply(MUL, (inner, IRInteger(1)))
    assert replace_all_many(expr, [rule1, rule2]) == IRSymbol("z")


def test_replace_all_does_not_recurse_into_replacement() -> None:
    """Single-pass behavior: a replacement that itself matches is NOT re-rewritten.

    Use cas_pattern_matching.rewrite() for fixed-point semantics.
    """
    # x -> Add(x, 0). The replacement still matches an Add-of-zero rule
    # but replace_all is single-pass, so we get one substitution and stop.
    grow = Rule(
        Pattern("x", Blank()),
        IRApply(ADD, (Pattern("x", Blank()), IRInteger(0))),
    )
    out = replace_all(IRSymbol("z"), grow)
    # If we'd recursed, we'd loop forever; instead we get a single rewrite.
    assert out == IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
