"""Symbolic integration — the ``Integrate`` handler (Phase 1).

Phase 1 is the "reverse derivative table" layer. Every rule here is the
product rule, the chain rule, or the derivative of a single elementary
function, read backwards. None of it is numeric — we emit binary applies
and let the VM's arithmetic handlers simplify.

What this phase can do
----------------------

- Constants:            ``∫ c dx = c·x``
- Power rule:           ``∫ x^n dx = x^(n+1) / (n+1)`` (with
                        ``n = -1`` specialised to ``log(x)``)
- Linearity:            ``∫ (f ± g) dx = ∫f ± ∫g``, ``∫ -f = -∫f``
- Constant factor:      ``∫ c·f dx = c·∫f dx``  (when ``c`` is free of x)
- Elementary:           ``∫ sin x = -cos x``, ``∫ cos x = sin x``,
                        ``∫ exp x = exp x``, ``∫ sqrt x = (2/3)·x^(3/2)``,
                        ``∫ log x = x·log x - x``
- Exponential const:    ``∫ a^x dx = a^x / log(a)``  (a independent of x)

What this phase can't do (yet)
------------------------------

- Integration by substitution (except trivial constant-factor / sum).
- Integration by parts (except the hard-coded ``log`` case).
- Rational functions that aren't already a single power of x — partial
  fractions and Hermite reduction are Phase 2.
- Any expression with two x-dependent factors in a product.

Anything we can't integrate comes back as ``Integrate(f, x)`` unchanged,
exactly as ``D`` does for unknown heads. The CAS stays consistent — it
never *claims* to have integrated something it didn't.

Only installed on :class:`~symbolic_vm.backends.SymbolicBackend`. In
strict mode ``Integrate`` falls through to
:meth:`Backend.on_unknown_head`, which raises.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SQRT,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backend import Handler

ONE = IRInteger(1)
TWO = IRInteger(2)


def integrate() -> Handler:
    """Return the ``Integrate`` handler for the symbolic backend."""

    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 2:
            raise TypeError(
                f"Integrate expects 2 arguments, got {len(expr.args)}"
            )
        f, x = expr.args
        if not isinstance(x, IRSymbol):
            # Integration with respect to something other than a plain
            # symbol is meaningless — leave the expression.
            return expr
        result = _integrate(f, x)
        if result is None:
            return IRApply(INTEGRATE, (f, x))
        return vm.eval(result)

    return handler


def _integrate(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Return an antiderivative of ``f`` w.r.t. ``x``, or ``None``.

    ``None`` means "I don't know a rule for this shape" — the caller
    will emit an unevaluated ``Integrate`` node.

    The caller is expected to feed the result back through ``vm.eval``
    so arithmetic identities collapse the output.
    """
    # ∫ c dx = c·x when c is free of x. Covers literals and every
    # symbol except x itself.
    if not _depends_on(f, x):
        return IRApply(MUL, (f, x))

    # ∫ x dx = (1/2)·x^2. Special-case the bare variable so we emit the
    # clean form instead of going through the Power-rule branch.
    if f == x:
        return IRApply(
            MUL, (IRRational(1, 2), IRApply(POW, (x, TWO)))
        )

    # Only compound forms remain — atoms that depended on x and weren't
    # x itself are unreachable in a well-typed tree.
    if not isinstance(f, IRApply):
        return None

    head = f.head

    # --- Linearity ----------------------------------------------------
    if head == ADD:
        pieces = []
        for arg in f.args:
            piece = _integrate(arg, x)
            if piece is None:
                return None
            pieces.append(piece)
        return IRApply(ADD, tuple(pieces))

    if head == SUB:
        a, b = f.args
        ia = _integrate(a, x)
        ib = _integrate(b, x)
        if ia is None or ib is None:
            return None
        return IRApply(SUB, (ia, ib))

    if head == NEG:
        (a,) = f.args
        ia = _integrate(a, x)
        if ia is None:
            return None
        return IRApply(NEG, (ia,))

    # --- Constant factor (product with an x-free operand) -------------
    if head == MUL:
        a, b = f.args
        if not _depends_on(a, x):
            ib = _integrate(b, x)
            return None if ib is None else IRApply(MUL, (a, ib))
        if not _depends_on(b, x):
            ia = _integrate(a, x)
            return None if ia is None else IRApply(MUL, (b, ia))
        # Both factors depend on x — Phase 1 gives up. Integration by
        # parts lives in a later phase.
        return None

    # --- Quotient (limited: constant denominator, or 1/x) -------------
    if head == DIV:
        a, b = f.args
        # ∫ (a/b) dx = (∫a dx) / b when b is free of x.
        if not _depends_on(b, x):
            ia = _integrate(a, x)
            return None if ia is None else IRApply(DIV, (ia, b))
        # ∫ (a/x) dx = a·log(x) when a is free of x. The only
        # x-in-denominator shape Phase 1 handles directly.
        if b == x and not _depends_on(a, x):
            return IRApply(MUL, (a, IRApply(LOG, (x,))))
        return None

    # --- Power rule ---------------------------------------------------
    if head == POW:
        base, exponent = f.args
        # ∫ x^n dx for exponent independent of x.
        if base == x and not _depends_on(exponent, x):
            if _is_minus_one(exponent):
                return IRApply(LOG, (x,))
            # For integer exponents we can fold ``1/(n+1)`` into a
            # single ``IRRational`` literal. That keeps the result in
            # the ``Mul(coef, Pow(x, m))`` shape that the simplifier
            # (and a future differentiator) can reason about.
            if isinstance(exponent, IRInteger):
                new_n = exponent.value + 1
                return IRApply(
                    MUL,
                    (
                        IRRational(1, new_n),
                        IRApply(POW, (x, IRInteger(new_n))),
                    ),
                )
            # Symbolic exponent — emit the textbook form. We don't yet
            # try to simplify ``(n+1)`` when n is a free symbol.
            new_exp = IRApply(ADD, (exponent, ONE))
            return IRApply(DIV, (IRApply(POW, (x, new_exp)), new_exp))
        # ∫ a^x dx = a^x / log(a) for base independent of x.
        if exponent == x and not _depends_on(base, x):
            return IRApply(DIV, (f, IRApply(LOG, (base,))))
        return None

    # --- Elementary functions at x ------------------------------------
    # These only fire for the direct ``head(x)`` shape. Chain-rule
    # composition (e.g. ``sin(2*x)``) is left for later phases; a full
    # answer requires substitution.
    if len(f.args) == 1 and f.args[0] == x:
        if head == SIN:
            return IRApply(NEG, (IRApply(COS, (x,)),))
        if head == COS:
            return IRApply(SIN, (x,))
        if head == EXP:
            return IRApply(EXP, (x,))
        if head == LOG:
            # ∫ log(x) dx = x·log(x) - x  (integration by parts,
            # hard-coded because it's the canonical example).
            return IRApply(
                SUB,
                (IRApply(MUL, (x, IRApply(LOG, (x,)))), x),
            )
        if head == SQRT:
            # ∫ sqrt(x) dx = (2/3)·x^(3/2)
            return IRApply(
                MUL,
                (
                    IRRational(2, 3),
                    IRApply(POW, (x, IRRational(3, 2))),
                ),
            )

    # Unknown shape — signal "no rule" to the caller.
    return None


def _is_minus_one(node: IRNode) -> bool:
    """True iff ``node`` is the integer literal ``-1``.

    We detect both the direct ``IRInteger(-1)`` and the wrapped
    ``Neg(IRInteger(1))`` — either form can appear depending on how a
    user wrote the expression.
    """
    if isinstance(node, IRInteger) and node.value == -1:
        return True
    return (
        isinstance(node, IRApply)
        and node.head == NEG
        and len(node.args) == 1
        and isinstance(node.args[0], IRInteger)
        and node.args[0].value == 1
    )


def _depends_on(node: IRNode, var: IRSymbol) -> bool:
    """True if ``var`` appears anywhere inside ``node``."""
    if isinstance(node, IRSymbol):
        return node == var
    if isinstance(node, IRApply):
        return _depends_on(node.head, var) or any(
            _depends_on(a, var) for a in node.args
        )
    if isinstance(node, (IRInteger, IRFloat, IRRational)):
        return False
    return False
