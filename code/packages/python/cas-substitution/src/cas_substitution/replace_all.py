"""Pattern-aware ``replace_all`` — Mathematica's ``/.`` operator.

Walks an IR tree and applies a single rule (or a list of rules)
everywhere a match is found. Differs from :func:`subst` in two ways:

1. The search target is a *pattern* (which may include named
   :class:`Pattern` wildcards), not a literal expression.
2. ``replace_all`` is **single-pass** — once a node is rewritten, the
   replacement is not searched again. For repeated application use
   ``cas_pattern_matching.rewrite``.

The walker descends into the IR top-down: at each node we try the
rule; if it fires, we return the replacement and do NOT recurse into
it. If it doesn't fire, we descend into children and reconstruct the
node.
"""

from __future__ import annotations

from collections.abc import Iterable

from cas_pattern_matching import apply_rule
from symbolic_ir import IRApply, IRNode


def replace_all(expr: IRNode, rule: IRApply) -> IRNode:
    """Apply ``rule`` to every position in ``expr`` where it matches.

    Top-down walk: when the rule fires at a node, the replacement is
    returned without descending into it. If the rule does not match at
    the current node, recurse into the head and args.
    """
    replacement = apply_rule(rule, expr)
    if replacement is not None:
        return replacement
    if isinstance(expr, IRApply):
        new_head = replace_all(expr.head, rule)
        new_args = tuple(replace_all(a, rule) for a in expr.args)
        if new_head is expr.head and new_args == expr.args:
            return expr
        return IRApply(new_head, new_args)
    return expr


def replace_all_many(
    expr: IRNode,
    rules: Iterable[IRApply],
) -> IRNode:
    """Apply each rule once across ``expr``, sequentially.

    The order of rules matters — later rules see the output of earlier
    ones. For fixed-point application of all rules together, use
    ``cas_pattern_matching.rewrite``.
    """
    out = expr
    for rule in rules:
        out = replace_all(out, rule)
    return out
