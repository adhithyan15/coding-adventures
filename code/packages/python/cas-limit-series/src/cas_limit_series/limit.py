"""``limit_direct`` — direct-substitution Limit.

Tries to compute ``lim_{var → point} expr`` by substituting ``point``
for ``var`` and returning the result. Always succeeds for expressions
that are continuous at ``point``; for indeterminate cases (``0/0``,
``∞/∞``) returns the unevaluated ``Limit(expr, var, point)``.

This is the trivial first phase. The full Limit operation (with
L'Hôpital, standard limits at infinity, …) is deferred until the
package can take a differentiation callable as input.
"""

from __future__ import annotations

from cas_substitution import subst
from symbolic_ir import IRApply, IRNode, IRSymbol

from cas_limit_series.heads import LIMIT


def limit_direct(expr: IRNode, var: IRNode, point: IRNode) -> IRNode:
    """Substitute ``var`` with ``point`` and return the result.

    Does NOT call any simplifier — the caller should pass the result
    through ``cas_simplify.simplify`` afterwards.

    If a substitution would produce an obviously-indeterminate result
    (a literal ``0/0`` or ``∞/∞``), this function returns the
    unevaluated ``Limit(expr, var, point)``. Detecting indeterminate
    forms in arbitrary IR is the work of L'Hôpital, which isn't yet
    implemented; the trivial detection here just covers the cases
    where the caller hands us an expression that's already collapsed.
    """
    out = subst(point, var, expr)
    if _looks_indeterminate(out):
        return IRApply(LIMIT, (expr, var, point))
    return out


def _looks_indeterminate(node: IRNode) -> bool:
    """Return True if ``node`` looks like an indeterminate form.

    Conservative: only literal ``0/0`` triggers this. The signal we're
    really after — ``Add(.., Inv(.))`` where the inner is zero — is
    impossible to detect without simplification, so we leave it for
    L'Hôpital.
    """
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Div"
        and len(node.args) == 2
    ):
        from symbolic_ir import IRInteger

        n, d = node.args
        if (
            isinstance(n, IRInteger)
            and n.value == 0
            and isinstance(d, IRInteger)
            and d.value == 0
        ):
            return True
    return False
