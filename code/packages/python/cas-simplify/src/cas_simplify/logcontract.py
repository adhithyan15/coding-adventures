"""Log contraction and expansion.

:func:`logcontract` — combine log sums into a single log
---------------------------------------------------------

Rules applied bottom-up:

1. ``log(a) + log(b) [+ log(c) + …] → log(a · b · c · …)``

   Any ``Add`` node whose args include two or more ``Log(...)`` terms
   has those terms merged into ``Log(product)``.  Non-log terms are
   left in the ``Add`` unchanged.

2. ``n · log(a) → log(aⁿ)``  for integer / rational n

   A ``Mul`` node with exactly one ``Log(a)`` factor and a single
   numeric coefficient ``n`` becomes ``Log(a^n)``.  If the ``Mul``
   has other non-numeric, non-log factors the rule does not fire (to
   avoid breaking expressions like ``x · log(y)``).

3. ``log(a) − log(b) → log(a/b)``

   A ``Sub(Log(a), Log(b))`` becomes ``Log(Div(a, b))``.

:func:`logexpand` — distribute a log over products / powers
------------------------------------------------------------

Rules applied bottom-up:

1. ``log(a^n) → n · log(a)``  for integer / rational n

2. ``log(a · b · …) → log(a) + log(b) + …``

3. ``log(a / b) → log(a) − log(b)``

The optional ``ctx`` parameter is accepted for API uniformity but not
currently used by any rule — all expansions are unconditional.

Example::

    from cas_simplify.logcontract import logcontract, logexpand
    from symbolic_ir import *

    a, b = IRSymbol("a"), IRSymbol("b")
    x = IRSymbol("x")

    # Contract: log(a) + log(b) → log(a*b)
    expr = IRApply(ADD, (IRApply(LOG, (a,)), IRApply(LOG, (b,))))
    logcontract(expr)   # → Log(Mul(a, b))

    # Expand: log(x^3) → 3*log(x)
    logexpand(IRApply(LOG, (IRApply(POW, (x, IRInteger(3))),)))
    # → Mul(3, Log(x))
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    DIV,
    LOG,
    MUL,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
)

from cas_simplify.assumptions import AssumptionContext

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def logcontract(expr: IRNode) -> IRNode:
    """Combine log sums and products into a single log.

    Applies rules 1–3 bottom-up in a single pass.
    """
    if not isinstance(expr, IRApply):
        return expr
    new_args = tuple(logcontract(a) for a in expr.args)
    node: IRApply = (
        IRApply(expr.head, new_args) if new_args != expr.args else expr
    )
    return _contract_node(node)


def logexpand(
    expr: IRNode,
    ctx: AssumptionContext | None = None,  # noqa: ARG001  (reserved for future use)
) -> IRNode:
    """Expand a single log over products, quotients, and powers.

    Applies rules 1–3 bottom-up.  The ``ctx`` parameter is accepted for
    API uniformity with the rest of Phase 21 but is not yet used.
    """
    if not isinstance(expr, IRApply):
        return expr
    new_args = tuple(logexpand(a, ctx) for a in expr.args)
    node: IRApply = (
        IRApply(expr.head, new_args) if new_args != expr.args else expr
    )
    return _expand_node(node)


# ---------------------------------------------------------------------------
# logcontract — internal helpers
# ---------------------------------------------------------------------------


def _contract_node(expr: IRApply) -> IRNode:
    head = expr.head
    if head == ADD:
        return _contract_add(expr)
    if head == SUB:
        return _contract_sub(expr)
    if head == MUL:
        return _contract_mul(expr)
    return expr


def _contract_add(expr: IRApply) -> IRNode:
    """Fold all Log(...) terms in an Add into a single Log(product)."""
    log_args: list[IRNode] = []
    other: list[IRNode] = []

    for arg in expr.args:
        if _is_log(arg):
            log_args.append(_log_inner(arg))
        else:
            other.append(arg)

    if len(log_args) < 2:
        # Nothing to contract.
        return expr

    # Build Log(Mul(a, b, c, ...)) as a flat product — avoids nested Muls.
    product: IRNode = (
        IRApply(MUL, tuple(log_args))
        if len(log_args) > 2
        else IRApply(MUL, (log_args[0], log_args[1]))
    )
    merged = IRApply(LOG, (product,))

    if not other:
        return merged
    # Preserve remaining non-log terms.
    return IRApply(ADD, (*other, merged))


def _contract_sub(expr: IRApply) -> IRNode:
    """Log(a) − Log(b) → Log(a/b)."""
    if len(expr.args) != 2:
        return expr
    lhs, rhs = expr.args
    if _is_log(lhs) and _is_log(rhs):
        return IRApply(LOG, (IRApply(DIV, (_log_inner(lhs), _log_inner(rhs))),))
    return expr


def _contract_mul(expr: IRApply) -> IRNode:
    """n · Log(a) → Log(a^n) when n is numeric and there is exactly one Log.

    This rule only fires when the Mul contains a single Log factor plus
    exactly one numeric coefficient (integer or rational).  If there are
    non-numeric, non-log factors the rule does not fire.
    """
    log_indices: list[int] = []
    numeric_indices: list[int] = []
    other_indices: list[int] = []

    for i, arg in enumerate(expr.args):
        if _is_log(arg):
            log_indices.append(i)
        elif isinstance(arg, (IRInteger, IRRational)):
            numeric_indices.append(i)
        else:
            other_indices.append(i)

    # Rule fires only when there is exactly one log and no other-type args.
    if len(log_indices) != 1 or other_indices:
        return expr

    # Need at least one numeric coefficient to lift.
    if not numeric_indices:
        return expr

    log_node = expr.args[log_indices[0]]
    coeff_args = [expr.args[i] for i in numeric_indices]

    # Build the coefficient: product of numeric factors.
    coeff: IRNode = coeff_args[0]
    if len(coeff_args) > 1:
        coeff = IRApply(MUL, tuple(coeff_args))

    # n * log(a) → log(a^n)
    return IRApply(LOG, (IRApply(POW, (_log_inner(log_node), coeff)),))


# ---------------------------------------------------------------------------
# logexpand — internal helpers
# ---------------------------------------------------------------------------


def _expand_node(expr: IRApply) -> IRNode:
    if expr.head == LOG:
        return _expand_log(expr)
    return expr


def _expand_log(expr: IRApply) -> IRNode:
    """Expand a single Log(...) into a sum / scaled log."""
    if len(expr.args) != 1:
        return expr
    arg = expr.args[0]

    # Rule 1: log(a^n) → n * log(a) for integer / rational n.
    if (
        isinstance(arg, IRApply)
        and arg.head == POW
        and len(arg.args) == 2
    ):
        base, exp_node = arg.args
        if isinstance(exp_node, (IRInteger, IRRational)):
            return IRApply(MUL, (exp_node, IRApply(LOG, (base,))))

    # Rule 2: log(a * b * …) → log(a) + log(b) + …
    if isinstance(arg, IRApply) and arg.head == MUL and len(arg.args) >= 2:
        log_terms = [IRApply(LOG, (a,)) for a in arg.args]
        # Build left-recursive Add: (...((log(a) + log(b)) + log(c)) + ...)
        result: IRNode = log_terms[0]
        for t in log_terms[1:]:
            result = IRApply(ADD, (result, t))
        return result

    # Rule 3: log(a / b) → log(a) − log(b).
    if (
        isinstance(arg, IRApply)
        and arg.head == DIV
        and len(arg.args) == 2
    ):
        num, den = arg.args
        return IRApply(SUB, (IRApply(LOG, (num,)), IRApply(LOG, (den,))))

    return expr


# ---------------------------------------------------------------------------
# Tiny helpers
# ---------------------------------------------------------------------------


def _is_log(node: IRNode) -> bool:
    """True if node is a Log(...) application with exactly one argument."""
    return (
        isinstance(node, IRApply)
        and node.head == LOG
        and len(node.args) == 1
    )


def _log_inner(node: IRApply) -> IRNode:
    """Return the argument of a Log(x) node (assumes _is_log is True)."""
    return node.args[0]
