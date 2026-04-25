"""apply_rule — single-shot rule application at the root."""

from __future__ import annotations

import pytest
from symbolic_ir import ADD, MUL, POW, IRApply, IRInteger, IRSymbol

from cas_pattern_matching import Blank, Pattern, Rule, apply_rule


def test_rule_fires_and_substitutes() -> None:
    """``Pow(x_, 0) -> 1`` fires on ``Pow(z, 0)``."""
    rule = Rule(
        IRApply(POW, (Pattern("x", Blank()), IRInteger(0))),
        IRInteger(1),
    )
    assert apply_rule(rule, IRApply(POW, (IRSymbol("z"), IRInteger(0)))) == IRInteger(1)


def test_rule_returns_none_on_no_match() -> None:
    rule = Rule(
        IRApply(POW, (Pattern("x", Blank()), IRInteger(0))),
        IRInteger(1),
    )
    # No Pow at the root.
    assert apply_rule(rule, IRSymbol("y")) is None


def test_rule_substitutes_captured_pattern_in_rhs() -> None:
    """``Add(x_, x_) -> 2*x_`` substitutes the captured x into the RHS."""
    x = Pattern("x", Blank())
    rule = Rule(
        IRApply(ADD, (x, x)),
        IRApply(MUL, (IRInteger(2), x)),
    )
    target = IRApply(ADD, (IRSymbol("a"), IRSymbol("a")))
    expected = IRApply(MUL, (IRInteger(2), IRSymbol("a")))
    assert apply_rule(rule, target) == expected


def test_apply_rule_rejects_non_rule() -> None:
    with pytest.raises(ValueError):
        apply_rule(IRApply(ADD, (IRInteger(1),)), IRInteger(1))
