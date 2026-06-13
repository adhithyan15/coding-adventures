"""``orthopoly`` — loadable orthogonal-polynomial package (Track M1).

A MACSYMA session gains numeric/symbolic closed-form expansion of the
classical orthogonal polynomials by calling::

    load("orthopoly");

Before that call, ``legendre_p(3, x)`` parses to ``LegendreP(3, x)`` and
stays unevaluated — the IR symbol exists (it's named in the spec of the
Legendre ODE) but no evaluator is registered for it.  After ``load``,
the same expression collapses to its closed-form polynomial::

    legendre_p(3, x);   →   (5*x^3 - 3*x)/2

What's covered
==============

Seven heads, all the classical orthogonal polynomial families that
:mod:`symbolic_ir.nodes` declares as named ODE solutions:

==================  =====================================================
Head                Closed form for non-negative integer ``n``
==================  =====================================================
``LegendreP(n, x)`` Bonnet recursion: ``P_0=1``, ``P_1=x``,
                    ``(n+1)P_{n+1} = (2n+1)x P_n − n P_{n−1}``.
``ChebyshevT(n, x)`` ``T_0=1``, ``T_1=x``,
                    ``T_{n+1} = 2x T_n − T_{n−1}``.
``ChebyshevU(n, x)`` ``U_0=1``, ``U_1=2x``,
                    ``U_{n+1} = 2x U_n − U_{n−1}``.
``HermiteH(n, x)``  Physicists' Hermite:
                    ``H_0=1``, ``H_1=2x``,
                    ``H_{n+1} = 2x H_n − 2n H_{n−1}``.
``LegendreQ(n, x)`` Held unevaluated when the package is loaded — the
                    second-kind functions involve logs and don't reduce
                    to polynomials.  Listed here so the symbol round-trips.
``BesselJ(n, x)``   Same: no closed-form polynomial reduction in general.
``BesselY(n, x)``   Same.
==================  =====================================================

For the second-kind / Bessel heads we deliberately install a *passthrough*
handler.  Once the user has ``load("orthopoly")``ed, they expect the
symbol to be a "known" name they can pattern-match against, even when no
closed form exists.  Returning the expression unevaluated is the right
behaviour and matches Maxima's ``orthopoly`` package contract.

Non-numeric ``n``
=================

If ``n`` isn't an :class:`~symbolic_ir.IRInteger` ≥ 0 we return the
expression unevaluated.  ``legendre_p(n, x)`` with a free symbolic ``n``
is fine; ``legendre_p(2.5, x)`` is not defined under our reduction rules
and we leave the call as-is rather than guess.

Loader contract
===============

The only public entry point is :func:`register_handlers`.  It takes a
:class:`~macsyma_runtime.backend.MacsymaBackend` and mutates its handler
table in place, adding the seven heads above.  The function is
*idempotent* — calling it twice is the same as calling it once, because
the second call overwrites the existing handler keys with the same
functions.  That property is what makes ``load("orthopoly")`` safe to
call repeatedly inside scripts.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING

from symbolic_ir import (
    DIV,
    MUL,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
)
from symbolic_vm.backend import Handler

if TYPE_CHECKING:
    from symbolic_vm import VM

    from macsyma_runtime.backend import MacsymaBackend


# ---------------------------------------------------------------------------
# Closed-form recurrences
# ---------------------------------------------------------------------------
#
# Each helper builds the IR tree symbolically; the VM's automatic
# simplifier (called via ``vm.eval`` in the handler) then folds the
# polynomial to a canonical form.  We deliberately work entirely in IR
# rather than evaluating at the host's float arithmetic — the user can
# pass a free symbol ``x`` and still get a polynomial back.


def _legendre_p(n: int, x: IRNode, vm: VM) -> IRNode:
    """Bonnet recursion for ``LegendreP(n, x)``.

    Stable for arbitrary ``n ≥ 0`` because every step is one multiply,
    one subtract, one rational divide.  Each intermediate is run through
    ``vm.eval`` so the polynomial stays in the backend's canonical form
    rather than ballooning into deeply-nested unsimplified IR.
    """
    if n == 0:
        return IRInteger(1)
    if n == 1:
        return x
    p_prev: IRNode = IRInteger(1)
    p_curr: IRNode = x
    for k in range(1, n):
        # (k+1) P_{k+1} = (2k+1) x P_k − k P_{k−1}
        two_k_plus_one = IRInteger(2 * k + 1)
        k_node = IRInteger(k)
        k_plus_one = IRInteger(k + 1)
        new = IRApply(
            DIV,
            (
                IRApply(
                    SUB,
                    (
                        IRApply(MUL, (two_k_plus_one, IRApply(MUL, (x, p_curr)))),
                        IRApply(MUL, (k_node, p_prev)),
                    ),
                ),
                k_plus_one,
            ),
        )
        p_prev = p_curr
        p_curr = vm.eval(new)
    return p_curr


def _chebyshev_t(n: int, x: IRNode, vm: VM) -> IRNode:
    """Chebyshev T recursion ``T_{n+1} = 2x T_n − T_{n−1}``."""
    if n == 0:
        return IRInteger(1)
    if n == 1:
        return x
    t_prev: IRNode = IRInteger(1)
    t_curr: IRNode = x
    two_x = IRApply(MUL, (IRInteger(2), x))
    for _ in range(1, n):
        new = IRApply(SUB, (IRApply(MUL, (two_x, t_curr)), t_prev))
        t_prev = t_curr
        t_curr = vm.eval(new)
    return t_curr


def _chebyshev_u(n: int, x: IRNode, vm: VM) -> IRNode:
    """Chebyshev U recursion ``U_{n+1} = 2x U_n − U_{n−1}``.

    Note the different seed from T: ``U_0 = 1``, ``U_1 = 2x``.
    """
    if n == 0:
        return IRInteger(1)
    two_x = vm.eval(IRApply(MUL, (IRInteger(2), x)))
    if n == 1:
        return two_x
    u_prev: IRNode = IRInteger(1)
    u_curr: IRNode = two_x
    two_x_factor = IRApply(MUL, (IRInteger(2), x))
    for _ in range(1, n):
        new = IRApply(SUB, (IRApply(MUL, (two_x_factor, u_curr)), u_prev))
        u_prev = u_curr
        u_curr = vm.eval(new)
    return u_curr


def _hermite_h(n: int, x: IRNode, vm: VM) -> IRNode:
    """Physicists' Hermite ``H_{n+1} = 2x H_n − 2n H_{n−1}``.

    Seed: ``H_0 = 1``, ``H_1 = 2x``.  This is the convention used by
    the IR head's docstring and matches Maxima's ``hermite``.
    """
    if n == 0:
        return IRInteger(1)
    two_x = vm.eval(IRApply(MUL, (IRInteger(2), x)))
    if n == 1:
        return two_x
    h_prev: IRNode = IRInteger(1)
    h_curr: IRNode = two_x
    two_x_factor = IRApply(MUL, (IRInteger(2), x))
    for k in range(1, n):
        two_k = IRInteger(2 * k)
        new = IRApply(
            SUB,
            (
                IRApply(MUL, (two_x_factor, h_curr)),
                IRApply(MUL, (two_k, h_prev)),
            ),
        )
        h_prev = h_curr
        h_curr = vm.eval(new)
    return h_curr


# ---------------------------------------------------------------------------
# Handler shells — extract args, route to recurrence, fall through on mismatch
# ---------------------------------------------------------------------------


def _check_nx(expr: IRApply) -> tuple[int, IRNode] | None:
    """Return ``(n, x)`` if ``expr`` is a 2-arg call with non-negative integer n.

    Any other shape — wrong arity, non-integer ``n``, negative ``n`` —
    returns ``None``, which the handlers translate into "leave the
    expression unevaluated".  This preserves the "no surprise" rule: if
    the package can't reduce, the symbol persists for the user to
    inspect.
    """
    if len(expr.args) != 2:
        return None
    n_node, x_node = expr.args
    if not isinstance(n_node, IRInteger):
        return None
    if n_node.value < 0:
        return None
    return n_node.value, x_node


def _make_recurrence_handler(
    fn: Callable[[int, IRNode, VM], IRNode],
) -> Handler:
    """Adapt a closed-form recurrence ``fn(n, x, vm)`` into a VM handler."""

    def handler(vm: VM, expr: IRApply) -> IRNode:
        parsed = _check_nx(expr)
        if parsed is None:
            return expr
        n, x = parsed
        return vm.eval(fn(n, x, vm))

    return handler


def _passthrough_handler(_vm: VM, expr: IRApply) -> IRNode:
    """No closed form — return the expression unevaluated.

    Used for ``LegendreQ``, ``BesselJ``, ``BesselY``: the symbol is
    known after ``load("orthopoly")`` but the runtime has no
    polynomial reduction to apply.  Returning ``expr`` preserves
    structure for downstream rewrites (Taylor, integrate, …).
    """
    return expr


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def register_handlers(backend: MacsymaBackend) -> None:
    """Install orthogonal-polynomial handlers on ``backend``.

    Idempotent: re-registering overwrites the same keys with the same
    functions.  No internal state is changed beyond the handler table.
    """
    handlers: dict[str, Handler] = {
        "LegendreP": _make_recurrence_handler(_legendre_p),
        "ChebyshevT": _make_recurrence_handler(_chebyshev_t),
        "ChebyshevU": _make_recurrence_handler(_chebyshev_u),
        "HermiteH": _make_recurrence_handler(_hermite_h),
        # Symbols that the package "knows" but doesn't reduce — see
        # module docstring for the rationale.
        "LegendreQ": _passthrough_handler,
        "BesselJ": _passthrough_handler,
        "BesselY": _passthrough_handler,
    }
    # ``_handlers`` is the canonical attribute used by SymbolicBackend.
    # ``handlers()`` returns a mapping view onto the same dict.
    backend._handlers.update(handlers)


# Re-export the unused-but-public IR constants so static analysers
# don't flag them.  They're imported at module load to validate that
# the symbolic-ir package exposes the heads our recurrences depend on.
__all__ = ["register_handlers"]
