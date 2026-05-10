"""Symbolic integration — the ``Integrate`` handler (Phases 1–24).

The handler tries two routes, in order:

1. **Rational-function route (Phases 2c–2f, 9–10)** — if the integrand is a
   rational function of ``x`` over Q, split off the polynomial part,
   run Hermite reduction, and produce the closed-form output:

   - Phase 2c: Hermite rational part (always closed-form).
   - Phase 2d: Rothstein–Trager log sum (when all log coefficients ∈ Q,
     and degree of denominator < 6).
   - Phase 2e: Arctan formula for an irreducible quadratic log part
     (when RT returns None and deg(log_den) == 2, no rational roots).
   - Phase 2f: Mixed partial-fraction split (L·Q denominator) when 2e
     returns None: separates into linear-factors piece (→ RT) and single
     irreducible-quadratic piece (→ arctan).
   - Phase 9: Two-distinct-irreducible-quadratic denominator — biquadratic
     factoring + partial fractions, each piece delegated to Phase 2e.
   - Phase 10: Generalized partial fractions — three distinct irreducible
     quadratics (Q₁·Q₂·Q₃) or any linear factors × two quadratics
     (Lᵐ·Q₁·Q₂). Uses a D×D partial-fraction system solved over Q.
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

- Integration by substitution beyond Phase 8 (two-arg non-POW outer functions).
- Rational × transcendental (e.g. ``(1/x)·eˣ``).
- Products of two transcendentals (e.g. ``exp(x)·log(x)``).
- Irreducible denominators of degree > 2 (e.g. ``1/(x³+x+1)``).
- Denominators with four or more distinct irreducible quadratic factors.
- Denominators of degree > 6 (general multi-factor case).
- Irrational factorizations (e.g. ``x⁴+1`` over Q(√2) only).

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
    multiply,
    normalize,
    rational_roots,
)
from symbolic_ir import (
    ACOS,
    ACOSH,
    ADD,
    ASIN,
    ASINH,
    ATAN,
    ATANH,
    COS,
    COSH,
    COTH,
    CSCH,
    DIV,
    EXP,
    INTEGRATE,
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
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.arctan_integral import arctan_integral
from symbolic_vm.asin_poly_integral import acos_poly_integral, asin_poly_integral
from symbolic_vm.asinh_poly_integral import acosh_poly_integral, asinh_poly_integral
from symbolic_vm.atan_poly_integral import atan_poly_integral
from symbolic_vm.atanh_poly_integral import atanh_poly_integral
from symbolic_vm.backend import Handler
from symbolic_vm.definite_integral import evaluate_definite
from symbolic_vm.exp_hyp_integral import (
    exp_cosh_integral,
    exp_hyp_degenerate,
    exp_sinh_integral,
)
from symbolic_vm.exp_integral import exp_integral
from symbolic_vm.exp_trig_integral import exp_cos_integral, exp_sin_integral
from symbolic_vm.hermite import hermite_reduce
from symbolic_vm.hyp_power_integral import (
    cosh_power_integral,
    sinh_power_integral,
    sinh_times_cosh_power,
)
from symbolic_vm.log_integral import log_poly_integral
from symbolic_vm.mixed_integral import mixed_integral
from symbolic_vm.polynomial_bridge import (
    from_polynomial,
    linear_to_ir,
    rt_pairs_to_ir,
    to_rational,
)
from symbolic_vm.recip_hyp_power_integral import (
    coth_power_integral,
    csch_power_integral,
    sech_power_integral,
    tanh_power_integral,
)
from symbolic_vm.rothstein_trager import rothstein_trager
from symbolic_vm.sinh_poly_integral import cosh_poly_integral, sinh_poly_integral
from symbolic_vm.special_functions import INTEGRATION_FALLBACKS
from symbolic_vm.trig_poly_integral import trig_cos_integral, trig_sin_integral

ONE = IRInteger(1)
TWO = IRInteger(2)


def integrate() -> Handler:
    """Return the ``Integrate`` handler for the symbolic backend."""

    def handler(vm, expr: IRApply) -> IRNode:
        nargs = len(expr.args)
        if nargs not in {2, 4}:
            raise TypeError(
                f"Integrate expects 2 or 4 arguments, got {nargs}"
            )

        if nargs == 4:
            # ---------------------------------------------------------------
            # Phase 24: definite integration  Integrate(f, x, a, b)
            #
            # Strategy:
            #   1. Compute the indefinite antiderivative F with the same
            #      three routes used for the 2-arg form.
            #   2. Delegate to evaluate_definite which applies F(b) − F(a),
            #      handling infinite limits via _eval_at_inf.
            # ---------------------------------------------------------------
            f, x, a, b = expr.args
            if not isinstance(x, IRSymbol):
                return expr

            F: IRNode | None = None

            # Route 1: rational function.
            rational_result = _integrate_rational(f, x)
            if rational_result is not None:
                F = vm.eval(rational_result)
                # If the rational result still has an unevaluated Integrate
                # sub-expression, fall through and return unevaluated.
                if _contains_integrate(F):
                    return IRApply(INTEGRATE, (f, x, a, b))
            else:
                # Route 2: Phase 1 pattern table.
                F = _integrate(f, x)
                if F is None:
                    # Route 3: Phase 23 special-function fallbacks.
                    for _sf_try in INTEGRATION_FALLBACKS:
                        F = _sf_try(f, x)
                        if F is not None:
                            break
                if F is None:
                    # No antiderivative found — leave unevaluated.
                    return IRApply(INTEGRATE, (f, x, a, b))
                F = vm.eval(F)
                if _contains_integrate(F):
                    return IRApply(INTEGRATE, (f, x, a, b))

            return evaluate_definite(f, x, a, b, F, vm)

        # nargs == 2 --------------------------------------------------------
        # Indefinite integration (original behaviour, Phases 1–23).
        # -------------------------------------------------------------------
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
            # Phase 23: special-function fallback — try erf, Si/Ci, Li₂,
            # Fresnel after all elementary routes have returned None.
            # Placed here (in the outer handler) so it fires regardless of
            # which early-return path inside _integrate was taken.
            for _sf_try in INTEGRATION_FALLBACKS:
                _sf_result = _sf_try(f, x)
                if _sf_result is not None:
                    return vm.eval(_sf_result)
            return IRApply(INTEGRATE, (f, x))
        return vm.eval(result)

    return handler


# ---------------------------------------------------------------------------
# Phase 24 helper — detect unevaluated Integrate nodes in a result
# ---------------------------------------------------------------------------


def _contains_integrate(expr: IRNode) -> bool:
    """Return True if *expr* contains any ``Integrate(...)`` sub-expression.

    Used by the definite-integration handler to bail out when the
    rational-function route returns a partially-unevaluated result (i.e. an
    integral whose transcendental residual could not be expressed in terms of
    elementary functions or the Phase 23 special functions).
    """
    if isinstance(expr, IRApply):
        if expr.head == INTEGRATE:
            return True
        return any(_contains_integrate(a) for a in expr.args)
    return False


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
    mixed_ir = None
    multi_ir = None
    general_ir = None
    if has_log:
        # Phase 2d: Rothstein–Trager.
        # Guard: degree ≥ 6 causes Sylvester-matrix coefficient explosion
        # (measured: 26 ks for three-quadratic denominator). Phase 10
        # handles those cases; skip RT to avoid the hang.
        den_n = normalize(hermite_log[1])
        if len(den_n) - 1 < 6:
            rt_pairs = rothstein_trager(hermite_log[0], hermite_log[1])
        if rt_pairs is None:
            at_ir = _try_arctan_integral(hermite_log[0], hermite_log[1], x)
        if rt_pairs is None and at_ir is None:
            mixed_ir = mixed_integral(hermite_log[0], hermite_log[1], x)
        # Phase 9: two distinct irreducible quadratic factors.
        if rt_pairs is None and at_ir is None and mixed_ir is None:
            multi_ir = _try_multi_quad_integral(
                hermite_log[0], hermite_log[1], x
            )
        # Phase 10: three quadratics, or linear factors × two quadratics.
        if rt_pairs is None and at_ir is None and mixed_ir is None and multi_ir is None:
            general_ir = _try_general_rational_integral(
                hermite_log[0], hermite_log[1], x
            )

    # "Made progress" = we extracted something whose closed form Phase
    # 1 couldn't produce. Without a polynomial part, a Hermite
    # reduction, an RT log sum, an arctan result, a mixed result, a
    # multi-quadratic result, or a general Phase 10 result, the output
    # would just echo the unevaluated input — return None so the handler
    # can fall through to Phase 1 or leave the integral unevaluated.
    if not (has_poly or has_rat or rt_pairs is not None
            or at_ir is not None or mixed_ir is not None
            or multi_ir is not None or general_ir is not None):
        return None

    pieces: list[IRNode] = []

    # 1. Polynomial part — trivially integrated coefficient-by-coefficient.
    if has_poly:
        pieces.append(from_polynomial(_integrate_polynomial(quot), x))

    # 2. Hermite rational part.
    if has_rat:
        pieces.append(_rational_to_ir(hermite_rat[0], hermite_rat[1], x))

    # 3. Squarefree log residual — try RT (2d), arctan (2e), mixed (2f).
    # When all fail, emit an unevaluated ``Integrate``. Re-entry on
    # that Integrate returns None from _integrate_rational (RT will
    # say None again; Hermite on a squarefree denom does nothing), so
    # there's no infinite loop.
    if has_log:
        if rt_pairs is not None:
            pieces.append(_rt_pairs_to_ir(rt_pairs, x))
        elif at_ir is not None:
            pieces.append(at_ir)
        elif mixed_ir is not None:
            pieces.append(mixed_ir)
        elif multi_ir is not None:
            pieces.append(multi_ir)
        elif general_ir is not None:
            pieces.append(general_ir)
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
    """Thin wrapper — delegates to ``rt_pairs_to_ir`` in polynomial_bridge."""
    return rt_pairs_to_ir(pairs, x)


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
        # Neg distribution: Mul(f, Neg(g)) → Neg(Mul(f, g)) so NEG rule fires.
        # This lets ∫ exp(x)·sinh(-x) dx work after Phase 31 simplifies
        # sinh(-x) → Neg(Sinh(x)) before the integrand reaches us.
        if isinstance(b, IRApply) and b.head == NEG and len(b.args) == 1:
            inner = _integrate(IRApply(MUL, (a, b.args[0])), x)
            return None if inner is None else IRApply(NEG, (inner,))
        if isinstance(a, IRApply) and a.head == NEG and len(a.args) == 1:
            inner = _integrate(IRApply(MUL, (a.args[0], b)), x)
            return None if inner is None else IRApply(NEG, (inner,))
        # Both factors depend on x — try Phase 3 then Phase 4 patterns.
        # Phase 3: exp(linear) and log(linear) products.
        result = _try_exp_product(a, b, x) or _try_exp_product(b, a, x)
        if result is not None:
            return result
        result = _try_log_product(a, b, x) or _try_log_product(b, a, x)
        if result is not None:
            return result
        # Phase 11: atan(linear) × polynomial via IBP.
        result = _try_atan_product(a, b, x) or _try_atan_product(b, a, x)
        if result is not None:
            return result
        # Phase 12: asin/acos(linear) × polynomial via IBP.
        result = _try_asin_product(a, b, x) or _try_asin_product(b, a, x)
        if result is not None:
            return result
        result = _try_acos_product(a, b, x) or _try_acos_product(b, a, x)
        if result is not None:
            return result
        # Phase 13: sinh/cosh/asinh/acosh(linear) × polynomial via IBP.
        result = _try_sinh_product(a, b, x) or _try_sinh_product(b, a, x)
        if result is not None:
            return result
        result = _try_cosh_product(a, b, x) or _try_cosh_product(b, a, x)
        if result is not None:
            return result
        result = _try_asinh_product(a, b, x) or _try_asinh_product(b, a, x)
        if result is not None:
            return result
        result = _try_acosh_product(a, b, x) or _try_acosh_product(b, a, x)
        if result is not None:
            return result
        # Phase 14c: atanh(linear) × polynomial via IBP.
        result = _try_atanh_product(a, b, x) or _try_atanh_product(b, a, x)
        if result is not None:
            return result
        # Phase 14a: exp(linear) × sinh/cosh(linear) — double-IBP closed form.
        result = _try_exp_hyp(a, b, x) or _try_exp_hyp(b, a, x)
        if result is not None:
            return result
        # Phase 14b: Pow(Sinh, m) × Pow(Cosh, n) — u-substitution (odd exponent).
        result = _try_sinh_cosh_product(a, b, x)
        if result is not None:
            return result
        # Phase 4b: trig × trig via product-to-sum identities.
        result = _try_trig_trig(a, b, x) or _try_trig_trig(b, a, x)
        if result is not None:
            return result
        # Phase 6: sinⁿ × cosᵐ with the same linear argument.
        result = _try_sin_cos_power(a, b, x)
        if result is not None:
            return result
        # Phase 7: u-substitution — f(g(x)) · c·g'(x).
        result = _try_u_sub(a, b, x)
        if result is not None:
            return result
        # Phase 8: u-substitution for POW(f(g(x)), n) · c·g'(x).
        result = _try_u_sub_pow(a, b, x)
        if result is not None:
            return result
        # Phase 4c: exp × trig via double-IBP closed form.
        result = _try_exp_trig(a, b, x) or _try_exp_trig(b, a, x)
        if result is not None:
            return result
        # Phase 4a: polynomial × trig via tabular IBP.
        result = _try_trig_product(a, b, x) or _try_trig_product(b, a, x)
        return result  # None if all patterns failed

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
        # f^0 = 1 — integrates as a constant.
        if isinstance(exponent, IRInteger) and exponent.value == 0:
            return x
        # f^1 = f — unwrap and delegate.
        if isinstance(exponent, IRInteger) and exponent.value == 1:
            return _integrate(base, x)
        # Phase 5b/5c: sinⁿ, cosⁿ, tanⁿ reduction formulas.
        result = _try_trig_power(base, exponent, x)
        if result is not None:
            return result
        # Phase 14: sinhⁿ, coshⁿ reduction formulas.
        result = _try_hyp_power(base, exponent, x)
        if result is not None:
            return result
        # Phase 16: sechⁿ, cschⁿ, cothⁿ reduction formulas.
        result = _try_recip_hyp_power(base, exponent, x)
        if result is not None:
            return result
        # Phase 8 bonus: ∫ (ax+b)^n dx = (ax+b)^(n+1)/((n+1)·a) or log(ax+b)/a.
        lin = _try_linear(base, x)
        if lin is not None:
            a_lin, b_lin = lin
            if a_lin != Fraction(0):
                arg_ir = base
                a_ir = _frac_ir(a_lin)
                if _is_minus_one(exponent):
                    # ∫ (ax+b)^(-1) dx = log(ax+b) / a
                    return IRApply(DIV, (IRApply(LOG, (arg_ir,)), a_ir))
                # ∫ (ax+b)^n dx = (ax+b)^(n+1) / ((n+1)·a)
                if isinstance(exponent, IRInteger):
                    new_n = exponent.value + 1
                    new_exp = IRInteger(new_n)
                    denom = IRApply(MUL, (IRInteger(new_n), a_ir))
                    return IRApply(DIV, (IRApply(POW, (arg_ir, new_exp)), denom))
                # Symbolic / rational exponent — use ADD form.
                new_exp = IRApply(ADD, (exponent, ONE))
                denom = IRApply(MUL, (new_exp, a_ir))
                return IRApply(DIV, (IRApply(POW, (arg_ir, new_exp)), denom))
        return None

    # --- Elementary functions at x  (Phase 1: argument must be bare x)
    if len(f.args) == 1 and f.args[0] == x:
        if head == SIN:
            return IRApply(NEG, (IRApply(COS, (x,)),))
        if head == COS:
            return IRApply(SIN, (x,))
        if head == TAN:
            # ∫ tan(x) dx = −log(cos(x))  (Phase 5a, a=1 b=0 case).
            return IRApply(NEG, (IRApply(LOG, (IRApply(COS, (x,)),)),))
        if head == EXP:
            return IRApply(EXP, (x,))
        if head == LOG:
            # ∫ log(x) dx = x·log(x) - x.
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

    # --- Phase 3a/3b/3c + Phase 5a + Phase 9 bonus: elementary fn of linear arg ---
    # Generalises the Phase 1 rules above to any a·x + b argument.
    # Only fires when the argument is strictly linear with a ≠ 0 and
    # (a, b) ≠ (1, 0) — otherwise the Phase 1 rules above already fired.
    if len(f.args) == 1 and head in {
        EXP, SIN, COS, LOG, TAN, ATAN, ASIN, ACOS,
        SINH, COSH, TANH, ASINH, ACOSH, ATANH,
        COTH, SECH, CSCH,                           # Phase 15
    }:
        lin = _try_linear(f.args[0], x)
        if lin is not None:
            a_frac, b_frac = lin
            if a_frac != 0:
                if head == EXP:
                    # ∫ exp(ax+b) dx = exp(ax+b)/a  (case 3a).
                    # Delegate to exp_integral with p = (1,).
                    from fractions import Fraction as _F
                    return exp_integral((_F(1),), a_frac, b_frac, x)
                if head == SIN:
                    # ∫ sin(ax+b) dx = -cos(ax+b)/a  (case 3b).
                    cos_ir = IRApply(COS, (f.args[0],))
                    a_ir = _frac_ir(a_frac)
                    return IRApply(NEG, (IRApply(DIV, (cos_ir, a_ir)),))
                if head == COS:
                    # ∫ cos(ax+b) dx = sin(ax+b)/a  (case 3c).
                    sin_ir = IRApply(SIN, (f.args[0],))
                    a_ir = _frac_ir(a_frac)
                    return IRApply(DIV, (sin_ir, a_ir))
                if head == LOG:
                    # ∫ log(ax+b) dx = case 3e with p = (1,).
                    from fractions import Fraction as _F
                    return log_poly_integral((_F(1),), a_frac, b_frac, x)
                if head == TAN:
                    # ∫ tan(ax+b) dx = −log(cos(ax+b))/a  (case 5a).
                    return _tan_integral(a_frac, b_frac, x)
                if head == ATAN:
                    # ∫ atan(ax+b) dx = ((ax+b)/a)·atan(ax+b) − (1/(2a))·log((ax+b)²+1)
                    # IBP: u=atan(ax+b), dv=dx, du=a/((ax+b)²+1)dx, v=x.
                    # Working out the v·du integral gives the b/a·atan correction:
                    #   ∫ atan(ax+b) dx = (x+b/a)·atan(ax+b) − (1/(2a))·log((ax+b)²+1)
                    # Equivalently the leading factor is (ax+b)/a.
                    arg_ir = f.args[0]  # ax+b
                    a_ir = _frac_ir(a_frac)
                    coef_ir = IRApply(DIV, (arg_ir, a_ir))  # (ax+b)/a
                    denom_sq_ir = IRApply(
                        ADD, (IRApply(POW, (arg_ir, TWO)), ONE)
                    )
                    log_part = IRApply(
                        DIV,
                        (
                            IRApply(LOG, (denom_sq_ir,)),
                            IRApply(MUL, (TWO, a_ir)),
                        ),
                    )
                    return IRApply(SUB, (IRApply(MUL, (coef_ir, f)), log_part))
                if head in (ASIN, ACOS):
                    # ∫ asin/acos(ax+b) dx — unit-polynomial IBP (Phase 12).
                    fn = asin_poly_integral if head == ASIN else acos_poly_integral
                    return fn((Fraction(1),), a_frac, b_frac, x)
                if head == SINH:
                    # ∫ sinh(ax+b) dx = (1/a)·cosh(ax+b).
                    return sinh_poly_integral((Fraction(1),), a_frac, b_frac, x)
                if head == COSH:
                    # ∫ cosh(ax+b) dx = (1/a)·sinh(ax+b).
                    return cosh_poly_integral((Fraction(1),), a_frac, b_frac, x)
                if head == TANH:
                    # ∫ tanh(ax+b) dx = (1/a)·log(cosh(ax+b)).
                    return _tanh_integral(a_frac, b_frac, x)
                if head == ASINH:
                    # ∫ asinh(ax+b) dx — unit-polynomial IBP (Phase 13).
                    return asinh_poly_integral((Fraction(1),), a_frac, b_frac, x)
                if head == ACOSH:
                    # ∫ acosh(ax+b) dx — unit-polynomial IBP (Phase 13).
                    return acosh_poly_integral((Fraction(1),), a_frac, b_frac, x)
                if head == ATANH:
                    # ∫ atanh(ax+b) dx = (ax+b)/a·atanh(ax+b) + (1/(2a))·log(1−(ax+b)²).
                    return _atanh_integral(a_frac, b_frac, f.args[0], x)
                if head == COTH:
                    # ∫ coth(ax+b) dx = (1/a)·log(sinh(ax+b)).
                    return _coth_integral(a_frac, b_frac, x)
                if head == SECH:
                    # ∫ sech(ax+b) dx = (1/a)·atan(sinh(ax+b)).
                    return _sech_integral(a_frac, b_frac, x)
                if head == CSCH:
                    # ∫ csch(ax+b) dx = (1/a)·log(tanh((ax+b)/2)).
                    return _csch_integral(a_frac, b_frac, x)

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


# ---------------------------------------------------------------------------
# Phase 3 helpers
# ---------------------------------------------------------------------------


def _try_linear(node: IRNode, x: IRSymbol) -> tuple[Fraction, Fraction] | None:
    """Return ``(a, b)`` if ``node`` represents ``a·x + b`` over Q, else ``None``.

    Handles the IR shapes that the MACSYMA compiler and ``from_polynomial``
    emit for linear polynomials: bare ``x``, integer/rational constants,
    ``Neg``, ``Mul(coef, x)``, ``Add(u, v)``, ``Sub(u, v)``.
    Returns ``None`` for any quadratic or higher term, any float, or any
    free symbol other than ``x``.
    """
    if isinstance(node, IRInteger):
        return (Fraction(0), Fraction(node.value))
    if isinstance(node, IRRational):
        return (Fraction(0), Fraction(node.numer, node.denom))
    if isinstance(node, IRFloat):
        return None  # floats break exact arithmetic
    if isinstance(node, IRSymbol):
        if node == x:
            return (Fraction(1), Fraction(0))
        return None  # free symbol — can't treat as rational constant
    if not isinstance(node, IRApply):
        return None

    head = node.head
    if head == NEG:
        inner = _try_linear(node.args[0], x)
        if inner is None:
            return None
        a, b = inner
        return (-a, -b)
    if head == MUL:
        left, right = node.args
        # c·x or x·c where c is free of x.
        if right == x and not _depends_on(left, x):
            c = _node_to_frac(left)
            return (c, Fraction(0)) if c is not None else None
        if left == x and not _depends_on(right, x):
            c = _node_to_frac(right)
            return (c, Fraction(0)) if c is not None else None
        # Both depend on x — quadratic or higher.
        return None
    if head == ADD:
        u = _try_linear(node.args[0], x)
        v = _try_linear(node.args[1], x)
        if u is None or v is None:
            return None
        return (u[0] + v[0], u[1] + v[1])
    if head == SUB:
        u = _try_linear(node.args[0], x)
        v = _try_linear(node.args[1], x)
        if u is None or v is None:
            return None
        return (u[0] - v[0], u[1] - v[1])
    if head == DIV:
        # f/c — only when the divisor is a non-zero rational constant.
        divisor = _node_to_frac(node.args[1])
        if divisor is None or divisor == 0 or _depends_on(node.args[1], x):
            return None
        inner = _try_linear(node.args[0], x)
        if inner is None:
            return None
        return (inner[0] / divisor, inner[1] / divisor)
    return None


def _node_to_frac(node: IRNode) -> Fraction | None:
    """Return a ``Fraction`` if ``node`` is a numeric literal, else ``None``."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _try_exp_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · exp(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Exp(linear)`` and
    ``poly_candidate`` is a polynomial (rational with denominator 1).
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != EXP:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None  # exp(b) is a constant — constant-factor rule handles it

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None  # denominator is not 1 — rational, not polynomial
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return exp_integral(poly, a_frac, b_frac, x)


def _try_log_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · log(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Log(linear)`` and
    ``poly_candidate`` is a polynomial (rational with denominator 1).
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != LOG:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None  # log(b) is a constant — constant-factor rule handles it

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None  # rational, not polynomial
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return log_poly_integral(poly, a_frac, b_frac, x)


def _try_atan_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · atan(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Atan(linear)`` and
    ``poly_candidate`` is a polynomial (rational with denominator 1).
    Phase 11 IBP formula: Q(x)·atan − a·(T(x) + arctan_integral(R, D)).
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ATAN:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None  # atan(b) is a constant — constant-factor rule handles it

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None  # rational, not polynomial
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return atan_poly_integral(poly, a_frac, b_frac, x)


def _try_asin_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · asin(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Asin(linear)`` and
    ``poly_candidate`` is a polynomial. Phase 12 IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ASIN:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return asin_poly_integral(poly, a_frac, b_frac, x)


def _try_acos_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · acos(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Acos(linear)`` and
    ``poly_candidate`` is a polynomial. Phase 12 IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ACOS:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return acos_poly_integral(poly, a_frac, b_frac, x)


def _try_atanh_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · atanh(linear) dx`` or ``None``.

    Checks whether ``transcendental`` is ``Atanh(linear)`` and
    ``poly_candidate`` is a polynomial. Phase 14c IBP formula:

        Q(x)·atanh − a·T(x) + (r₁/(2a))·log(1−(ax+b)²)

    where Q = ∫P, T = ∫S, and r₁, r₀ are the remainder from
    dividing Q by 1−(ax+b)².
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ATANH:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None  # atanh(b) is a constant — constant-factor rule handles it

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None  # rational, not polynomial
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return atanh_poly_integral(poly, a_frac, b_frac, x)


def _try_sinh_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · sinh(linear) dx`` or ``None``.

    Phase 13 tabular IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != SINH:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return sinh_poly_integral(poly, a_frac, b_frac, x)


def _try_cosh_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · cosh(linear) dx`` or ``None``.

    Phase 13 tabular IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != COSH:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return cosh_poly_integral(poly, a_frac, b_frac, x)


def _try_asinh_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · asinh(linear) dx`` or ``None``.

    Phase 13 reduction IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ASINH:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return asinh_poly_integral(poly, a_frac, b_frac, x)


def _try_acosh_product(
    transcendental: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · acosh(linear) dx`` or ``None``.

    Phase 13 reduction IBP formula.
    """
    if not isinstance(transcendental, IRApply):
        return None
    if transcendental.head != ACOSH:
        return None
    lin = _try_linear(transcendental.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None
    return acosh_poly_integral(poly, a_frac, b_frac, x)


# ---------------------------------------------------------------------------
# Phase 4 helpers
# ---------------------------------------------------------------------------


def _try_trig_product(
    trig: IRNode, poly_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ poly_candidate · sin/cos(linear) dx`` or ``None``.

    Checks whether ``trig`` is ``Sin`` or ``Cos`` of a linear argument and
    ``poly_candidate`` is a polynomial (rational with denominator 1).
    """
    if not isinstance(trig, IRApply):
        return None
    if trig.head not in {SIN, COS}:
        return None
    lin = _try_linear(trig.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == 0:
        return None  # constant trig factor — constant-factor rule handles it

    r = to_rational(poly_candidate, x)
    if r is None:
        return None
    num, den = r
    from polynomial import normalize as _norm
    if len(_norm(den)) > 1:
        return None  # rational, not polynomial
    poly = tuple(Fraction(c) for c in _norm(num))
    if not poly:
        return None

    if trig.head == SIN:
        return trig_sin_integral(poly, a_frac, b_frac, x)
    return trig_cos_integral(poly, a_frac, b_frac, x)


def _try_hyp_power(base: IRNode, exponent: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ sinh^n(linear) dx`` or ``∫ cosh^n(linear) dx``, or ``None``.

    Phase 14 — hyperbolic power reduction.

    Fires when:
    - ``base`` is ``IRApply(SINH, (linear,))`` or ``IRApply(COSH, (linear,))``.
    - ``exponent`` is ``IRInteger(n)`` with ``n ≥ 2``.
    - The argument of sinh/cosh is a non-constant linear expression.

    Returns ``None`` for non-hyperbolic bases or non-integer exponents.
    """
    if not isinstance(base, IRApply):
        return None
    if base.head not in {SINH, COSH}:
        return None
    if not isinstance(exponent, IRInteger) or exponent.value < 2:
        return None
    n = exponent.value
    if len(base.args) != 1:
        return None
    lin = _try_linear(base.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == Fraction(0):
        return None
    if base.head == SINH:
        return sinh_power_integral(n, a_frac, b_frac, x)
    return cosh_power_integral(n, a_frac, b_frac, x)


def _try_recip_hyp_power(base: IRNode, exponent: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ sech^n(linear) dx``, ``∫ csch^n(linear) dx``,
    ``∫ coth^n(linear) dx``, ``∫ tanh^n(linear) dx``, or ``None``.

    Phase 16/17 — hyperbolic identity power reduction.

    Fires when:
    - ``base`` is ``IRApply(SECH/CSCH/COTH/TANH, (linear,))``.
    - ``exponent`` is ``IRInteger(n)`` with ``n ≥ 2``.
    - The argument of the function is a non-constant linear expression ``ax+b``.

    Returns ``None`` for other bases or non-integer exponents.
    Falls through for ``n < 2`` (bare n=1 cases handled in Phase 13/15 dispatch).

    Phase 17 adds TANH to the handled set alongside Phase 16's SECH/CSCH/COTH.
    """
    if not isinstance(base, IRApply):
        return None
    if base.head not in {SECH, CSCH, COTH, TANH}:
        return None
    if not isinstance(exponent, IRInteger) or exponent.value < 2:
        return None
    n = exponent.value
    if len(base.args) != 1:
        return None
    lin = _try_linear(base.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == Fraction(0):
        return None
    if base.head == SECH:
        return sech_power_integral(n, a_frac, b_frac, x)
    if base.head == CSCH:
        return csch_power_integral(n, a_frac, b_frac, x)
    if base.head == TANH:
        return tanh_power_integral(n, a_frac, b_frac, x)
    return coth_power_integral(n, a_frac, b_frac, x)


def _try_exp_hyp(exp_node: IRNode, hyp_node: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ exp(linear) · sinh/cosh(linear) dx`` or ``None``.

    Phase 14a — exp × hyperbolic double-IBP closed form.

    Uses the formulas:
        ∫ e^(ax+b)·sinh(cx+d) dx = e^(ax+b)·[a·sinh−c·cosh] / (a²−c²)
        ∫ e^(ax+b)·cosh(cx+d) dx = e^(ax+b)·[a·cosh−c·sinh] / (a²−c²)

    Falls through (returns ``None``) when ``a² = c²`` (degenerate case).
    """
    if not isinstance(exp_node, IRApply) or exp_node.head != EXP:
        return None
    if not isinstance(hyp_node, IRApply) or hyp_node.head not in {SINH, COSH}:
        return None
    lin_exp = _try_linear(exp_node.args[0], x)
    lin_hyp = _try_linear(hyp_node.args[0], x)
    if lin_exp is None or lin_hyp is None:
        return None
    a_frac, b_frac = lin_exp
    c_frac, d_frac = lin_hyp
    if a_frac == 0 or c_frac == 0:
        return None
    D = a_frac * a_frac - c_frac * c_frac
    if Fraction(0) == D:
        # a² = c² — expand into exponentials and integrate directly.
        return exp_hyp_degenerate(
            a_frac, b_frac, c_frac, d_frac,
            is_sinh=(hyp_node.head == SINH),
            x_sym=x,
        )
    if hyp_node.head == SINH:
        return exp_sinh_integral(a_frac, b_frac, c_frac, d_frac, x)
    return exp_cosh_integral(a_frac, b_frac, c_frac, d_frac, x)


def _try_sinh_cosh_product(f1: IRNode, f2: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ sinh^m · cosh^n dx`` via u-substitution when one exponent is 1.

    Phase 14b.

    Handles the patterns:
    - ``Sinh(linear) × Pow(Cosh(same linear), n)`` → ``cosh^(n+1)/(n+1)/a``
    - ``Pow(Sinh(linear), m) × Cosh(same linear)`` → ``sinh^(m+1)/(m+1)/a``
    - ``Sinh(linear) × Cosh(same linear)``           → ``sinh²/(2a)``

    Both arguments may be in either order since this function is called once
    (not mirrored like the exp handlers).  It tries all four orderings
    internally.

    Returns ``None`` if neither factor is a bare sinh or cosh (both are Pow).
    """
    from fractions import Fraction as _F

    def _extract_hyp_pow(
        node: IRNode,
    ) -> tuple[IRSymbol, int, _F, _F] | None:
        """Return ``(head, exp, a, b)`` if node is Sinh/Cosh(ax+b)^n, else None."""
        if not isinstance(node, IRApply):
            return None
        if node.head in {SINH, COSH}:
            lin = _try_linear(node.args[0], x) if node.args else None
            if lin is None:
                return None
            a_f, b_f = lin
            if a_f == _F(0):
                return None
            return (node.head, 1, a_f, b_f)  # type: ignore[return-value]
        if node.head == POW and len(node.args) == 2:
            base_, exp_ = node.args
            if not isinstance(base_, IRApply) or base_.head not in {SINH, COSH}:
                return None
            if not isinstance(exp_, IRInteger) or exp_.value < 1:
                return None
            lin = _try_linear(base_.args[0], x) if base_.args else None
            if lin is None:
                return None
            a_f, b_f = lin
            if a_f == _F(0):
                return None
            return (base_.head, exp_.value, a_f, b_f)  # type: ignore[return-value]
        return None

    h1 = _extract_hyp_pow(f1)
    h2 = _extract_hyp_pow(f2)
    if h1 is None or h2 is None:
        return None

    head1, n1, a1, b1 = h1
    head2, n2, a2, b2 = h2

    # Arguments must be the same linear expression.
    if a1 != a2 or b1 != b2:
        return None

    # Pattern: one is SINH and the other is COSH, at least one has power 1.
    if {head1, head2} != {SINH, COSH}:
        return None

    if head1 == SINH:
        m, n = n1, n2
    else:
        m, n = n2, n1

    return sinh_times_cosh_power(m, n, a1, b1, x)


def _try_trig_trig(f1: IRNode, f2: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ f1 · f2 dx`` via product-to-sum for trig × trig, or ``None``.

    Handles the three ordered cases:
    - sin(u)·sin(v) → [cos(u−v) − cos(u+v)] / 2
    - cos(u)·cos(v) → [cos(u−v) + cos(u+v)] / 2
    - sin(u)·cos(v) → [sin(u+v) + sin(u−v)] / 2

    The cos(u)·sin(v) case is handled by the swapped-order call
    ``_try_trig_trig(f2, f1, x)`` in the caller, which becomes sin·cos.
    """
    if not isinstance(f1, IRApply) or not isinstance(f2, IRApply):
        return None
    h1, h2 = f1.head, f2.head
    if h1 not in {SIN, COS} or h2 not in {SIN, COS}:
        return None
    # Skip cos·sin — the caller will retry as sin·cos.
    if h1 == COS and h2 == SIN:
        return None

    lin1 = _try_linear(f1.args[0], x)
    lin2 = _try_linear(f2.args[0], x)
    if lin1 is None or lin2 is None:
        return None
    a1, b1 = lin1
    a2, b2 = lin2
    if a1 == 0 or a2 == 0:
        return None  # constant — constant-factor rule handles it

    a_sum, b_sum = a1 + a2, b1 + b2
    a_diff, b_diff = a1 - a2, b1 - b2
    half = IRRational(1, 2)

    sum_arg = linear_to_ir(a_sum, b_sum, x)
    diff_arg = linear_to_ir(a_diff, b_diff, x)

    if h1 == SIN and h2 == SIN:
        # sin(u)·sin(v) = [cos(u−v) − cos(u+v)] / 2
        cos_diff = IRApply(COS, (diff_arg,))
        cos_sum = IRApply(COS, (sum_arg,))
        reduced = IRApply(MUL, (half, IRApply(SUB, (cos_diff, cos_sum))))
    elif h1 == COS and h2 == COS:
        # cos(u)·cos(v) = [cos(u−v) + cos(u+v)] / 2
        cos_diff = IRApply(COS, (diff_arg,))
        cos_sum = IRApply(COS, (sum_arg,))
        reduced = IRApply(MUL, (half, IRApply(ADD, (cos_diff, cos_sum))))
    else:
        # sin(u)·cos(v) = [sin(u+v) + sin(u−v)] / 2
        sin_sum = IRApply(SIN, (sum_arg,))
        sin_diff = IRApply(SIN, (diff_arg,))
        reduced = IRApply(MUL, (half, IRApply(ADD, (sin_sum, sin_diff))))

    return _integrate(reduced, x)


def _try_exp_trig(exp_node: IRNode, trig_node: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ exp(linear) · sin/cos(linear) dx`` or ``None``.

    Uses the double-IBP closed form:

        ∫ exp(ax+b)·sin(cx+d) dx = exp(ax+b)·[a·sin−c·cos] / (a²+c²)
        ∫ exp(ax+b)·cos(cx+d) dx = exp(ax+b)·[a·cos+c·sin] / (a²+c²)
    """
    if not isinstance(exp_node, IRApply) or not isinstance(trig_node, IRApply):
        return None
    if exp_node.head != EXP:
        return None
    if trig_node.head not in {SIN, COS}:
        return None

    lin_exp = _try_linear(exp_node.args[0], x)
    lin_trig = _try_linear(trig_node.args[0], x)
    if lin_exp is None or lin_trig is None:
        return None
    a_frac, b_frac = lin_exp
    c_frac, d_frac = lin_trig
    if a_frac == 0 or c_frac == 0:
        return None  # constant exp or trig — other rules handle those

    if trig_node.head == SIN:
        return exp_sin_integral(a_frac, b_frac, c_frac, d_frac, x)
    return exp_cos_integral(a_frac, b_frac, c_frac, d_frac, x)


# ---------------------------------------------------------------------------
# Phase 5 helpers
# ---------------------------------------------------------------------------


def _tan_integral(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Return the IR for ``∫ tan(ax+b) dx = −log(cos(ax+b)) / a``.

    Precondition: ``a ≠ 0``.
    """
    cos_ir = IRApply(COS, (linear_to_ir(a, b, x),))
    log_cos = IRApply(LOG, (cos_ir,))
    if a == Fraction(1):
        return IRApply(NEG, (log_cos,))
    return IRApply(NEG, (IRApply(DIV, (log_cos, _frac_ir(a))),))


def _tanh_integral(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Return the IR for ``∫ tanh(ax+b) dx = log(cosh(ax+b)) / a``.

    Precondition: ``a ≠ 0``.
    """
    arg_ir = linear_to_ir(a, b, x)
    cosh_ir = IRApply(COSH, (arg_ir,))
    log_cosh = IRApply(LOG, (cosh_ir,))
    if a == Fraction(1):
        return log_cosh
    return IRApply(DIV, (log_cosh, _frac_ir(a)))


def _atanh_integral(
    a: Fraction, b: Fraction, arg_ir: IRNode, x: IRSymbol
) -> IRNode:
    """Return the IR for ``∫ atanh(ax+b) dx``.

    Formula: (ax+b)/a · atanh(ax+b) + (1/(2a)) · log(1−(ax+b)²)

    Precondition: ``a ≠ 0``.  ``arg_ir`` is the already-constructed ``ax+b``
    IR node (avoids re-building it).
    """
    a_ir = _frac_ir(a)
    coef_ir = IRApply(DIV, (arg_ir, a_ir))
    atanh_ir = IRApply(ATANH, (arg_ir,))
    main_term = IRApply(MUL, (coef_ir, atanh_ir))

    # (1/(2a)) · log(1 − (ax+b)²)
    inner = IRApply(SUB, (ONE, IRApply(POW, (arg_ir, TWO))))
    log_part = IRApply(
        DIV,
        (IRApply(LOG, (inner,)), IRApply(MUL, (TWO, a_ir))),
    )
    return IRApply(ADD, (main_term, log_part))


def _coth_integral(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Return the IR for ``∫ coth(ax+b) dx = log(sinh(ax+b)) / a``.

    Derivation:  d/dx log(sinh(ax+b)) = a·cosh(ax+b)/sinh(ax+b) = a·coth(ax+b).

    The antiderivative is defined for ``sinh(ax+b) > 0``, i.e. ``ax+b > 0``.
    As is conventional for a CAS, we omit the absolute value and return the
    branch that covers the positive half-line.

    Precondition: ``a ≠ 0``.
    """
    arg_ir = linear_to_ir(a, b, x)
    log_sinh = IRApply(LOG, (IRApply(SINH, (arg_ir,)),))
    if a == Fraction(1):
        return log_sinh
    return IRApply(DIV, (log_sinh, _frac_ir(a)))


def _sech_integral(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Return the IR for ``∫ sech(ax+b) dx = atan(sinh(ax+b)) / a``.

    Derivation:
      d/dx atan(sinh(ax+b))
        = a·cosh(ax+b) / (1 + sinh²(ax+b))
        = a·cosh(ax+b) / cosh²(ax+b)      [since 1 + sinh² = cosh²]
        = a / cosh(ax+b)
        = a·sech(ax+b).

    Precondition: ``a ≠ 0``.
    """
    arg_ir = linear_to_ir(a, b, x)
    atan_sinh = IRApply(ATAN, (IRApply(SINH, (arg_ir,)),))
    if a == Fraction(1):
        return atan_sinh
    return IRApply(DIV, (atan_sinh, _frac_ir(a)))


def _csch_integral(a: Fraction, b: Fraction, x: IRSymbol) -> IRNode:
    """Return the IR for ``∫ csch(ax+b) dx = log(tanh((ax+b)/2)) / a``.

    Derivation (for x such that ax+b > 0 so tanh((ax+b)/2) > 0):

      Let u = ax+b, h = u/2.  Then:

        d/dx [log(tanh(h))]
          = (1/tanh(h)) · sech²(h) · (a/2)
          = (a/2) · cosh(h) / (sinh(h) · cosh²(h))
          = (a/2) · 1 / (sinh(h) · cosh(h))
          = (a/2) · 2 / sinh(2h)            [since sinh(2h) = 2·sinh(h)·cosh(h)]
          = a / sinh(u)
          = a · csch(u).

    The half-argument ``(ax+b)/2 = (a/2)·x + b/2`` is a linear expression
    in ``x`` with ``Fraction`` coefficients, so ``linear_to_ir`` handles it
    directly.

    Note: ``−atanh(cosh(ax+b))`` is algebraically equivalent but requires
    ``|cosh(ax+b)| < 1``, which is never true for real ``x``.  The tanh/2
    form is real-valued and numerically safe for ``ax+b ≠ 0``.

    Precondition: ``a ≠ 0``.
    """
    half_a = a / Fraction(2)
    half_b = b / Fraction(2)
    half_arg_ir = linear_to_ir(half_a, half_b, x)
    log_tanh = IRApply(LOG, (IRApply(TANH, (half_arg_ir,)),))
    if a == Fraction(1):
        return log_tanh
    return IRApply(DIV, (log_tanh, _frac_ir(a)))


def _try_trig_power(
    base: IRNode, exponent: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ base^exponent dx`` for integer trig powers, or ``None``.

    Handles:
    - ``POW(SIN(linear), n)`` for integer n ≥ 2 (Phase 5b, sin reduction)
    - ``POW(COS(linear), n)`` for integer n ≥ 2 (Phase 5b, cos reduction)
    - ``POW(TAN(linear), n)`` for integer n ≥ 2 (Phase 5c, tan reduction)

    Returns ``None`` for non-integer, negative, or zero exponents.
    """
    if not isinstance(exponent, IRInteger):
        return None
    n = exponent.value
    if n < 2:
        return None
    if not isinstance(base, IRApply):
        return None
    head = base.head
    if head not in {SIN, COS, TAN}:
        return None
    if len(base.args) != 1:
        return None
    lin = _try_linear(base.args[0], x)
    if lin is None:
        return None
    a_frac, b_frac = lin
    if a_frac == Fraction(0):
        return None  # constant trig — constant-factor rule handles it

    if head == SIN:
        return _sin_power(n, a_frac, b_frac, x)
    if head == COS:
        return _cos_power(n, a_frac, b_frac, x)
    return _tan_power(n, a_frac, b_frac, x)


def _sin_power(n: int, a: Fraction, b: Fraction, x: IRSymbol) -> IRNode | None:
    """Return ``∫ sinⁿ(ax+b) dx`` via the reduction formula.

    Reduction: ``−sinⁿ⁻¹(ax+b)·cos(ax+b)/(n·a) + (n−1)/n · ∫sinⁿ⁻²(ax+b) dx``.
    Terminates when the recursive ``_integrate`` handles the n−2 case
    (n=1 → Phase 3b, n=0 → constant).
    """
    arg = linear_to_ir(a, b, x)
    sin_ir = IRApply(SIN, (arg,))
    cos_ir = IRApply(COS, (arg,))
    na = _frac_ir(Fraction(n) * a)
    # −sinⁿ⁻¹(ax+b)·cos(ax+b) / (n·a)
    sin_nm1 = IRApply(POW, (sin_ir, IRInteger(n - 1)))
    term = IRApply(NEG, (IRApply(DIV, (IRApply(MUL, (sin_nm1, cos_ir)), na)),))
    # (n−1)/n · ∫sinⁿ⁻²(ax+b) dx
    inner_f = IRApply(POW, (sin_ir, IRInteger(n - 2)))
    inner_int = _integrate(inner_f, x)
    if inner_int is None:
        return None
    coef = _frac_ir(Fraction(n - 1, n))
    tail = IRApply(MUL, (coef, inner_int))
    return IRApply(ADD, (term, tail))


def _cos_power(n: int, a: Fraction, b: Fraction, x: IRSymbol) -> IRNode | None:
    """Return ``∫ cosⁿ(ax+b) dx`` via the reduction formula.

    Reduction: ``cosⁿ⁻¹(ax+b)·sin(ax+b)/(n·a) + (n−1)/n · ∫cosⁿ⁻²(ax+b) dx``.
    """
    arg = linear_to_ir(a, b, x)
    cos_ir = IRApply(COS, (arg,))
    sin_ir = IRApply(SIN, (arg,))
    na = _frac_ir(Fraction(n) * a)
    # cosⁿ⁻¹(ax+b)·sin(ax+b) / (n·a)
    cos_nm1 = IRApply(POW, (cos_ir, IRInteger(n - 1)))
    term = IRApply(DIV, (IRApply(MUL, (cos_nm1, sin_ir)), na))
    # (n−1)/n · ∫cosⁿ⁻²(ax+b) dx
    inner_f = IRApply(POW, (cos_ir, IRInteger(n - 2)))
    inner_int = _integrate(inner_f, x)
    if inner_int is None:
        return None
    coef = _frac_ir(Fraction(n - 1, n))
    tail = IRApply(MUL, (coef, inner_int))
    return IRApply(ADD, (term, tail))


def _tan_power(n: int, a: Fraction, b: Fraction, x: IRSymbol) -> IRNode | None:
    """Return ``∫ tanⁿ(ax+b) dx`` via the Pythagorean reduction formula.

    Reduction: ``tanⁿ⁻¹(ax+b) / ((n−1)·a) − ∫tanⁿ⁻²(ax+b) dx``.
    Terminates at n=1 (Phase 5a) or n=0 (constant).
    """
    arg = linear_to_ir(a, b, x)
    tan_ir = IRApply(TAN, (arg,))
    n1a = _frac_ir(Fraction(n - 1) * a)
    # tanⁿ⁻¹(ax+b) / ((n−1)·a)
    tan_nm1 = IRApply(POW, (tan_ir, IRInteger(n - 1)))
    term = IRApply(DIV, (tan_nm1, n1a))
    # ∫tanⁿ⁻²(ax+b) dx  (recursive)
    inner_f = IRApply(POW, (tan_ir, IRInteger(n - 2)))
    inner_int = _integrate(inner_f, x)
    if inner_int is None:
        return None
    return IRApply(SUB, (term, inner_int))


# ---------------------------------------------------------------------------
# Phase 6 helpers — sinⁿ·cosᵐ mixed trig powers
# ---------------------------------------------------------------------------


def _extract_trig_power(
    node: IRNode, x: IRSymbol
) -> tuple[IRSymbol, int, Fraction, Fraction] | None:
    """Return ``(head, exponent, a, b)`` if ``node`` is ``SIN/COS(linear)^n``.

    Accepts bare ``SIN/COS(linear)`` (treated as exponent 1) and
    ``POW(SIN/COS(linear), n)`` for integer n ≥ 1.  Returns ``None``
    for anything else.
    """
    if not isinstance(node, IRApply):
        return None
    if node.head in {SIN, COS} and len(node.args) == 1:
        lin = _try_linear(node.args[0], x)
        if lin is not None:
            return (node.head, 1, lin[0], lin[1])
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        if (
            isinstance(base, IRApply)
            and base.head in {SIN, COS}
            and len(base.args) == 1
            and isinstance(exp, IRInteger)
            and exp.value >= 1
        ):
            lin = _try_linear(base.args[0], x)
            if lin is not None:
                return (base.head, exp.value, lin[0], lin[1])
    return None


def _try_sin_cos_power(
    fa: IRNode, fb: IRNode, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ sinⁿ cosᵐ dx`` for matched trig-power pairs, or ``None``.

    Guards:
    - One factor is SIN-based, the other is COS-based.
    - Both use the same linear argument ``ax + b`` (a ≠ 0, coefficients ∈ Q).
    - Both exponents ≥ 1, and at least one ≥ 2 (the n=m=1 same-arg case is
      already handled by ``_try_trig_trig`` via the product-to-sum identity).

    Dispatches to:
    - Case A (n odd): closed-form binomial sum over cos powers.
    - Case B (m odd, n even): closed-form binomial sum over sin powers.
    - Case C (both even): IBP reduction formula reducing n by 2 each step.
    """
    ta = _extract_trig_power(fa, x)
    tb = _extract_trig_power(fb, x)
    if ta is None or tb is None:
        return None
    head_a, na, aa, ba = ta
    head_b, nb, ab, bb = tb
    # Must have one SIN factor and one COS factor.
    if head_a == head_b:
        return None
    # Same linear argument required.
    if aa != ab or ba != bb:
        return None
    # Linear coefficient must be non-zero.
    if aa == Fraction(0):
        return None
    # Normalise: n = sin exponent, m = cos exponent.
    if head_a == SIN:
        n, m = na, nb
    else:
        n, m = nb, na
    a_frac, b_frac = aa, ba
    # Require at least one exponent ≥ 2 (n=m=1 same-arg case → _try_trig_trig).
    if max(n, m) < 2:
        return None

    if n % 2 == 1:
        return _sin_cos_odd_sin(n, m, a_frac, b_frac, x)
    if m % 2 == 1:
        return _sin_cos_odd_cos(n, m, a_frac, b_frac, x)
    return _sin_cos_even(n, m, a_frac, b_frac, x)


def _sin_cos_odd_sin(
    n: int, m: int, a: Fraction, b: Fraction, x: IRSymbol
) -> IRNode:
    """Return ``∫ sinⁿ cosᵐ (ax+b) dx`` for odd n via cosine substitution.

    u = cos(ax+b), du = -a·sin(ax+b)dx.  sinⁿ⁻¹ = (1−u²)^k (k=(n−1)/2).
    Binomial expansion gives a closed-form sum over cos powers:

      -(1/a) · Σ_{j=0}^{k} C(k,j)·(−1)^j / (m+2j+1) · cos^{m+2j+1}(ax+b)
    """
    import math

    k = (n - 1) // 2
    arg = linear_to_ir(a, b, x)
    cos_ir = IRApply(COS, (arg,))
    # Build left-associative ADD chain of terms.
    terms: list[IRNode] = []
    for j in range(k + 1):
        coef = Fraction(math.comb(k, j) * ((-1) ** j), m + 2 * j + 1)
        pow_ir = IRApply(POW, (cos_ir, IRInteger(m + 2 * j + 1)))
        term = IRApply(MUL, (_frac_ir(coef), pow_ir))
        terms.append(term)
    # Sum all terms.
    total: IRNode = terms[0]
    for t in terms[1:]:
        total = IRApply(ADD, (total, t))
    # Multiply by -(1/a).
    scaled = IRApply(NEG, (IRApply(DIV, (total, _frac_ir(a))),))
    return scaled


def _sin_cos_odd_cos(
    n: int, m: int, a: Fraction, b: Fraction, x: IRSymbol
) -> IRNode:
    """Return ``∫ sinⁿ cosᵐ (ax+b) dx`` for even n, odd m via sine substitution.

    u = sin(ax+b), du = a·cos(ax+b)dx.  cosᵐ⁻¹ = (1−u²)^k (k=(m−1)/2).
    Binomial expansion gives a closed-form sum over sin powers:

      (1/a) · Σ_{j=0}^{k} C(k,j)·(−1)^j / (n+2j+1) · sin^{n+2j+1}(ax+b)
    """
    import math

    k = (m - 1) // 2
    arg = linear_to_ir(a, b, x)
    sin_ir = IRApply(SIN, (arg,))
    terms: list[IRNode] = []
    for j in range(k + 1):
        coef = Fraction(math.comb(k, j) * ((-1) ** j), n + 2 * j + 1)
        pow_ir = IRApply(POW, (sin_ir, IRInteger(n + 2 * j + 1)))
        term = IRApply(MUL, (_frac_ir(coef), pow_ir))
        terms.append(term)
    total: IRNode = terms[0]
    for t in terms[1:]:
        total = IRApply(ADD, (total, t))
    return IRApply(DIV, (total, _frac_ir(a)))


def _sin_cos_even(
    n: int, m: int, a: Fraction, b: Fraction, x: IRSymbol
) -> IRNode | None:
    """Return ``∫ sinⁿ cosᵐ (ax+b) dx`` for even n, m via IBP reduction.

    Reduction formula (reduces n by 2):

      ∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹ cosᵐ⁺¹ / ((n+m)·a) + (n−1)/(n+m) · ∫ sinⁿ⁻² cosᵐ dx

    Terminates at n=0: delegates to Phase 5b (``∫ cosᵐ dx``).
    """
    arg = linear_to_ir(a, b, x)
    sin_ir = IRApply(SIN, (arg,))
    cos_ir = IRApply(COS, (arg,))
    nm = n + m
    nma = _frac_ir(Fraction(nm) * a)
    # -sinⁿ⁻¹ cosᵐ⁺¹ / ((n+m)·a)
    sin_nm1 = IRApply(POW, (sin_ir, IRInteger(n - 1)))
    cos_mp1 = IRApply(POW, (cos_ir, IRInteger(m + 1)))
    term = IRApply(NEG, (IRApply(DIV, (IRApply(MUL, (sin_nm1, cos_mp1)), nma)),))
    # (n-1)/(n+m) · ∫ sinⁿ⁻² cosᵐ dx
    coef = _frac_ir(Fraction(n - 1, nm))
    n2 = n - 2
    if n2 == 0:
        inner_f: IRNode = IRApply(POW, (cos_ir, IRInteger(m)))
    else:
        sin_n2 = IRApply(POW, (sin_ir, IRInteger(n2)))
        cos_m = IRApply(POW, (cos_ir, IRInteger(m)))
        inner_f = IRApply(MUL, (sin_n2, cos_m))
    inner_int = _integrate(inner_f, x)
    if inner_int is None:
        return None
    tail = IRApply(MUL, (coef, inner_int))
    return IRApply(ADD, (term, tail))


# ---------------------------------------------------------------------------
# Phase 7 helpers — u-substitution (chain-rule reversal)
# ---------------------------------------------------------------------------


def _poly_deriv(p: tuple) -> tuple:
    """Return the derivative of polynomial coefficient tuple ``p``.

    Ascending-coefficient convention: index ``i`` = coeff of ``x^i``.
    A constant polynomial (degree 0) differentiates to zero → ``(Fraction(0),)``.
    """
    if len(p) <= 1:
        return (Fraction(0),)
    return tuple(Fraction(i) * Fraction(p[i]) for i in range(1, len(p)))


def _poly_mul(p: tuple, q: tuple) -> tuple:
    """Multiply two polynomial coefficient tuples."""
    if not p or not q:
        return (Fraction(0),)
    result = [Fraction(0)] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            result[i + j] += Fraction(a) * Fraction(b)
    return tuple(result)


def _diff_ir(g: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``dg/dx`` as an IR node, or ``None`` if the form is unknown.

    Handles: constants, bare ``x``, polynomials in ``x`` (via ``to_rational``),
    and single-argument functions (SIN, COS, EXP, LOG, SQRT) of recursively
    differentiable arguments via the chain rule.  Also handles integer-exponent
    ``POW`` via the power rule + chain rule.
    """
    # Constant free of x → 0.
    if not _depends_on(g, x):
        return IRInteger(0)
    # Bare variable → 1.
    if g == x:
        return IRInteger(1)
    if not isinstance(g, IRApply):
        return None

    head = g.head

    # Polynomial (and rational) functions of x.
    r = to_rational(g, x)
    if r is not None:
        num, den = r
        from polynomial import normalize as _pnorm
        den_n = _pnorm(den)
        # Only handle pure-polynomial g here (denominator = 1).
        if len(den_n) <= 1:
            dnum = _poly_deriv(num)
            result_coeffs = _pnorm(dnum)
            if not result_coeffs:
                return IRInteger(0)
            return from_polynomial(result_coeffs, x)
        # Rational function derivatives deferred (quotient rule not needed yet).
        return None

    # Additive / negation rules — linearity of differentiation.
    if head == NEG:
        # d/dx(−f) = −f'
        da = _diff_ir(g.args[0], x)
        if da is None:
            return None
        if isinstance(da, IRInteger) and da.value == 0:
            return IRInteger(0)
        return IRApply(NEG, (da,))

    if head == ADD:
        # d/dx(f+g) = f' + g'
        da = _diff_ir(g.args[0], x)
        db = _diff_ir(g.args[1], x)
        if da is None or db is None:
            return None
        zero = IRInteger(0)
        if da == zero:
            return db
        if db == zero:
            return da
        return IRApply(ADD, (da, db))

    if head == SUB:
        # d/dx(f−g) = f' − g'
        da = _diff_ir(g.args[0], x)
        db = _diff_ir(g.args[1], x)
        if da is None or db is None:
            return None
        if isinstance(db, IRInteger) and db.value == 0:
            return da
        return IRApply(SUB, (da, db))

    # Single-argument functions — apply chain rule.
    if len(g.args) == 1:
        arg = g.args[0]
        darg = _diff_ir(arg, x)
        if darg is None:
            return None
        darg_is_one = isinstance(darg, IRInteger) and darg.value == 1

        if head == SIN:
            # d/dx sin(u) = cos(u)·u'
            cos_u = IRApply(COS, (arg,))
            return cos_u if darg_is_one else IRApply(MUL, (cos_u, darg))

        if head == COS:
            # d/dx cos(u) = −sin(u)·u'
            neg_sin = IRApply(NEG, (IRApply(SIN, (arg,)),))
            return neg_sin if darg_is_one else IRApply(MUL, (neg_sin, darg))

        if head == ASIN:
            # d/dx asin(u) = u'/√(1−u²)
            denom = IRApply(
                SQRT, (IRApply(SUB, (ONE, IRApply(POW, (arg, TWO)))),)
            )
            if darg_is_one:
                return IRApply(DIV, (ONE, denom))
            return IRApply(DIV, (darg, denom))

        if head == ACOS:
            # d/dx acos(u) = −u'/√(1−u²)
            denom = IRApply(
                SQRT, (IRApply(SUB, (ONE, IRApply(POW, (arg, TWO)))),)
            )
            neg_inv = IRApply(NEG, (IRApply(DIV, (ONE, denom)),))
            if darg_is_one:
                return neg_inv
            return IRApply(MUL, (IRApply(NEG, (darg,)), IRApply(DIV, (ONE, denom))))

        if head == SINH:
            # d/dx sinh(u) = cosh(u)·u'
            cosh_u = IRApply(COSH, (arg,))
            return cosh_u if darg_is_one else IRApply(MUL, (cosh_u, darg))

        if head == COSH:
            # d/dx cosh(u) = sinh(u)·u'
            sinh_u = IRApply(SINH, (arg,))
            return sinh_u if darg_is_one else IRApply(MUL, (sinh_u, darg))

        if head == TANH:
            # d/dx tanh(u) = u'/cosh²(u)
            cosh_u = IRApply(COSH, (arg,))
            denom = IRApply(POW, (cosh_u, TWO))
            if darg_is_one:
                return IRApply(DIV, (ONE, denom))
            return IRApply(DIV, (darg, denom))

        if head == ASINH:
            # d/dx asinh(u) = u'/√(u²+1)
            denom = IRApply(
                SQRT, (IRApply(ADD, (IRApply(POW, (arg, TWO)), ONE)),)
            )
            if darg_is_one:
                return IRApply(DIV, (ONE, denom))
            return IRApply(DIV, (darg, denom))

        if head == ACOSH:
            # d/dx acosh(u) = u'/√(u²−1)
            denom = IRApply(
                SQRT, (IRApply(SUB, (IRApply(POW, (arg, TWO)), ONE)),)
            )
            if darg_is_one:
                return IRApply(DIV, (ONE, denom))
            return IRApply(DIV, (darg, denom))

        if head == ATANH:
            # d/dx atanh(u) = u'/(1−u²)
            denom = IRApply(SUB, (ONE, IRApply(POW, (arg, TWO))))
            if darg_is_one:
                return IRApply(DIV, (ONE, denom))
            return IRApply(DIV, (darg, denom))

        if head == COTH:
            # d/dx coth(u) = −u'/sinh²(u)
            # We express the derivative in terms of sinh (not coth/csch) so
            # the simplifier and evaluator don't need to recurse through coth.
            sinh_u = IRApply(SINH, (arg,))
            denom = IRApply(POW, (sinh_u, TWO))
            numer = ONE if darg_is_one else darg
            return IRApply(NEG, (IRApply(DIV, (numer, denom)),))

        if head == SECH:
            # d/dx sech(u) = −u'·sinh(u)/cosh²(u)
            sinh_u = IRApply(SINH, (arg,))
            cosh_u = IRApply(COSH, (arg,))
            numer = sinh_u if darg_is_one else IRApply(MUL, (darg, sinh_u))
            denom = IRApply(POW, (cosh_u, TWO))
            return IRApply(NEG, (IRApply(DIV, (numer, denom)),))

        if head == CSCH:
            # d/dx csch(u) = −u'·cosh(u)/sinh²(u)
            cosh_u = IRApply(COSH, (arg,))
            sinh_u = IRApply(SINH, (arg,))
            numer = cosh_u if darg_is_one else IRApply(MUL, (darg, cosh_u))
            denom = IRApply(POW, (sinh_u, TWO))
            return IRApply(NEG, (IRApply(DIV, (numer, denom)),))

        if head == EXP:
            # d/dx exp(u) = exp(u)·u'
            return g if darg_is_one else IRApply(MUL, (g, darg))

        if head == LOG:
            # d/dx log(u) = u'/u
            if darg_is_one:
                return IRApply(DIV, (IRInteger(1), arg))
            return IRApply(DIV, (darg, arg))

        if head == SQRT:
            # d/dx sqrt(u) = u'/(2·sqrt(u))
            denom = IRApply(MUL, (TWO, g))
            if darg_is_one:
                return IRApply(DIV, (IRInteger(1), denom))
            return IRApply(DIV, (darg, denom))

    # Integer-exponent POW — power rule + chain rule.
    if head == POW and len(g.args) == 2:
        base, exp_node = g.args
        if isinstance(exp_node, IRInteger):
            n = exp_node.value
            dbase = _diff_ir(base, x)
            if dbase is None:
                return None
            dbase_is_one = isinstance(dbase, IRInteger) and dbase.value == 1
            if n == 1:
                return dbase
            # n·base^(n−1)·base'
            coef = IRInteger(n)
            pow_nm1 = (
                base if n == 2 else IRApply(POW, (base, IRInteger(n - 1)))
            )
            inner = (
                pow_nm1 if dbase_is_one else IRApply(MUL, (pow_nm1, dbase))
            )
            return IRApply(MUL, (coef, inner))

    return None


def _scalar_split(node: IRNode) -> tuple[Fraction, IRNode]:
    """Return ``(c, expr)`` such that ``node = c · expr`` with ``c`` rational.

    For nodes of the form ``MUL(rational, expr)`` or ``MUL(expr, rational)``
    this extracts the rational factor.  For a bare rational literal it returns
    ``(literal, IRInteger(1))``.  Anything else returns ``(Fraction(1), node)``.
    """
    if isinstance(node, IRApply) and node.head == MUL:
        a, b = node.args
        ca = _node_to_frac(a)
        if ca is not None:
            return (ca, b)
        cb = _node_to_frac(b)
        if cb is not None:
            return (cb, a)
    cn = _node_to_frac(node)
    if cn is not None:
        return (cn, ONE)
    return (Fraction(1), node)


def _ratio_const(
    f: IRNode, g: IRNode, x: IRSymbol
) -> Fraction | None:
    """Return ``c`` if ``f = c · g`` for some rational ``c``, else ``None``.

    Checks structural equality, scalar-split (handles MUL commutativity),
    NEG-matching, and a polynomial ratio check for rational functions of ``x``.
    """
    # Exact structural equality.
    if f == g:
        return Fraction(1)

    # Scalar-split: normalise both to (c, expr) and compare the expr parts.
    # This handles MUL commutativity and all scalar-multiple forms in one step.
    f_sc, f_ex = _scalar_split(f)
    g_sc, g_ex = _scalar_split(g)
    if f_ex == g_ex and g_sc != 0:
        return f_sc / g_sc

    # NEG(expr) = −1·expr — scalar_split doesn't unpack NEG.
    if isinstance(g, IRApply) and g.head == NEG and g.args[0] == f:
        return Fraction(-1)
    if isinstance(f, IRApply) and f.head == NEG and f.args[0] == g:
        return Fraction(-1)
    # f = c · NEG(expr) vs g = NEG(expr).
    if (
        f_sc != 1
        and isinstance(f_ex, IRApply)
        and f_ex.head == NEG
        and f_ex.args[0] == g
    ):
        return -f_sc
    # g = c · NEG(expr) vs f = NEG(expr).
    if (
        g_sc != 1
        and isinstance(g_ex, IRApply)
        and g_ex.head == NEG
        and g_ex.args[0] == f
        and g_sc != 0
    ):
        return Fraction(-1) / g_sc

    # Polynomial ratio — both must be rational functions of x.
    r_f = to_rational(f, x)
    r_g = to_rational(g, x)
    if r_f is not None and r_g is not None:
        from polynomial import normalize as _pnorm
        f_num, f_den = r_f
        g_num, g_den = r_g
        numer_n = _pnorm(_poly_mul(f_num, g_den))
        denom_n = _pnorm(_poly_mul(f_den, g_num))
        if not numer_n or not denom_n:
            return None
        if len(numer_n) != len(denom_n):
            return None
        ratio: Fraction | None = None
        for nc, dc in zip(numer_n, denom_n, strict=True):
            nc, dc = Fraction(nc), Fraction(dc)
            if dc == 0:
                if nc != 0:
                    return None
            else:
                r = nc / dc
                if ratio is None:
                    ratio = r
                elif ratio != r:
                    return None
        return ratio

    return None


def _subst(node: IRNode, old: IRNode, new: IRNode) -> IRNode:
    """Replace every occurrence of ``old`` with ``new`` in ``node``."""
    if node == old:
        return new
    if isinstance(node, IRApply):
        new_args = tuple(_subst(a, old, new) for a in node.args)
        if new_args == node.args:
            return node
        return IRApply(node.head, new_args)
    return node


def _try_u_sub_one(
    outer: IRNode, gp_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Try u-sub treating ``outer`` as ``f(g(x))`` and ``gp_candidate`` as ``c·g'(x)``.

    Returns an antiderivative IR node or ``None``.
    """
    if not isinstance(outer, IRApply):
        return None
    # Only handle single-argument outer functions (SIN, COS, EXP, LOG, TAN, SQRT).
    if len(outer.args) != 1:
        return None
    g = outer.args[0]
    # Skip g = x (handled by Phase 1 rules directly).
    if g == x:
        return None
    # Skip linear g — Phases 3–5 already cover f(ax+b) for all supported f.
    lin = _try_linear(g, x)
    if lin is not None and lin[0] != Fraction(0):
        return None

    gprime = _diff_ir(g, x)
    if gprime is None:
        return None

    c = _ratio_const(gp_candidate, gprime, x)
    if c is None or c == Fraction(0):
        return None

    # Substitute g(x) → u, integrate, substitute u → g(x).
    u = IRSymbol("__u__")
    F_u = _subst(outer, g, u)
    G_u = _integrate(F_u, u)
    if G_u is None:
        return None
    G_gx = _subst(G_u, u, g)

    if c == Fraction(1):
        return G_gx
    return IRApply(MUL, (_frac_ir(c), G_gx))


def _try_u_sub(fa: IRNode, fb: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ fa·fb dx`` via u-substitution, or ``None``.

    Tries both orderings: (fa as outer, fb as g') and (fb as outer, fa as g').
    """
    result = _try_u_sub_one(fa, fb, x)
    if result is not None:
        return result
    return _try_u_sub_one(fb, fa, x)


# ---------------------------------------------------------------------------
# Phase 8 helpers — power-of-composite u-substitution
# ---------------------------------------------------------------------------


def _try_u_sub_pow_one(
    pow_node: IRNode, gp_candidate: IRNode, x: IRSymbol
) -> IRNode | None:
    """Try u-sub for ``POW(base, n) · c·g'(x)``, returning the antiderivative or None.

    Two cases:

    Case A — ``base = f(g(x))`` where ``f`` is a single-arg function:
      Substitute u = g, integrate ``POW(f(u), n)`` (delegated to Phase 5 for
      SIN/COS/TAN powers), back-substitute.

    Case B — ``base = g(x)`` (polynomial, sums, etc.):
      Substitute u = base, integrate ``u^n`` via Phase 1 power rule,
      back-substitute.
    """
    if not isinstance(pow_node, IRApply) or pow_node.head != POW:
        return None
    base, exp_node = pow_node.args
    if _depends_on(exp_node, x) or not _depends_on(base, x):
        return None

    u = IRSymbol("__u__")

    # Case A: base is a single-arg function of some inner g(x).
    if isinstance(base, IRApply) and len(base.args) == 1:
        g = base.args[0]
        # Skip f(x)^n — Phase 5 _try_trig_power handles it via the POW branch.
        if g == x:
            return None
        # Skip f(ax+b)^n — Phase 5 handles linear-arg trig powers.
        lin = _try_linear(g, x)
        if lin is not None and lin[0] != Fraction(0):
            return None
        gprime = _diff_ir(g, x)
        if gprime is None:
            return None
        c = _ratio_const(gp_candidate, gprime, x)
        if c is None or c == Fraction(0):
            return None
        # Replace g → u inside the base function, keeping the outer POW.
        base_u = _subst(base, g, u)
        G_u = _integrate(IRApply(POW, (base_u, exp_node)), u)
        if G_u is None:
            return None
        G_gx = _subst(G_u, u, g)
        if c == Fraction(1):
            return G_gx
        return IRApply(MUL, (_frac_ir(c), G_gx))

    # Case B: base is a general expression g(x) — treat the whole base as g.
    # Skip bare x (Phase 1) and linear base (Phase 8 bonus in POW branch).
    if base == x:
        return None
    lin = _try_linear(base, x)
    if lin is not None and lin[0] != Fraction(0):
        return None
    gprime = _diff_ir(base, x)
    if gprime is None:
        return None
    c = _ratio_const(gp_candidate, gprime, x)
    if c is None or c == Fraction(0):
        return None
    # ∫ u^n du — Phase 1 power rule applies since u is now bare.
    G_u = _integrate(IRApply(POW, (u, exp_node)), u)
    if G_u is None:
        return None
    G_gx = _subst(G_u, u, base)
    if c == Fraction(1):
        return G_gx
    return IRApply(MUL, (_frac_ir(c), G_gx))


def _try_u_sub_pow(fa: IRNode, fb: IRNode, x: IRSymbol) -> IRNode | None:
    """Return ``∫ fa·fb dx`` via power-of-composite u-sub, or ``None``.

    Tries both orderings: (fa as pow_node, fb as g') and (fb as pow_node, fa as g').
    """
    result = _try_u_sub_pow_one(fa, fb, x)
    if result is not None:
        return result
    return _try_u_sub_pow_one(fb, fa, x)


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


# ---------------------------------------------------------------------------
# Phase 9 helpers — multi-quadratic partial-fraction integration
# ---------------------------------------------------------------------------


def _int_divisors(n: int) -> list[int]:
    """Return the positive integer divisors of ``abs(n)``."""
    n = abs(n)
    if n == 0:
        return []
    divs = []
    i = 1
    while i * i <= n:
        if n % i == 0:
            divs.append(i)
            if i != n // i:
                divs.append(n // i)
        i += 1
    return divs


def _rational_divisors(p: Fraction) -> list[Fraction]:
    """Return ±(divisors of numerator) / (divisors of denominator).

    Gives the finite candidate set for the biquadratic-factoring search
    — analogous to the Rational Roots Theorem's candidate pool.  Zero is
    excluded since ``b = 0`` would make ``d = p₀/b`` undefined.
    """
    num_divs = _int_divisors(p.numerator)
    den_divs = _int_divisors(p.denominator)
    if not num_divs or not den_divs:
        return []
    seen: set[Fraction] = set()
    result: list[Fraction] = []
    for u in num_divs:
        for v in den_divs:
            for sign in (1, -1):
                cand = Fraction(sign * u, v)
                if cand not in seen:
                    seen.add(cand)
                    result.append(cand)
    return result


def _factor_biquadratic(
    E: tuple,
) -> tuple[tuple[Fraction, ...], tuple[Fraction, ...]] | None:
    """Try to write degree-4 monic squarefree ``E`` as ``Q₁·Q₂``.

    Both ``Q₁`` and ``Q₂`` must be irreducible monic quadratics over Q.
    ``E`` is passed as an ascending tuple of ``Fraction`` coefficients with
    the leading coefficient equal to 1.

    Returns ``(Q₁, Q₂)`` as ascending coefficient tuples, or ``None`` when
    no rational biquadratic factorization exists.

    Algorithm — for ``E = x⁴+p₃x³+p₂x²+p₁x+p₀``:

    Write ``Q₁ = x²+ax+b``, ``Q₂ = x²+cx+d``.  The four coefficient-match
    equations are:

    ``(1) a+c=p₃   (2) b+d+ac=p₂   (3) ad+bc=p₁   (4) bd=p₀``

    From (1): ``c = p₃−a``.  From (4): ``d = p₀/b``.  Substituting into (3)
    gives ``a = b(p₁−b·p₃)/(p₀−b²)`` whenever ``b²≠p₀``.  We then verify
    equation (2).
    """
    n = normalize(E)
    if len(n) - 1 != 4:
        return None
    coeffs = [Fraction(c) for c in n]
    p0, p1, p2, p3 = coeffs[0], coeffs[1], coeffs[2], coeffs[3]
    # leading coefficient must be 1 (monic)
    if coeffs[4] != Fraction(1):
        return None
    for b in _rational_divisors(p0):
        b2 = b * b
        if b2 == p0:
            continue  # denominator of a-formula would be zero
        denom = p0 - b2
        a = b * (p1 - b * p3) / denom
        c = p3 - a
        d = p0 / b
        # verify equation (2)
        if b + d + a * c != p2:
            continue
        # irreducibility: discriminant < 0 for both quadratics
        if a * a - 4 * b >= 0:
            continue
        if c * c - 4 * d >= 0:
            continue
        Q1: tuple[Fraction, ...] = (b, a, Fraction(1))
        Q2: tuple[Fraction, ...] = (d, c, Fraction(1))
        return Q1, Q2
    return None


def _solve_pf_2quad(
    num: tuple,
    Q1: tuple,
    Q2: tuple,
) -> tuple[Fraction, Fraction, Fraction, Fraction] | None:
    """Solve the partial-fraction system for ``N/(Q₁·Q₂)``.

    Finds ``A₁,B₁,A₂,B₂ ∈ Q`` satisfying
    ``N = (A₁x+B₁)·Q₂ + (A₂x+B₂)·Q₁``.

    The 4×4 system (matching x³, x², x, 1) is:

    ``[1  0  1  0 ][A₁]   [n₃]``
    ``[c  1  a  1 ][B₁] = [n₂]``
    ``[d  c  b  a ][A₂]   [n₁]``
    ``[0  d  0  b ][B₂]   [n₀]``

    where ``Q₁ = x²+ax+b`` and ``Q₂ = x²+cx+d``.

    Uses Gaussian elimination with partial pivoting over ``Fraction``.
    Returns ``None`` if the system is singular (shouldn't happen when
    Q₁ and Q₂ are coprime irreducible quadratics).
    """
    a, c = Fraction(Q1[1]), Fraction(Q2[1])
    b, d = Fraction(Q1[0]), Fraction(Q2[0])
    num_n = normalize(num)
    # Pad num to length 4 (coefficients of 1, x, x², x³).
    padded = [Fraction(0)] * 4
    for i, v in enumerate(num_n):
        if i < 4:
            padded[i] = Fraction(v)
    n0, n1, n2, n3 = padded[0], padded[1], padded[2], padded[3]

    # Build augmented matrix [M | rhs] row-major.
    # Unknowns order: [A1, B1, A2, B2].
    # Rows match x^3, x^2, x^1, x^0 coefficient equations.
    mat = [
        [Fraction(1), Fraction(0), Fraction(1), Fraction(0), n3],
        [c,           Fraction(1), a,           Fraction(1), n2],
        [d,           c,           b,            a,           n1],
        [Fraction(0), d,           Fraction(0), b,           n0],
    ]
    n_rows = 4
    for col in range(4):
        # Partial pivot: find row with largest abs value in this column.
        pivot_row = max(range(col, n_rows), key=lambda r: abs(mat[r][col]))
        mat[col], mat[pivot_row] = mat[pivot_row], mat[col]
        if mat[col][col] == 0:
            return None  # singular
        piv = mat[col][col]
        for row in range(col + 1, n_rows):
            if mat[row][col] == 0:
                continue
            factor = mat[row][col] / piv
            for k in range(col, 5):
                mat[row][k] -= factor * mat[col][k]
    # Back substitution.
    sol = [Fraction(0)] * 4
    for i in range(3, -1, -1):
        if mat[i][i] == 0:
            return None
        s = mat[i][4]
        for j in range(i + 1, 4):
            s -= mat[i][j] * sol[j]
        sol[i] = s / mat[i][i]
    return sol[0], sol[1], sol[2], sol[3]


def _try_multi_quad_integral(
    num: tuple, den: tuple, x_sym: IRSymbol
) -> IRNode | None:
    """Phase 9: integrate ``num/den`` when ``den`` is a product of two
    distinct irreducible quadratic factors.

    Pre-conditions mirror those for ``mixed_integral``:
    - RT, arctan (2e), and mixed (2f) have already returned ``None``.
    - ``den`` is squarefree with rational coefficients.
    - ``deg num < deg den``.

    Returns ``ADD(ir1, ir2)`` or ``None`` when the denominator doesn't
    fit the two-distinct-quadratics shape.
    """
    den_n = normalize(den)
    if len(den_n) - 1 != 4:
        return None
    den_fracs = tuple(Fraction(c) for c in den_n)
    # Must have no rational roots — otherwise mixed_integral would have fired.
    if rational_roots(den_fracs):
        return None
    # Make monic for the biquadratic algorithm.
    leading = den_fracs[-1]
    if leading == 0:
        return None
    monic_den = tuple(c / leading for c in den_fracs)
    factors = _factor_biquadratic(monic_den)
    if factors is None:
        return None
    Q1, Q2 = factors
    # Adjust numerator for the leading coefficient of den.
    num_n = normalize(num)
    num_fracs = tuple(Fraction(c) / leading for c in num_n)
    coeffs = _solve_pf_2quad(num_fracs, Q1, Q2)
    if coeffs is None:
        return None
    A1, B1, A2, B2 = coeffs
    ir1 = arctan_integral((B1, A1), Q1, x_sym)
    ir2 = arctan_integral((B2, A2), Q2, x_sym)
    return IRApply(ADD, (ir1, ir2))


# ---------------------------------------------------------------------------
# Phase 10 — Generalized partial fractions (three quadratics, L^m × Q₁ × Q₂)
# ---------------------------------------------------------------------------


def _factor_triple_quadratic(
    E: tuple,
) -> tuple[tuple, tuple, tuple] | None:
    """Factor a monic degree-6 squarefree poly into three irreducible monic quadratics.

    ``E`` must be a 7-element ascending Fraction coefficient tuple with leading
    coefficient 1 and no rational roots.  Iterates over candidate constant
    terms ``b₁`` from ``_rational_divisors(p₀)`` and linear coefficients
    ``a₁`` from a finite search set, checks exact divisibility via
    ``divmod_poly``, and delegates the degree-4 quotient to
    ``_factor_biquadratic``.  Returns ``(Q₁, Q₂, Q₃)`` on success, ``None``
    if no rational factorization is found.
    """
    E_fracs = tuple(Fraction(c) for c in E)
    if len(E_fracs) != 7:
        return None
    p0 = E_fracs[0]
    p5 = E_fracs[5]  # degree-5 coefficient
    if p0 == 0:
        return None

    # Candidate linear coefficients: small integers plus rational divisors
    # of the degree-5 coefficient (covers non-diagonal factors like x²+2x+5).
    small_ints = [Fraction(k) for k in range(-4, 5)]
    a1_extra = _rational_divisors(p5) if p5 != 0 else []
    seen_a: set[Fraction] = set()
    a1_candidates: list[Fraction] = []
    for a in small_ints + a1_extra:
        if a not in seen_a:
            seen_a.add(a)
            a1_candidates.append(a)

    for b1 in _rational_divisors(p0):
        for a1 in a1_candidates:
            Q1_cand = (b1, a1, Fraction(1))
            # Skip if Q1 is reducible over Q (discriminant ≥ 0).
            if a1 * a1 - 4 * b1 >= 0:
                continue
            quot, rem = divmod_poly(E_fracs, Q1_cand)
            if any(c != 0 for c in normalize(rem)):
                continue
            # Degree-4 quotient — try biquadratic factoring.
            quot_n = normalize(quot)
            if len(quot_n) != 5:
                continue
            # Make monic.
            lead = quot_n[-1]
            if lead == 0:
                continue
            monic_quot = tuple(c / lead for c in quot_n)
            rest = _factor_biquadratic(monic_quot)
            if rest is None:
                continue
            Q2, Q3 = rest
            return (Q1_cand, Q2, Q3)
    return None


def _solve_pf_general(
    num: tuple, factors: list[tuple]
) -> list[tuple] | None:
    """Solve the partial-fraction system N/∏Pᵢ = ΣNᵢ/Pᵢ by Gaussian elimination.

    ``factors`` is a list of coprime polynomial tuples (ascending Fraction
    coefficients).  ``D = Σ deg Pᵢ`` is the total degree; the system has
    ``D`` equations and ``D`` unknowns (the coefficients of each ``Nᵢ``).

    The D×D matrix is built column by column: the column for the j-th
    coefficient of ``Nᵢ`` is the cofactor ``Mᵢ = ∏_{k≠i} Pₖ`` shifted
    left by ``j`` rows.

    Returns a list of numerator tuples ``[N₁, N₂, …, Nₖ]`` (each in
    ascending-coefficient order), or ``None`` if the system is singular.
    """
    D = sum(len(f) - 1 for f in factors)
    if D == 0:
        return None

    # Build cofactors Mᵢ = ∏_{j≠i} Pⱼ.
    cofactors: list[tuple] = []
    for i in range(len(factors)):
        M: tuple = (Fraction(1),)
        for j, Pj in enumerate(factors):
            if j != i:
                M = multiply(M, Pj)
        cofactors.append(normalize(M))

    # Assemble D×D augmented matrix [A | b].
    A = [[Fraction(0)] * (D + 1) for _ in range(D)]
    # RHS: coefficients of num padded to length D.
    num_f = tuple(Fraction(c) for c in normalize(num))
    for row in range(D):
        A[row][D] = num_f[row] if row < len(num_f) else Fraction(0)

    col = 0
    for Pi, Mi in zip(factors, cofactors, strict=True):
        di = len(Pi) - 1
        for j in range(di):  # Nᵢ = Σ c_{i,j} · xʲ
            for row in range(D):
                k = row - j
                if 0 <= k < len(Mi):
                    A[row][col] = Mi[k]
            col += 1

    # Gaussian elimination with partial pivoting over Fraction.
    pivot_row = 0
    col_order: list[int] = []
    for c in range(D):
        # Find pivot.
        best = pivot_row
        for r in range(pivot_row + 1, D):
            if abs(A[r][c]) > abs(A[best][c]):
                best = r
        if A[best][c] == 0:
            return None  # Singular — shouldn't happen for coprime factors.
        A[pivot_row], A[best] = A[best], A[pivot_row]
        col_order.append(c)
        # Eliminate.
        piv = A[pivot_row][c]
        for r in range(D):
            if r == pivot_row:
                continue
            if A[r][c] == 0:
                continue
            factor = A[r][c] / piv
            for cc in range(D + 1):
                A[r][cc] -= factor * A[pivot_row][cc]
        pivot_row += 1

    # Back-substitution: extract solution.
    solution = [Fraction(0)] * D
    for pr in range(D):
        c = col_order[pr]
        piv = A[pr][c]
        if piv == 0:
            return None
        solution[c] = A[pr][D] / piv

    # Repack solution into per-factor numerator tuples.
    result: list[tuple] = []
    idx = 0
    for Pi in factors:
        di = len(Pi) - 1
        Ni = tuple(solution[idx + j] for j in range(di))
        result.append(Ni)
        idx += di
    return result


def _try_general_rational_integral(
    num: tuple, den: tuple, x_sym: IRSymbol
) -> IRNode | None:
    """Phase 10 driver — generalized partial fractions for degree-5/6 denominators.

    Handles two cases after RT, arctan (2e), mixed (2f), and Phase 9 all return None:

    - **Q₁·Q₂·Q₃** (degree 6, no rational roots): factors the denominator
      into three irreducible monic quadratics via ``_factor_triple_quadratic``,
      then solves the 6×6 partial-fraction system.

    - **Lᵐ·Q₁·Q₂** (degree 5 or 6, at least one rational root): extracts the
      linear factors via ``rational_roots`` + ``multiply``, factors the degree-4
      quadratic remainder via ``_factor_biquadratic``, and solves the D×D system.

    Each linear piece ``A/(x−r)`` integrates to ``A·log(x−r)`` (via
    ``rt_pairs_to_ir``); each quadratic piece uses ``arctan_integral``.
    """
    den_n = normalize(den)
    deg = len(den_n) - 1
    if deg not in (5, 6):
        return None

    den_fracs = tuple(Fraction(c) for c in den_n)
    # Normalize to monic.
    leading = den_fracs[-1]
    if leading == 0:
        return None
    monic_den = tuple(c / leading for c in den_fracs)

    # Find all rational roots (linear factors).
    roots = rational_roots(monic_den)

    # Build L = ∏(x − r) for each rational root.
    L: tuple = (Fraction(1),)
    for r in roots:
        L = multiply(L, (-r, Fraction(1)))
    L = normalize(L)

    # Divide out the linear part to get the quadratic remainder Q_total.
    Q_total, rem = divmod_poly(monic_den, L)
    if any(c != 0 for c in normalize(rem)):
        return None
    Q_total = normalize(Q_total)
    qdeg = len(Q_total) - 1

    # Q_total must be degree 4 (two quadratics) or 6 (three quadratics).
    if qdeg not in (4, 6):
        return None

    # Q_total must have no rational roots (all remaining factors are quadratic).
    if rational_roots(Q_total):
        return None

    # Make Q_total monic.
    q_lead = Q_total[-1]
    if q_lead == 0:
        return None
    monic_Q = tuple(c / q_lead for c in Q_total)

    # Factor Q_total into irreducible quadratics.
    if qdeg == 4:
        quad_pair = _factor_biquadratic(monic_Q)
        if quad_pair is None:
            return None
        quad_list = list(quad_pair)
    else:  # qdeg == 6
        triple = _factor_triple_quadratic(monic_Q)
        if triple is None:
            return None
        quad_list = list(triple)

    # Assemble the full factor list: linear factors first, then quadratics.
    linear_factors: list[tuple] = [
        normalize((-r, Fraction(1))) for r in roots
    ]
    all_factors: list[tuple] = linear_factors + quad_list

    # Verify total degree matches (sanity check).
    total_d = sum(len(f) - 1 for f in all_factors)
    if total_d != deg:
        return None

    # Adjust numerator for the leading coefficient of the original den.
    num_n = normalize(num)
    num_fracs = tuple(Fraction(c) / leading for c in num_n)

    # Solve the D×D partial-fraction system.
    pieces_num = _solve_pf_general(num_fracs, all_factors)
    if pieces_num is None:
        return None

    # Integrate each piece.
    ir_parts: list[IRNode] = []
    for fi, Ni in zip(all_factors, pieces_num, strict=True):
        fi_deg = len(fi) - 1
        if fi_deg == 1:
            # ∫ A/(x−r) dx = A·log(x−r).
            A = Ni[0] if Ni else Fraction(0)
            r = -fi[0] / fi[1]  # fi = (−r, 1) so fi[0] = −r·fi[1]
            if A != 0:
                log_ir = rt_pairs_to_ir([(A, fi)], x_sym)
                ir_parts.append(log_ir)
        elif fi_deg == 2:
            # ∫ (Ax+B)/Qᵢ dx via Phase 2e arctan_integral.
            part_ir = arctan_integral(Ni, fi, x_sym)
            ir_parts.append(part_ir)

    if not ir_parts:
        return None

    acc = ir_parts[0]
    for p in ir_parts[1:]:
        acc = IRApply(ADD, (acc, p))
    return acc
