"""Simple structural substitution.

``subst(value, var, expr)`` walks ``expr`` and returns a new IR where
every occurrence of ``var`` is replaced by ``value``.

This is the MACSYMA convention — the variable to replace comes first,
then the substitute, then the expression. Maxima's surface form is
``subst(2, x, expr)`` meaning "replace x with 2 in expr"; the IR
mirror keeps the same argument order.
"""

from __future__ import annotations

from collections.abc import Iterable

from symbolic_ir import IRApply, IRNode


def subst(value: IRNode, var: IRNode, expr: IRNode) -> IRNode:
    """Replace every occurrence of ``var`` in ``expr`` with ``value``.

    The match is structural equality — equal nodes (same value, same
    type) get replaced. This is more general than just a symbol
    substitution: any sub-expression can act as the search target.
    """
    if expr == var:
        return value
    if isinstance(expr, IRApply):
        new_head = subst(value, var, expr.head)
        new_args = tuple(subst(value, var, a) for a in expr.args)
        return IRApply(new_head, new_args)
    return expr


def subst_many(
    rules: Iterable[tuple[IRNode, IRNode]],
    expr: IRNode,
) -> IRNode:
    """Apply a sequence of ``(var, value)`` substitutions in order.

    Each rule is applied to the result of the previous one. Two rules
    can interact: the second sees the first's result and may rewrite
    pieces of the substitution.
    """
    out = expr
    for var, value in rules:
        out = subst(value, var, out)
    return out
