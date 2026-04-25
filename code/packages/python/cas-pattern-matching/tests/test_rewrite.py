"""rewrite — apply rules bottom-up to fixed point."""

from __future__ import annotations

import pytest
from symbolic_ir import ADD, MUL, POW, IRApply, IRInteger, IRSymbol

from cas_pattern_matching import Blank, Pattern, RewriteCycleError, Rule, rewrite

X = Pattern("x", Blank())


# Identity rules.
ADD_ZERO = Rule(IRApply(ADD, (X, IRInteger(0))), X)
MUL_ONE = Rule(IRApply(MUL, (X, IRInteger(1))), X)
MUL_ZERO = Rule(IRApply(MUL, (X, IRInteger(0))), IRInteger(0))
POW_ZERO = Rule(IRApply(POW, (X, IRInteger(0))), IRInteger(1))


def test_identity_rule_fires_at_root() -> None:
    expr = IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
    assert rewrite(expr, [ADD_ZERO]) == IRSymbol("z")


def test_identity_rule_fires_in_subtree() -> None:
    """Bottom-up walk: ``Mul(2, Add(z, 0))`` -> ``Mul(2, z)``."""
    inner = IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
    expr = IRApply(MUL, (IRInteger(2), inner))
    expected = IRApply(MUL, (IRInteger(2), IRSymbol("z")))
    assert rewrite(expr, [ADD_ZERO]) == expected


def test_multiple_rules_all_fire() -> None:
    """Apply (z+0)*1 — both rules fire to give just `z`."""
    inner = IRApply(ADD, (IRSymbol("z"), IRInteger(0)))
    expr = IRApply(MUL, (inner, IRInteger(1)))
    assert rewrite(expr, [ADD_ZERO, MUL_ONE]) == IRSymbol("z")


def test_iterates_until_fixed_point() -> None:
    """((z+0)+0) needs two ADD_ZERO firings."""
    expr = IRApply(
        ADD,
        (IRApply(ADD, (IRSymbol("z"), IRInteger(0))), IRInteger(0)),
    )
    assert rewrite(expr, [ADD_ZERO]) == IRSymbol("z")


def test_no_rule_fires() -> None:
    expr = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
    # No identity opportunity.
    assert rewrite(expr, [ADD_ZERO]) == expr


def test_pow_zero_rule() -> None:
    expr = IRApply(POW, (IRSymbol("z"), IRInteger(0)))
    assert rewrite(expr, [POW_ZERO]) == IRInteger(1)


def test_cycle_detection() -> None:
    """A rule that grows the expression forever is detected."""
    # Rule: x_ -> Add(x_, 0). This expands forever because the rewritten
    # form still matches its own LHS.
    grow = Rule(X, IRApply(ADD, (X, IRInteger(0))))
    with pytest.raises(RewriteCycleError):
        rewrite(IRSymbol("z"), [grow], max_iterations=10)
