"""Symbolic differentiation — the ``D`` handler.

Differentiation is just a tree walk driven by the calculus rules every
student learns: the sum rule, product rule, quotient rule, chain rule,
and power rule. None of it is numerical — we build new IR and let the
VM simplify the result through the normal arithmetic handlers. That's
how ``D(x^2, x)`` turns into ``2*x`` even though this file never
computes the constant ``2`` itself.

Because we emit binary applies that then re-enter ``vm.eval``, the
algebraic identities in :mod:`symbolic_vm.handlers` (``0 + x → x``,
``1 * x → x``, ``x^0 → 1``) clean up the intermediate clutter that a
naive product/chain rule would otherwise leave behind.

Only installed on :class:`~symbolic_vm.backends.SymbolicBackend`. In
strict mode ``D`` falls through to :meth:`Backend.on_unknown_head`,
which raises.
"""

from __future__ import annotations

from symbolic_ir import (
    ACOSH,
    ADD,
    ASINH,
    ATANH,
    COS,
    COSH,
    COTH,
    CSCH,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    SECH,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    D,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backend import Handler

ZERO = IRInteger(0)
ONE = IRInteger(1)


def differentiate() -> Handler:
    """Return the ``D`` handler for the symbolic backend."""

    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 2:
            raise TypeError(f"D expects 2 arguments, got {len(expr.args)}")
        f, x = expr.args
        if not isinstance(x, IRSymbol):
            # Differentiation with respect to something other than a
            # plain symbol is not supported. Leave the expression.
            return expr
        return vm.eval(_diff(f, x))

    return handler


def _diff(f: IRNode, x: IRSymbol) -> IRNode:
    """Return ``d f / d x`` as unevaluated IR.

    The caller is expected to feed the result back through ``vm.eval``
    so arithmetic identities apply.
    """
    # df/dx = 0 when f doesn't depend on x. This catches all constants
    # (integers, floats, rationals, strings) and any free variables
    # other than x.
    if not _depends_on(f, x):
        return ZERO

    # df/dx = 1 when f IS x.
    if f == x:
        return ONE

    # Everything else must be an IRApply (only atoms could equal x or
    # be independent, and we've handled both).
    if not isinstance(f, IRApply):
        return IRApply(D, (f, x))

    head = f.head

    # -- Sum and difference --------------------------------------------
    if head == ADD:
        return IRApply(ADD, tuple(_diff(a, x) for a in f.args))
    if head == SUB:
        a, b = f.args
        return IRApply(SUB, (_diff(a, x), _diff(b, x)))

    # -- Negation and inverse ------------------------------------------
    if head == NEG:
        (a,) = f.args
        return IRApply(NEG, (_diff(a, x),))

    # -- Product rule --------------------------------------------------
    if head == MUL:
        a, b = f.args
        # d(ab)/dx = a' b + a b'
        return IRApply(
            ADD,
            (
                IRApply(MUL, (_diff(a, x), b)),
                IRApply(MUL, (a, _diff(b, x))),
            ),
        )

    # -- Quotient rule -------------------------------------------------
    if head == DIV:
        a, b = f.args
        # d(a/b)/dx = (a' b - a b') / b^2
        return IRApply(
            DIV,
            (
                IRApply(
                    SUB,
                    (
                        IRApply(MUL, (_diff(a, x), b)),
                        IRApply(MUL, (a, _diff(b, x))),
                    ),
                ),
                IRApply(POW, (b, IRInteger(2))),
            ),
        )

    # -- Power rule (handles general case via chain rule) --------------
    if head == POW:
        base, exponent = f.args
        base_depends = _depends_on(base, x)
        exp_depends = _depends_on(exponent, x)
        if not exp_depends:
            # d(f^n)/dx = n * f^(n-1) * f'
            return IRApply(
                MUL,
                (
                    IRApply(
                        MUL,
                        (
                            exponent,
                            IRApply(
                                POW,
                                (
                                    base,
                                    IRApply(SUB, (exponent, ONE)),
                                ),
                            ),
                        ),
                    ),
                    _diff(base, x),
                ),
            )
        if not base_depends:
            # d(c^g)/dx = c^g * ln(c) * g'
            return IRApply(
                MUL,
                (
                    IRApply(
                        MUL,
                        (
                            f,  # c^g
                            IRApply(LOG, (base,)),
                        ),
                    ),
                    _diff(exponent, x),
                ),
            )
        # General f(x)^g(x) — use f^g = exp(g*ln(f)) and chain rule.
        return _diff(
            IRApply(EXP, (IRApply(MUL, (exponent, IRApply(LOG, (base,)))),)),
            x,
        )

    # -- Elementary functions (chain rule) -----------------------------
    if head == SIN:
        (inner,) = f.args
        return IRApply(MUL, (IRApply(COS, (inner,)), _diff(inner, x)))
    if head == COS:
        (inner,) = f.args
        return IRApply(
            MUL,
            (IRApply(NEG, (IRApply(SIN, (inner,)),)), _diff(inner, x)),
        )
    if head == TAN:
        (inner,) = f.args
        # d/dx tan(u) = sec²(u) · u' = u' / cos²(u)
        return IRApply(
            DIV,
            (_diff(inner, x), IRApply(POW, (IRApply(COS, (inner,)), IRInteger(2)))),
        )
    if head == EXP:
        (inner,) = f.args
        return IRApply(MUL, (IRApply(EXP, (inner,)), _diff(inner, x)))
    if head == LOG:
        (inner,) = f.args
        # d/dx ln(u) = u'/u
        return IRApply(DIV, (_diff(inner, x), inner))
    if head == SQRT:
        (inner,) = f.args
        # d/dx sqrt(u) = u' / (2 sqrt(u))
        return IRApply(
            DIV,
            (
                _diff(inner, x),
                IRApply(MUL, (IRInteger(2), IRApply(SQRT, (inner,)))),
            ),
        )

    # -- Hyperbolic functions (chain rule) --------------------------------
    # d/dx sinh(u) = cosh(u) · u'
    if head == SINH:
        (inner,) = f.args
        return IRApply(MUL, (IRApply(COSH, (inner,)), _diff(inner, x)))
    # d/dx cosh(u) = sinh(u) · u'
    if head == COSH:
        (inner,) = f.args
        return IRApply(MUL, (IRApply(SINH, (inner,)), _diff(inner, x)))
    # d/dx tanh(u) = sech²(u) · u' = u' / cosh²(u)
    if head == TANH:
        (inner,) = f.args
        return IRApply(
            DIV,
            (_diff(inner, x), IRApply(POW, (IRApply(COSH, (inner,)), IRInteger(2)))),
        )
    # d/dx asinh(u) = u' / sqrt(u² + 1)
    if head == ASINH:
        (inner,) = f.args
        u2_plus_1 = IRApply(ADD, (IRApply(POW, (inner, IRInteger(2))), ONE))
        denom = IRApply(SQRT, (u2_plus_1,))
        return IRApply(DIV, (_diff(inner, x), denom))
    # d/dx acosh(u) = u' / sqrt(u² - 1)
    if head == ACOSH:
        (inner,) = f.args
        u2_minus_1 = IRApply(SUB, (IRApply(POW, (inner, IRInteger(2))), ONE))
        denom = IRApply(SQRT, (u2_minus_1,))
        return IRApply(DIV, (_diff(inner, x), denom))
    # d/dx atanh(u) = u' / (1 - u²)
    if head == ATANH:
        (inner,) = f.args
        denom = IRApply(SUB, (ONE, IRApply(POW, (inner, IRInteger(2)))))
        return IRApply(DIV, (_diff(inner, x), denom))

    # -- Reciprocal hyperbolic functions (Phase 15) -----------------------
    #
    # Derivatives are expressed in terms of sinh/cosh (not in terms of
    # coth/sech/csch themselves) so the simplifier doesn't need to recurse
    # back through these heads.
    #
    # d/dx coth(u) = -u' / sinh²(u)
    if head == COTH:
        (inner,) = f.args
        denom = IRApply(POW, (IRApply(SINH, (inner,)), IRInteger(2)))
        return IRApply(NEG, (IRApply(DIV, (_diff(inner, x), denom)),))
    # d/dx sech(u) = -u'·sinh(u) / cosh²(u)
    if head == SECH:
        (inner,) = f.args
        numer = IRApply(MUL, (_diff(inner, x), IRApply(SINH, (inner,))))
        denom = IRApply(POW, (IRApply(COSH, (inner,)), IRInteger(2)))
        return IRApply(NEG, (IRApply(DIV, (numer, denom)),))
    # d/dx csch(u) = -u'·cosh(u) / sinh²(u)
    if head == CSCH:
        (inner,) = f.args
        numer = IRApply(MUL, (_diff(inner, x), IRApply(COSH, (inner,))))
        denom = IRApply(POW, (IRApply(SINH, (inner,)), IRInteger(2)))
        return IRApply(NEG, (IRApply(DIV, (numer, denom)),))

    # Unknown function — leave as ``D(f, x)`` unevaluated.
    return IRApply(D, (f, x))


def _depends_on(node: IRNode, var: IRSymbol) -> bool:
    """True if ``var`` appears anywhere inside ``node``."""
    if isinstance(node, IRSymbol):
        return node == var
    if isinstance(node, IRApply):
        return _depends_on(node.head, var) or any(
            _depends_on(a, var) for a in node.args
        )
    # Literals never depend on a variable.
    if isinstance(node, (IRInteger, IRFloat, IRRational)):
        return False
    return False
