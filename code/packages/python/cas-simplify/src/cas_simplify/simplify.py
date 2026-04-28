"""Top-level ``simplify`` entry point.

Algorithm — fixed-point loop::

    while True:
        prev = current
        current = canonical(current)
        current = numeric_fold(current)
        current = rewrite(current, IDENTITY_RULES)
        if current == prev:
            break
    return current

We bound the loop at ``max_iterations=50`` so a buggy custom rule
database can never lock the program in an infinite spin. In practice
2–4 passes is enough for the textbook examples.
"""

from __future__ import annotations

from cas_pattern_matching import RewriteCycleError, rewrite
from symbolic_ir import IRNode

from cas_simplify.canonical import canonical
from cas_simplify.numeric_fold import numeric_fold
from cas_simplify.rules import IDENTITY_RULES


def simplify(expr: IRNode, *, max_iterations: int = 50) -> IRNode:
    """Apply canonical → numeric fold → identity rules to fixed point.

    Returns the simplified IR. Raises :class:`RewriteCycleError` (from
    cas-pattern-matching) if the inner rule application cannot
    converge — that's an indicator the rule database has a problem.
    """
    current = expr
    for _ in range(max_iterations):
        prev = current
        current = canonical(current)
        current = numeric_fold(current)
        try:
            current = rewrite(current, IDENTITY_RULES, max_iterations=200)
        except RewriteCycleError:
            # If the rules themselves cycle there's nothing we can do
            # locally; surface the error to the caller.
            raise
        if current == prev:
            return current
    return current
