"""TrigSimplify: apply Pythagorean identities, sign rules, and special values.

Algorithm
---------
Fixed-point loop:
  1. Canonicalize (sort/flatten via ``cas_simplify.canonical``).
  2. Walk the tree looking for:
     a. Special values: ``Sin(%pi/6)`` → ``1/2``, etc.
     b. Pythagorean patterns: ``sin²(x)+cos²(x)`` → ``1``.
     c. Sign rules: ``sin(-x)`` → ``-sin(x)``, ``cos(-x)`` → ``cos(x)``.
  3. Numeric-fold constants.
  4. Repeat until stable.
"""

from __future__ import annotations

from cas_pattern_matching import rewrite
from cas_simplify import canonical
from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_trig.rules import TRIG_RULES
from cas_trig.special_values import lookup_special_value

_TRIG_HEADS = frozenset({"Sin", "Cos", "Tan", "Csc", "Sec", "Cot"})

# Maximum fixed-point iterations to avoid infinite loops
_MAX_ITER = 20


def trig_simplify(expr: IRNode) -> IRNode:
    """Apply trig simplification rules to ``expr`` until stable.

    Applies:
    1. Special-value lookup (``sin(π/6)`` → ``1/2``, etc.).
    2. Pythagorean and sign-rule rewrites.
    3. Canonical algebraic simplification.

    Returns the simplified IR node.
    """
    for _ in range(_MAX_ITER):
        prev = expr
        # Step 1: apply special-value lookups bottom-up
        expr = _special_value_walk(expr)
        # Step 2: apply pattern rules
        try:
            expr = rewrite(expr, TRIG_RULES)
        except Exception:
            pass  # RewriteCycleError or similar: skip
        # Step 3: algebraic canonical form
        expr = canonical(expr)
        if expr == prev:
            break
    return expr


# ---------------------------------------------------------------------------
# Internal: bottom-up special-value walk
# ---------------------------------------------------------------------------


def _special_value_walk(node: IRNode) -> IRNode:
    """Recursively replace trig special values in ``node``."""
    if not isinstance(node, IRApply):
        return node

    # Recurse into args first
    new_args = tuple(_special_value_walk(a) for a in node.args)
    if new_args != node.args:
        node = IRApply(node.head, new_args)

    # Check if this node is a trig function applied to a special value
    if isinstance(node.head, IRSymbol) and node.head.name in _TRIG_HEADS:
        if len(node.args) == 1:
            val = lookup_special_value(node.head.name, node.args[0])
            if val is not None:
                return val

    return node
