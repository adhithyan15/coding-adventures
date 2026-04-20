"""Symbolic integration — the ``Integrate`` handler (Phases 1–2e).

The handler tries two routes, in order:

1. **Rational-function route (Phases 2c–2e)** — if the integrand is a
   rational function of ``x`` over Q, split off the polynomial part,
   run Hermite reduction, and produce the closed-form output:

   - Phase 2c: Hermite rational part (always closed-form).
   - Phase 2d: Rothstein–Trager log sum (when all log coefficients ∈ Q).
   - Phase 2e: Arctan formula for an irreducible quadratic log part
     (when RT returns None and deg(log_den) == 2, no rational roots).
   - Otherwise: unevaluated ``Integrate`` for the residual log part.

2. **Phase 1 route** — the "reverse derivative table". Covers linear
   combinations of elementary functions, power rule, constant factor,
   and a few hard-coded integration-by-parts cases (log, sqrt).

The rational route fires first because Hermite is a *decision
procedure* on its input domain — when the integrand is rational, it
gives an answer every time, and that answer has a cleaner structure
than whatever Phase 1 would have pattern-matched. Phase 1 remains the
fallback for everything else (trig, exp, mixed products, etc.).

What Phase 1 can do
-------------------

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
- Irreducible denominators of degree > 2 (e.g. ``1/(x³+x+1)``).
- Mixed denominators with both linear and quadratic irreducible factors.
- Any expression with two x-dependent, non-rational factors.

Anything we can't integrate comes back as ``Integrate(f, x)`` unchanged,
exactly as ``D`` does for unknown heads. The CAS stays consistent — it
never *claims* to have integrated something it didn't.

Only installed on :class:`~symbolic_vm.backends.SymbolicBackend`. In
strict mode ``Integrate`` falls through to
:meth:`Backend.on_unknown_head`, which raises.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    divmod_poly,
    normalize,
)
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

from symbolic_vm.arctan_integral import arctan_integral
from symbolic_vm.backend import Handler
from symbolic_vm.hermite import hermite_reduce
from symbolic_vm.polynomial_bridge import from_polynomial, to_rational
from symbolic_vm.rothstein_trager import rothstein_trager

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
        # Rational-function route first (Phase 2c). ``to_rational``
        # returns None for anything outside Q(x); only then do we fall
        # back to the Phase 1 pattern table.
        rational_result = _integrate_rational(f, x)
        if rational_result is not None:
            return vm.eval(rational_result)
        result = _integrate(f, x)
        if result is None:
            return IRApply(INTEGRATE, (f, x))
        return vm.eval(result)

    return handler


# ---------------------------------------------------------------------------
# Phase 2c — rational-function integration via Hermite reduction
# ---------------------------------------------------------------------------


def _integrate_rational(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Attempt the rational-function route. ``None`` means "not rational".

    When ``f`` is a rational function of ``x`` over Q, splits off the
    polynomial part (which integrates trivially via the power rule
    applied coefficient-wise), runs Hermite reduction on the proper
    rational remainder, and assembles the IR output:

        ∫ f dx  =  ∫ poly dx              (power rule)
                 + rat_num / rat_den      (Hermite rational part)
                 + Integrate(log_num / log_den, x)   (squarefree residual)

    Any of the three pieces may be zero; we skip them when so.

    Returns ``None`` for anything ``to_rational`` rejects — which is
    every non-rational integrand, so Phase 1 gets those.
    """
    r = to_rational(f, x)
    if r is None:
        return None
    num, den = r
    # Bail for trivial denominators — pure polynomial integrands have
    # a cleaner closed form through the Phase 1 Add / power-rule path,
    # which emits ``IRApply`` shapes the existing differentiator and
    # test suite are written against. Hermite has nothing to contribute
    # to a polynomial anyway (no repeated factors in the denominator).
    if len(normalize(den)) <= 1:
        return None
    # Polynomial division: f = q + r/den with deg r < deg den. ``q`` is
    # the polynomial part we integrate with the power rule; ``r`` is
    # the proper rational we feed to Hermite.
    quot, rem = divmod_poly(num, den)
    has_poly = bool(normalize(quot))

    hermite_rat = None
    hermite_log = None
    if normalize(rem):
        (hermite_rat, hermite_log) = hermite_reduce(rem, den)

    has_rat = hermite_rat is not None and bool(normalize(hermite_rat[0]))
    has_log = hermite_log is not None and bool(normalize(hermite_log[0]))

    # Try Rothstein–Trager up-front — a successful closed-form log sum
    # counts as "progress" just like a polynomial or Hermite rational
    # part. When RT returns ``None``, fall through to Phase 2e (arctan
    # formula for irreducible quadratic denominators).
    rt_pairs = None
    at_ir = None
    if has_log:
        rt_pairs = rothstein_trager(hermite_log[0], hermite_log[1])
        if rt_pairs is None:
            at_ir = _try_arctan_integral(hermite_log[0], hermite_log[1], x)

    # "Made progress" = we extracted something whose closed form Phase
    # 1 couldn't produce. Without a polynomial part, a Hermite
    # reduction, an RT log sum, or an arctan result, the output would
    # just echo the unevaluated input — return None so the handler can
    # fall through to Phase 1 or leave the integral unevaluated.
    if not (has_poly or has_rat or rt_pairs is not None or at_ir is not None):
        return None

    pieces: list[IRNode] = []

    # 1. Polynomial part — trivially integrated coefficient-by-coefficient.
    if has_poly:
        pieces.append(from_polynomial(_integrate_polynomial(quot), x))

    # 2. Hermite rational part.
    if has_rat:
        pieces.append(_rational_to_ir(hermite_rat[0], hermite_rat[1], x))

    # 3. Squarefree log residual — try RT (Phase 2d) then arctan (Phase 2e).
    # When both fail, emit an unevaluated ``Integrate``. Re-entry on
    # that Integrate returns None from _integrate_rational (RT will
    # say None again; Hermite on a squarefree denom does nothing), so
    # there's no infinite loop.
    if has_log:
        if rt_pairs is not None:
            pieces.append(_rt_pairs_to_ir(rt_pairs, x))
        elif at_ir is not None:
            pieces.append(at_ir)
        else:
            integrand = _rational_to_ir(hermite_log[0], hermite_log[1], x)
            pieces.append(IRApply(INTEGRATE, (integrand, x)))

    if len(pieces) == 1:
        return pieces[0]
    # Binary left-associative Add chain — the VM's Add handler is
    # strictly binary.
    acc = pieces[0]
    for piece in pieces[1:]:
        acc = IRApply(ADD, (acc, piece))
    return acc


def _integrate_polynomial(p: Polynomial) -> Polynomial:
    """Integrate a polynomial coefficient-wise: ``a_i · x^i → a_i/(i+1) · x^(i+1)``.

    The constant of integration is omitted (as everywhere in this
    handler). The result has length ``len(p) + 1`` before normalisation.
    """
    if not normalize(p):
        return ()
    # Leading zero is the new constant term (the +C we drop). Each
    # higher coefficient is ``a_i / (i+1)`` sitting at position i+1.
    result: list = [0]
    for i, c in enumerate(p):
        result.append(Fraction(c) / Fraction(i + 1))
    return normalize(tuple(result))


def _rt_pairs_to_ir(pairs, x: IRSymbol) -> IRNode:
    """Assemble ``Σ c_i · log(v_i)`` as IR from the Rothstein–Trager pairs.

    Each pair is ``(c: Fraction, v: Polynomial)`` with ``v`` monic and
    non-constant. The emitted IR is a left-associative binary ``Add``
    chain of log terms; the chain collapses to a single node when
    there's only one pair. A coefficient of ``1`` renders as bare
    ``Log(v)`` (no redundant ``Mul(1, ·)``); ``−1`` renders as
    ``Neg(Log(v))``.
    """
    terms: list[IRNode] = []
    for c, v in pairs:
        log_ir = IRApply(LOG, (from_polynomial(v, x),))
        if c == 1:
            terms.append(log_ir)
        elif c == -1:
            terms.append(IRApply(NEG, (log_ir,)))
        else:
            # Integer coefficients render as IRInteger; the general
            # rational case uses IRRational. This keeps the IR shape
            # aligned with what the Phase 1 rules (and the hand-rolled
            # tests for that path) already produce.
            if c.denominator == 1:
                coef: IRNode = IRInteger(c.numerator)
            else:
                coef = IRRational(c.numerator, c.denominator)
            terms.append(IRApply(MUL, (coef, log_ir)))
    if len(terms) == 1:
        return terms[0]
    acc = terms[0]
    for t in terms[1:]:
        acc = IRApply(ADD, (acc, t))
    return acc


def _try_arctan_integral(
    num: Polynomial, den: Polynomial, x: IRSymbol
) -> IRNode | None:
    """Phase 2e: arctan antiderivative for an irreducible quadratic denominator.

    Returns the closed-form IR when ``den`` is degree 2 with no rational
    roots, or ``None`` if the denominator doesn't fit that shape.
    """
    from polynomial import rational_roots
    den_n = normalize(den)
    if len(den_n) - 1 != 2:
        return None
    if rational_roots(den_n):
        return None
    return arctan_integral(num, den, x)


def _rational_to_ir(
    num: Polynomial, den: Polynomial, x: IRSymbol
) -> IRNode:
    """Build the IR for ``num(x) / den(x)``.

    If ``den`` is the constant polynomial ``1``, we emit just the
    numerator — avoids wrapping every rational in a trivial ``Div``.
    """
    num_ir = from_polynomial(num, x)
    den_n = normalize(den)
    if den_n == (Fraction(1),) or den_n == (1,):
        return num_ir
    den_ir = from_polynomial(den, x)
    return IRApply(DIV, (num_ir, den_ir))


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
