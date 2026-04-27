"""Apply rewrite rules to IR trees.

Two operations:

- :func:`apply_rule` — try one rule at the *root* of an expression.
  Returns the rewritten IR or None if the rule didn't match.
- :func:`rewrite` — recursively apply a list of rules until no rule
  fires anywhere in the tree (or ``max_iterations`` runs out).

Rules are applied bottom-up (post-order): we rewrite children first,
then try rules at the current node. This usually converges faster than
top-down for algebraic identity rules ("simplify the inside, then look
again at the outside").

Cycle detection: if a rule rewrites an expression to a structurally
larger form that then matches again, the loop would never terminate.
:func:`rewrite` bounds total iterations at ``max_iterations`` (default
100) and raises :class:`RewriteCycleError` if the bound is hit. In
practice well-behaved rules converge in 2–5 passes; the bound exists
to surface bugs in user-written rule sets.
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRNode

from cas_pattern_matching.bindings import Bindings
from cas_pattern_matching.matcher import match
from cas_pattern_matching.nodes import is_rule


class RewriteCycleError(RuntimeError):
    """Raised when :func:`rewrite` cannot reach a fixed point."""


# ---------------------------------------------------------------------------
# Single-rule application
# ---------------------------------------------------------------------------


def apply_rule(rule: IRApply, expr: IRNode) -> IRNode | None:
    """Try ``rule`` against ``expr``. Return the rewritten IR or None.

    The rule must be a ``Rule(lhs, rhs)`` or ``RuleDelayed(lhs, rhs)``
    apply. The rule's ``rhs`` is evaluated by structural substitution
    of the captured bindings.
    """
    if not is_rule(rule):
        raise ValueError(f"apply_rule expected Rule/RuleDelayed, got {rule!r}")
    lhs, rhs = rule.args
    bindings = match(lhs, expr)
    if bindings is None:
        return None
    return _substitute(rhs, bindings)


def _substitute(template: IRNode, bindings: Bindings) -> IRNode:
    """Replace every named-pattern reference in ``template`` with its binding.

    ``template`` is a rule's RHS — typically a normal IR tree, but any
    ``Pattern(name, _)`` references inside refer to bindings captured
    during matching. They expand to the captured value.
    """
    from cas_pattern_matching.nodes import is_pattern, pattern_name

    if is_pattern(template):
        assert isinstance(template, IRApply)
        name = pattern_name(template)
        if name in bindings:
            return bindings[name]
        # Unbound pattern in RHS — leave as-is. Slightly forgiving; a
        # stricter implementation would raise. Keep it forgiving so
        # users can write half-finished rules during exploration.
        return template

    if isinstance(template, IRApply):
        new_head = _substitute(template.head, bindings)
        new_args = tuple(_substitute(a, bindings) for a in template.args)
        return IRApply(new_head, new_args)

    return template


# ---------------------------------------------------------------------------
# Bottom-up rewrite to fixed point
# ---------------------------------------------------------------------------


def rewrite(
    expr: IRNode,
    rules: list[IRApply],
    *,
    max_iterations: int = 100,
) -> IRNode:
    """Apply ``rules`` to ``expr`` until no rule fires.

    Walks the tree bottom-up. On each node, after rewriting its children,
    tries every rule in order; if one fires, recurses on the result.
    Returns the fixed-point IR.

    Raises :class:`RewriteCycleError` if the iteration count exceeds
    ``max_iterations``.
    """
    counter = [0]

    def walk(node: IRNode) -> IRNode:
        # Rewrite children first.
        if isinstance(node, IRApply):
            new_head = walk(node.head)
            new_args = tuple(walk(a) for a in node.args)
            current: IRNode = (
                node
                if (new_head is node.head and new_args == node.args)
                else IRApply(new_head, new_args)
            )
        else:
            current = node

        # Then try rules at this position.
        while True:
            counter[0] += 1
            if counter[0] > max_iterations:
                raise RewriteCycleError(
                    f"rewrite did not converge within {max_iterations} iterations"
                )
            fired = False
            for rule in rules:
                replacement = apply_rule(rule, current)
                if replacement is not None and replacement != current:
                    # Re-walk the replacement to apply rules to its sub-parts.
                    current = walk(replacement)
                    fired = True
                    break
            if not fired:
                return current

    return walk(expr)
