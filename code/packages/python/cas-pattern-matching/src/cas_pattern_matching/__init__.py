"""Mathematica-style pattern matching for the symbolic IR.

Quick start::

    from cas_pattern_matching import (
        Blank, Pattern, Rule, match, apply_rule, rewrite,
    )
    from symbolic_ir import ADD, POW, IRApply, IRInteger, IRSymbol

    # Rule: x^0 -> 1
    rule = Rule(
        IRApply(POW, (Pattern("x", Blank()), IRInteger(0))),
        IRInteger(1),
    )

    rewrite(IRApply(POW, (IRSymbol("z"), IRInteger(0))), [rule])
    # IRInteger(1)
"""

from cas_pattern_matching.bindings import Bindings
from cas_pattern_matching.matcher import match
from cas_pattern_matching.nodes import (
    BLANK,
    PATTERN,
    REPLACE,
    REPLACE_ALL,
    REPLACE_REPEATED,
    RULE,
    RULE_DELAYED,
    Blank,
    Pattern,
    Rule,
    RuleDelayed,
    blank_head,
    is_blank,
    is_pattern,
    is_rule,
    pattern_inner,
    pattern_name,
)
from cas_pattern_matching.rewriter import RewriteCycleError, apply_rule, rewrite

__all__ = [
    "BLANK",
    "Bindings",
    "Blank",
    "PATTERN",
    "Pattern",
    "REPLACE",
    "REPLACE_ALL",
    "REPLACE_REPEATED",
    "RULE",
    "RULE_DELAYED",
    "RewriteCycleError",
    "Rule",
    "RuleDelayed",
    "apply_rule",
    "blank_head",
    "is_blank",
    "is_pattern",
    "is_rule",
    "match",
    "pattern_inner",
    "pattern_name",
    "rewrite",
]
