"""Special functions as integration and differentiation fallback — Phase 23.

When the Risch algorithm and all IBP rules exhaust their elementary
representations, MACSYMA returned answers in terms of named special functions.
This module provides the pattern-matching, differentiation rules, and numeric
evaluation logic for the five families introduced in Phase 23.

Integration fallbacks (each returns ``IRNode | None``):

  ∫ exp(c·x²) dx    → erf or erfi form       [23a]
  ∫ sin(ax)/x dx    → Si(ax)                  [23b]
  ∫ cos(ax)/x dx    → Ci(ax)                  [23b]
  ∫ sinh(ax)/x dx   → Shi(ax)                 [23b]
  ∫ cosh(ax)/x dx   → Chi(ax)                 [23b]
  ∫ log(1-ax)/x dx  → -Li₂(ax)               [23c]
  ∫ log(x)/(1-x) dx → Li₂(1-x)               [23c]
  ∫ sin(q·π·x²) dx  → scaled FresnelS         [23e]
  ∫ cos(q·π·x²) dx  → scaled FresnelC         [23e]

Differentiation rules (called from derivative.py via ``diff_special``):

  d/dx erf(f)       = (2/√π)·exp(−f²)·f′
  d/dx erfc(f)      = −(2/√π)·exp(−f²)·f′
  d/dx erfi(f)      = (2/√π)·exp(f²)·f′
  d/dx Si(f)        = sin(f)/f·f′
  d/dx Ci(f)        = cos(f)/f·f′
  d/dx Shi(f)       = sinh(f)/f·f′
  d/dx Chi(f)       = cosh(f)/f·f′
  d/dx Li₂(f)       = −log(1−f)/f·f′
  d/dx FresnelS(f)  = sin(π·f²/2)·f′
  d/dx FresnelC(f)  = cos(π·f²/2)·f′

Numeric evaluation (pure Python; no external CAS libraries):

  ``gamma_eval(n)``   — exact for positive integers and half-integers;
                        Lanczos (g=7) for general floats.
  ``beta_eval(a, b)`` — reduces via Γ(a)·Γ(b) / Γ(a+b).
  ``erf_numeric(x)``  — convergent series for |x| ≤ 4; asymptotic otherwise.
  ``si_numeric(x)``   — Maclaurin series, enough terms for |x| ≤ 30.
  ``ci_numeric(x)``   — via auxiliary functions f(x), g(x).
  ``li2_numeric(x)``  — series for |x| < 1, functional equations otherwise.
  ``fresnel_s_numeric(x)`` / ``fresnel_c_numeric(x)`` — Maclaurin series.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    CHI,
    CI,
    COS,
    COSH,
    DIV,
    ERF,
    ERFC,
    ERFI,
    EXP,
    FRESNEL_C,
    FRESNEL_S,
    LI2,
    LOG,
    MUL,
    NEG,
    POW,
    SHI,
    SI,
    SIN,
    SINH,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import to_rational

# ── Module-level constant symbols ────────────────────────────────────────────

_PI_SYM = IRSymbol("%pi")  # MACSYMA's %pi constant

ONE = IRInteger(1)
TWO = IRInteger(2)
ZERO = IRInteger(0)


# ── IR construction helpers ───────────────────────────────────────────────────


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a :class:`~fractions.Fraction` to its canonical IR literal.

    ``Fraction(3, 1)`` → ``IRInteger(3)``
    ``Fraction(1, 2)`` → ``IRRational(1, 2)``

    Avoids proliferating unnecessary ``IRRational(n, 1)`` wrappers.
    """
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _node_to_frac(node: IRNode) -> Fraction | None:
    """Return a Fraction if *node* is a numeric literal, else ``None``."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _isqrt_exact(n: int) -> int | None:
    """Return the integer square root of *n* if *n* is a perfect square, else None.

    Works for both positive and zero:  ``_isqrt_exact(9) == 3``,
    ``_isqrt_exact(8) == None``.
    """
    if n < 0:
        return None
    r = math.isqrt(n)
    return r if r * r == n else None


def _sqrt_frac_ir(q: Fraction) -> IRNode:
    """Return the IR node for √q, simplified where possible.

    If ``q = p²/r²`` (both numerator and denominator are perfect squares),
    return the exact rational.  Otherwise wrap in SQRT.

    Examples::

      _sqrt_frac_ir(Fraction(1, 1))  → IRInteger(1)
      _sqrt_frac_ir(Fraction(4, 1))  → IRInteger(2)
      _sqrt_frac_ir(Fraction(1, 4))  → IRRational(1, 2)
      _sqrt_frac_ir(Fraction(2, 1))  → IRApply(SQRT, (IRInteger(2),))
    """
    if q.numerator <= 0:
        # Return unevaluated for zero/negative; callers should not reach here.
        return IRApply(SQRT, (_frac_ir(q),))
    sn = _isqrt_exact(q.numerator)
    sd = _isqrt_exact(q.denominator)
    if sn is not None and sd is not None:
        return _frac_ir(Fraction(sn, sd))
    # Cannot simplify — emit SQRT(q).
    return IRApply(SQRT, (_frac_ir(q),))


# ── Low-level IR tree helpers ─────────────────────────────────────────────────


def _try_linear_frac(
    node: IRNode, x: IRSymbol
) -> tuple[Fraction, Fraction] | None:
    """Return (a, b) if *node* = a·x + b with rational a, b; else None.

    Uses :func:`polynomial_bridge.to_rational` for reliable handling of all
    IR shapes the MACSYMA compiler may emit (bare x, c*x, neg(c*x), etc.).
    """
    result = to_rational(node, x)
    if result is None:
        return None
    num, den = result
    # Denominator must be the constant 1.
    if len(den) != 1 or Fraction(den[0]) != Fraction(1):
        return None
    # Numerator must be degree ≤ 1.
    if len(num) > 2:
        return None
    if len(num) == 0:
        return (Fraction(0), Fraction(0))
    b = Fraction(num[0])
    a = Fraction(num[1]) if len(num) == 2 else Fraction(0)
    return (a, b)


def _try_quadratic_coeff(node: IRNode, x: IRSymbol) -> Fraction | None:
    """Return c if *node* = c·x² (no linear or constant terms), else None.

    Pattern: the polynomial ``to_rational`` yields must be exactly
    ``(0, 0, c)`` — zero constant, zero linear coefficient, nonzero c.
    """
    result = to_rational(node, x)
    if result is None:
        return None
    num, den = result
    # Denominator must be constant 1.
    if len(den) != 1 or Fraction(den[0]) != Fraction(1):
        return None
    # Numerator must have degree exactly 2 with zero lower-degree coefficients.
    if len(num) != 3:
        return None
    if Fraction(num[0]) != Fraction(0) or Fraction(num[1]) != Fraction(0):
        return None
    c = Fraction(num[2])
    return c if c != Fraction(0) else None


def _is_pi_sym(node: IRNode) -> bool:
    """True iff *node* is the ``%pi`` symbol."""
    return isinstance(node, IRSymbol) and node == _PI_SYM


def _collect_mul_div(
    node: IRNode,
) -> tuple[list[IRNode], list[IRNode]]:
    """Walk a MUL/DIV/NEG tree, collecting numerator and denominator factors.

    Returns ``(num_factors, den_factors)`` so the caller can inspect each
    leaf individually (checking for ``%pi``, ``POW(x,2)``, rational constants,
    etc.).

    ``MUL(%pi, DIV(POW(x,2), 2))``
      → numerator=[%pi, POW(x,2)], denominator=[2]
    ``DIV(MUL(%pi, POW(x,2)), 2)``
      → same result
    ``NEG(MUL(%pi, POW(x,2)))``
      → numerator=[-1, %pi, POW(x,2)], denominator=[]
    """
    if not isinstance(node, IRApply):
        return ([node], [])
    if node.head == MUL:
        n1, d1 = _collect_mul_div(node.args[0])
        n2, d2 = _collect_mul_div(node.args[1])
        return (n1 + n2, d1 + d2)
    if node.head == DIV:
        # DIV(a, b) = a/b.  Collect a's factors normally; flip b's.
        n_a, d_a = _collect_mul_div(node.args[0])
        n_b, d_b = _collect_mul_div(node.args[1])
        return (n_a + d_b, d_a + n_b)
    if node.head == NEG:
        n, d = _collect_mul_div(node.args[0])
        return ([IRInteger(-1)] + n, d)
    return ([node], [])


def _try_pi_factor_and_quadratic(
    arg: IRNode, x: IRSymbol
) -> Fraction | None:
    """Return q if *arg* = q · %pi · x², else ``None`` (q rational ≠ 0).

    Handles any product/quotient arrangement of ``%pi``, ``x²``, and
    rational constants.  Examples that all return ``Fraction(1, 2)``:
    - ``MUL(%pi, DIV(POW(x,2), 2))``
    - ``DIV(MUL(%pi, POW(x,2)), 2)``
    - ``MUL(DIV(1,2), MUL(%pi, POW(x,2)))``
    """
    num_factors, den_factors = _collect_mul_div(arg)
    has_pi = False
    has_x2 = False
    rational = Fraction(1)

    for f in num_factors:
        if _is_pi_sym(f):
            if has_pi:
                return None  # two π factors — not the expected form
            has_pi = True
        elif (
            isinstance(f, IRApply)
            and f.head == POW
            and len(f.args) == 2
            and f.args[0] == x
            and isinstance(f.args[1], IRInteger)
            and f.args[1].value == 2
        ):
            if has_x2:
                return None  # two x² factors — not the expected form
            has_x2 = True
        else:
            c = _node_to_frac(f)
            if c is None:
                return None  # unknown symbolic factor
            rational *= c

    for f in den_factors:
        if _is_pi_sym(f) or isinstance(f, IRSymbol):
            return None  # π or free symbol in denominator — can't handle
        c = _node_to_frac(f)
        if c is None or c == Fraction(0):
            return None
        rational /= c

    if not has_pi or not has_x2:
        return None

    return rational if rational != Fraction(0) else None


# ── 23a — Error function integration ─────────────────────────────────────────


def try_erf_integral(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Return the antiderivative of ``exp(c·x²)`` in erf/erfi form.

    Integrand must be exactly ``Exp(c·x²)`` with rational c ≠ 0.

    Formula:
      c < 0 (α = √(−c)):  ∫ exp(c·x²) dx = √π / (2·α) · erf(α·x)
      c > 0 (α = √c):     ∫ exp(c·x²) dx = √π / (2·α) · erfi(α·x)

    The ``α = 1`` case (c = ±1) simplifies cleanly:
      ∫ exp(−x²) dx = √π/2 · erf(x)
      ∫ exp(x²)  dx = √π/2 · erfi(x)
    """
    if not isinstance(f, IRApply) or f.head != EXP:
        return None
    if len(f.args) != 1:
        return None
    c = _try_quadratic_coeff(f.args[0], x)
    if c is None:
        return None  # exponent is not a pure c·x²

    # Determine α = √|c|.  _sqrt_frac_ir simplifies when |c| is a perfect square.
    abs_c = Fraction(-c) if c < 0 else c
    alpha_node = _sqrt_frac_ir(abs_c)  # √|c|

    # Build erf(α·x) or erfi(α·x).  If α = 1, the argument is just x.
    if abs_c == Fraction(1):
        arg_node: IRNode = x
    else:
        arg_node = IRApply(MUL, (alpha_node, x))

    special_fn = ERF if c < 0 else ERFI
    special_node = IRApply(special_fn, (arg_node,))

    # Coefficient = √π / (2·α).
    sqrt_pi: IRNode = IRApply(SQRT, (_PI_SYM,))
    if abs_c == Fraction(1):
        # Coefficient = √π/2.
        coeff: IRNode = IRApply(DIV, (sqrt_pi, TWO))
    else:
        # Coefficient = √π / (2·α).
        two_alpha: IRNode = IRApply(MUL, (TWO, alpha_node))
        coeff = IRApply(DIV, (sqrt_pi, two_alpha))

    return IRApply(MUL, (coeff, special_node))


# ── 23b — Trigonometric integral integration ──────────────────────────────────


def try_si_ci_integral(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Return the antiderivative when the integrand is trig(ax)/x.

    Detected forms and their antiderivatives:
      sin(ax)/x  → Si(ax)       (sine integral)
      cos(ax)/x  → Ci(ax)       (cosine integral)
      sinh(ax)/x → Shi(ax)      (hyperbolic sine integral)
      cosh(ax)/x → Chi(ax)      (hyperbolic cosine integral)

    Derivation: d/dx Si(ax) = sin(ax)/(ax) · a = sin(ax)/x ✓.

    The denominator must be *exactly* the integration variable ``x`` (not
    a scaled multiple like ``ax``), because the cancellation in the chain
    rule above relies on that exact form.
    """
    if not isinstance(f, IRApply) or f.head != DIV:
        return None
    if len(f.args) != 2:
        return None
    numerator, denominator = f.args

    # Denominator must be exactly x.
    if denominator != x:
        return None

    if not isinstance(numerator, IRApply):
        return None

    head = numerator.head
    if head not in {SIN, COS, SINH, COSH}:
        return None
    if len(numerator.args) != 1:
        return None

    # Argument to the trig function must be a·x (linear, zero constant term).
    lin = _try_linear_frac(numerator.args[0], x)
    if lin is None:
        return None
    a, b = lin
    if a == Fraction(0) or b != Fraction(0):
        # a = 0 means the trig arg is constant; b ≠ 0 adds a shift.
        # Both cases fall outside the pure Si/Ci antiderivative form.
        return None

    # Inner argument: Si(ax) needs ax in the head.
    if a == Fraction(1):
        inner: IRNode = x
    else:
        inner = IRApply(MUL, (_frac_ir(a), x))

    _HEAD_MAP = {SIN: SI, COS: CI, SINH: SHI, COSH: CHI}
    return IRApply(_HEAD_MAP[head], (inner,))


# ── 23c — Dilogarithm integration ────────────────────────────────────────────


def try_li2_integral(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Return the antiderivative when the integrand matches a Li₂ pattern.

    Two patterns are recognised:

    **Pattern 1** — ``log(1 − a·x) / x`` → ``−Li₂(a·x)``

    Derivation:
      d/dx (−Li₂(ax)) = −d/d(ax) Li₂(ax) · a = log(1−ax)/(ax) · a = log(1−ax)/x ✓

    **Pattern 2** — ``log(x) / (1 − x)`` → ``Li₂(1 − x)``

    Derivation:
      d/dx Li₂(1−x) = −log(1−(1−x))/(1−x) · (−1) = log(x)/(1−x) ✓
    """
    if not isinstance(f, IRApply) or f.head != DIV:
        return None
    if len(f.args) != 2:
        return None
    numer, denom = f.args

    # ── Pattern 1: log(1 − ax) / x ─────────────────────────────────────────
    if denom == x and isinstance(numer, IRApply) and numer.head == LOG:
        log_arg = numer.args[0]
        # log_arg must be (1 − ax), i.e. linear with b = 1, a_coeff = −a < 0.
        lin = _try_linear_frac(log_arg, x)
        if lin is not None:
            a_coeff, b = lin
            # log_arg = a_coeff · x + b.  We need b = 1 and a_coeff < 0.
            if b == Fraction(1) and a_coeff < Fraction(0):
                # Scale a = −a_coeff > 0.  log_arg = 1 − a·x.
                a = -a_coeff
                if a == Fraction(1):
                    inner: IRNode = x
                else:
                    inner = IRApply(MUL, (_frac_ir(a), x))
                return IRApply(NEG, (IRApply(LI2, (inner,)),))

    # ── Pattern 2: log(x) / (1 − x) ────────────────────────────────────────
    if (
        isinstance(numer, IRApply)
        and numer.head == LOG
        and len(numer.args) == 1
        and numer.args[0] == x
    ):
        # Denominator must be 1 − x, i.e. linear with a_coeff = −1, b = 1.
        lin = _try_linear_frac(denom, x)
        if lin is not None:
            a_coeff, b = lin
            if a_coeff == Fraction(-1) and b == Fraction(1):
                # ∫ log(x)/(1−x) dx = Li₂(1−x).
                one_minus_x: IRNode = IRApply(SUB, (ONE, x))
                return IRApply(LI2, (one_minus_x,))

    return None


# ── 23e — Fresnel integral integration ────────────────────────────────────────


def try_fresnel_integral(f: IRNode, x: IRSymbol) -> IRNode | None:
    """Return the antiderivative for sin/cos of a quadratic in x.

    Two classes are handled:

    **Class A — argument contains π:**  ``q · π · x²``

      ∫ sin(q·π·x²) dx = (1/√(2q)) · FresnelS(√(2q)·x)

    Special case q = 1/2:  ∫ sin(π·x²/2) dx = FresnelS(x)   (cleanest form)

    **Class B — pure rational quadratic:**  ``a · x²``  (no π factor)

      ∫ sin(a·x²) dx = √(π/(2a)) · FresnelS(x·√(2a/π))

    Both cases: replace SIN with COS to get the FresnelC analogue.

    Mathematical basis:
      FresnelS(x) = ∫₀^x sin(π·t²/2) dt  →  d/dx FresnelS(x) = sin(π·x²/2)

    For scaled argument u = √(2q)·x:
      d/dx FresnelS(√(2q)·x) = sin(π·(√(2q)·x)²/2)·√(2q) = sin(q·π·x²)·√(2q)
      so ∫ sin(q·π·x²) dx = (1/√(2q)) · FresnelS(√(2q)·x)
    """
    if not isinstance(f, IRApply):
        return None
    head = f.head
    if head not in {SIN, COS}:
        return None
    if len(f.args) != 1:
        return None
    arg = f.args[0]

    fresnel_fn = FRESNEL_S if head == SIN else FRESNEL_C

    # ── Class A: argument = q·π·x² ──────────────────────────────────────────
    q = _try_pi_factor_and_quadratic(arg, x)
    if q is not None and q > Fraction(0):
        two_q = Fraction(2) * q
        if two_q == Fraction(1):
            # q = 1/2 → ∫ sin(π·x²/2) dx = FresnelS(x).
            return IRApply(fresnel_fn, (x,))
        # General: (1/√(2q)) · FresnelS(√(2q)·x)
        sqrt_2q = _sqrt_frac_ir(two_q)
        scale_arg: IRNode = IRApply(MUL, (sqrt_2q, x))
        coeff: IRNode = IRApply(DIV, (ONE, sqrt_2q))
        return IRApply(MUL, (coeff, IRApply(fresnel_fn, (scale_arg,))))

    # ── Class B: argument = a·x² (rational, no π) ───────────────────────────
    a = _try_quadratic_coeff(arg, x)
    if a is not None and a > Fraction(0):
        # ∫ sin(a·x²) dx = √(π/(2a)) · FresnelS(x·√(2a/π))
        # We emit SQRT(%pi/(2a)) and SQRT(2a/%pi) symbolically.
        two_a = Fraction(2) * a
        sqrt_pi_over_2a: IRNode = IRApply(
            SQRT, (IRApply(DIV, (_PI_SYM, _frac_ir(two_a))),)
        )
        sqrt_2a_over_pi: IRNode = IRApply(
            SQRT, (IRApply(DIV, (_frac_ir(two_a), _PI_SYM)),)
        )
        scale_arg2: IRNode = IRApply(MUL, (x, sqrt_2a_over_pi))
        return IRApply(
            MUL, (sqrt_pi_over_2a, IRApply(fresnel_fn, (scale_arg2,)))
        )

    return None


# ── Dispatch table for integration fallbacks ─────────────────────────────────

#: Ordered list of special-function fallback integration functions.
#: ``_integrate()`` in ``integrate.py`` calls each in turn after all
#: elementary rules have returned ``None``.
INTEGRATION_FALLBACKS = (
    try_erf_integral,
    try_si_ci_integral,
    try_li2_integral,
    try_fresnel_integral,
)


# ── Differentiation rules ─────────────────────────────────────────────────────


def diff_special(
    f: IRApply, x: IRSymbol, _diff_fn: object
) -> IRNode | None:
    """Apply the Phase 23 chain-rule derivative for a special-function head.

    Returns ``d/dx f`` as unevaluated IR if the head of *f* is one of the
    Phase 23 special functions; returns ``None`` if *f* is not recognised
    (so ``derivative.py`` can fall through to its ``D(f, x)`` sentinel).

    *_diff_fn* is the recursive ``_diff`` function from ``derivative.py``,
    injected to avoid a circular import.
    """
    _diff = _diff_fn  # type: ignore[assignment]
    head = f.head
    if len(f.args) != 1:
        return None  # all Phase 23 specials take exactly one argument
    inner = f.args[0]

    # ── d/dx erf(f) = (2/√π)·exp(−f²)·f′ ───────────────────────────────────
    if head == ERF:
        # 2/√π · exp(−inner²) · inner′
        inner_prime = _diff(inner, x)
        two_over_sqrt_pi: IRNode = IRApply(
            DIV, (TWO, IRApply(SQRT, (_PI_SYM,)))
        )
        exp_neg_sq: IRNode = IRApply(
            EXP, (IRApply(NEG, (IRApply(POW, (inner, TWO)),)),)
        )
        return IRApply(
            MUL,
            (two_over_sqrt_pi, IRApply(MUL, (exp_neg_sq, inner_prime))),
        )

    # ── d/dx erfc(f) = −(2/√π)·exp(−f²)·f′ ──────────────────────────────────
    if head == ERFC:
        inner_prime = _diff(inner, x)
        two_over_sqrt_pi = IRApply(DIV, (TWO, IRApply(SQRT, (_PI_SYM,))))
        exp_neg_sq = IRApply(
            EXP, (IRApply(NEG, (IRApply(POW, (inner, TWO)),)),)
        )
        pos = IRApply(MUL, (two_over_sqrt_pi, IRApply(MUL, (exp_neg_sq, inner_prime))))
        return IRApply(NEG, (pos,))

    # ── d/dx erfi(f) = (2/√π)·exp(f²)·f′ ────────────────────────────────────
    if head == ERFI:
        inner_prime = _diff(inner, x)
        two_over_sqrt_pi = IRApply(DIV, (TWO, IRApply(SQRT, (_PI_SYM,))))
        exp_sq: IRNode = IRApply(EXP, (IRApply(POW, (inner, TWO)),))
        return IRApply(
            MUL,
            (two_over_sqrt_pi, IRApply(MUL, (exp_sq, inner_prime))),
        )

    # ── d/dx Si(f) = sin(f)/f · f′ ───────────────────────────────────────────
    if head == SI:
        inner_prime = _diff(inner, x)
        sin_over_inner: IRNode = IRApply(DIV, (IRApply(SIN, (inner,)), inner))
        return IRApply(MUL, (sin_over_inner, inner_prime))

    # ── d/dx Ci(f) = cos(f)/f · f′ ───────────────────────────────────────────
    if head == CI:
        inner_prime = _diff(inner, x)
        cos_over_inner: IRNode = IRApply(DIV, (IRApply(COS, (inner,)), inner))
        return IRApply(MUL, (cos_over_inner, inner_prime))

    # ── d/dx Shi(f) = sinh(f)/f · f′ ─────────────────────────────────────────
    if head == SHI:
        inner_prime = _diff(inner, x)
        sinh_over_inner: IRNode = IRApply(
            DIV, (IRApply(SINH, (inner,)), inner)
        )
        return IRApply(MUL, (sinh_over_inner, inner_prime))

    # ── d/dx Chi(f) = cosh(f)/f · f′ ─────────────────────────────────────────
    if head == CHI:
        inner_prime = _diff(inner, x)
        cosh_over_inner: IRNode = IRApply(
            DIV, (IRApply(COSH, (inner,)), inner)
        )
        return IRApply(MUL, (cosh_over_inner, inner_prime))

    # ── d/dx Li₂(f) = −log(1−f)/f · f′ ──────────────────────────────────────
    if head == LI2:
        inner_prime = _diff(inner, x)
        one_minus_f: IRNode = IRApply(SUB, (ONE, inner))
        neg_log_over_f: IRNode = IRApply(
            NEG,
            (IRApply(DIV, (IRApply(LOG, (one_minus_f,)), inner)),),
        )
        return IRApply(MUL, (neg_log_over_f, inner_prime))

    # ── d/dx FresnelS(f) = sin(π·f²/2) · f′ ─────────────────────────────────
    if head == FRESNEL_S:
        inner_prime = _diff(inner, x)
        # π·f²/2
        pi_f_sq_over_2: IRNode = IRApply(
            DIV,
            (
                IRApply(MUL, (_PI_SYM, IRApply(POW, (inner, TWO)))),
                TWO,
            ),
        )
        return IRApply(MUL, (IRApply(SIN, (pi_f_sq_over_2,)), inner_prime))

    # ── d/dx FresnelC(f) = cos(π·f²/2) · f′ ─────────────────────────────────
    if head == FRESNEL_C:
        inner_prime = _diff(inner, x)
        pi_f_sq_over_2 = IRApply(
            DIV,
            (
                IRApply(MUL, (_PI_SYM, IRApply(POW, (inner, TWO)))),
                TWO,
            ),
        )
        return IRApply(MUL, (IRApply(COS, (pi_f_sq_over_2,)), inner_prime))

    return None  # not a Phase 23 head


#: Set of IR head symbols handled by :func:`diff_special`.
DIFF_SPECIAL_HEADS: frozenset[IRSymbol] = frozenset(
    {ERF, ERFC, ERFI, SI, CI, SHI, CHI, LI2, FRESNEL_S, FRESNEL_C}
)


# ── 23d — Numeric evaluation helpers ─────────────────────────────────────────


def gamma_eval(z: float) -> float:
    """Evaluate Γ(z) numerically using the Lanczos approximation (g=7).

    Handles positive real z.  For z ≤ 0 (or non-integer poles) raises
    ``ValueError``.

    The 7-term Lanczos approximation is accurate to ~13 significant figures
    for Re(z) > 0.  Coefficients from Numerical Recipes, §6.1.

    Special cases computed before the approximation:
      - Positive integers n: returns (n−1)! exactly as a float.
      - Half-integers (n + 1/2): returns the exact value via
        ``Γ(n + 1/2) = (2n−1)!! / 2^n · √π``.
    """
    if z <= 0.0:
        raise ValueError(f"gamma_eval: z must be positive, got {z}")

    # ── Exact integer case ────────────────────────────────────────────────────
    if z == round(z) and z <= 21:
        n = int(round(z))
        return float(math.factorial(n - 1))

    # ── Half-integer case Γ(n + 1/2) ─────────────────────────────────────────
    half = z - 0.5
    if half == round(half) and half >= 0:
        # Γ(1/2) = √π; Γ(3/2) = √π/2; Γ(5/2) = 3√π/4; Γ(7/2) = 15√π/8
        # Recurrence: Γ(n+3/2) = (n+1/2)·Γ(n+1/2)
        n = int(round(half))  # z = n + 1/2
        sqrt_pi = math.sqrt(math.pi)
        # Compute Γ(1/2) = √π, then step up.
        g = sqrt_pi
        for k in range(n):
            g *= k + 0.5
        return g

    # ── Lanczos approximation ─────────────────────────────────────────────────
    # Coefficients for g = 7 from Lanczos (1964).
    _g = 7
    _c = (
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    )
    if z < 0.5:
        # Use reflection formula: Γ(z)·Γ(1−z) = π/sin(π·z).
        return math.pi / (math.sin(math.pi * z) * gamma_eval(1.0 - z))
    z -= 1
    x = _c[0]
    for i in range(1, _g + 2):
        x += _c[i] / (z + i)
    t = z + _g + 0.5
    return math.sqrt(2 * math.pi) * (t ** (z + 0.5)) * math.exp(-t) * x


def beta_eval(a: float, b: float) -> float:
    """Evaluate B(a, b) = Γ(a)·Γ(b)/Γ(a+b) numerically."""
    return gamma_eval(a) * gamma_eval(b) / gamma_eval(a + b)


def erf_numeric(x: float) -> float:
    """Evaluate erf(x) = (2/√π) ∫₀^x exp(−t²) dt numerically.

    Uses the convergent power series for all finite x:
      erf(x) = (2/√π) · Σ_{n=0}^∞ (−1)^n · x^(2n+1) / (n! · (2n+1))

    The series converges for all real x, though for large |x| we use
    the relation erf(x) → ±1.
    """
    if abs(x) > 27.0:
        return math.copysign(1.0, x)
    # Use math.erf (available in Python ≥ 3.2, implemented via C libm).
    return math.erf(x)


def erfi_numeric(x: float) -> float:
    """Evaluate erfi(x) = −i·erf(ix) = (2/√π) ∫₀^x exp(t²) dt numerically.

    For moderate |x|, use the power series:
      erfi(x) = (2/√π) · Σ_{n=0}^∞ x^(2n+1) / (n! · (2n+1))

    For large |x| the function grows rapidly — cap at 1e300.
    """
    if abs(x) > 27.0:
        return math.copysign(1e300, x)
    # Compute via the series (avoid overflow with early exit).
    term = x
    total = x
    x2 = x * x
    for n in range(1, 200):
        term *= x2 / n
        delta = term / (2 * n + 1)
        total += delta
        if abs(delta) < abs(total) * 1e-15:
            break
    return total * 2.0 / math.sqrt(math.pi)


def si_numeric(x: float) -> float:
    """Evaluate Si(x) = ∫₀^x sin(t)/t dt numerically.

    Uses the convergent Maclaurin series:
      Si(x) = Σ_{n=0}^∞ (−1)^n · x^(2n+1) / ((2n+1) · (2n+1)!)

    Converges for all finite x.  For |x| ≤ 30 the series is practical;
    for larger |x| we use the auxiliary function approach (see ci_numeric).
    """
    if abs(x) > 1e14:
        return math.copysign(math.pi / 2, x)
    total = 0.0
    term = x
    for n in range(200):
        total += term / (2 * n + 1)
        # Next term: multiply by −x² / ((2n+2)(2n+3))
        term *= -(x * x) / ((2 * n + 2) * (2 * n + 3))
        if abs(term) < abs(total) * 1e-15:
            break
    return total


def ci_numeric(x: float) -> float:
    """Evaluate Ci(x) = γ + log(x) + ∫₀^x (cos(t)−1)/t dt for x > 0.

    Uses the Maclaurin series for the integral part:
      ∫₀^x (cos(t)−1)/t dt = Σ_{n=1}^∞ (−1)^n · x^(2n) / (2n · (2n)!)

    Combined with γ + log(x) to form the full cosine integral.
    """
    if x <= 0.0:
        raise ValueError("ci_numeric: x must be positive")
    _EULER_MASCHERONI = 0.5772156649015328606
    integral = 0.0
    term = -(x * x) / 2.0  # first term: (−1)^1 · x^2 / (2 · 2!)
    for n in range(1, 200):
        integral += term / (2 * n)
        term *= -(x * x) / ((2 * n + 1) * (2 * n + 2))
        if abs(term) < abs(integral) * 1e-15 + 1e-30:
            break
    return _EULER_MASCHERONI + math.log(x) + integral


def li2_numeric(x: float) -> float:
    """Evaluate Li₂(x) = −∫₀^x log(1−t)/t dt for x < 1.

    Uses the power-series definition:
      Li₂(x) = Σ_{k=1}^∞ x^k / k²   for |x| ≤ 1.

    For x > 1: Li₂(x) = π²/6 − log(x)·log(1−x)??? — actually for
    x > 1 we use the reflection:
      Li₂(x) = π²/3 − Li₂(1/x) − (π²/6 + log²(x)/2)  [nope, complex].

    For simplicity, this implementation only handles x ∈ (−∞, 1).
    x = 1 → π²/6, x = −1 → −π²/12.  For x ≥ 1 raise ValueError.
    """
    if x == 1.0:
        return math.pi ** 2 / 6
    if x == -1.0:
        return -(math.pi ** 2) / 12
    if x > 1.0:
        # Use functional equation: Li₂(x) + Li₂(1/x) = π²/3 − log(x)·(log(x)−iπ)
        # For real x > 1, the dilogarithm is complex.  Return NaN as sentinel.
        return float("nan")
    if abs(x) > 0.5 and 0.0 < x < 1.0:
        # Use the reflection identity for faster convergence:
        # Li₂(x) = π²/6 − log(x)·log(1−x) − Li₂(1−x)  for 0 < x < 1.
        return (
            math.pi ** 2 / 6
            - math.log(x) * math.log(1 - x)
            - li2_numeric(1.0 - x)
        )
    total = 0.0
    term = x
    for k in range(1, 500):
        total += term / (k * k)
        term *= x
        if abs(term) < abs(total) * 1e-15:
            break
    return total


def fresnel_s_numeric(x: float) -> float:
    """Evaluate FresnelS(x) = ∫₀^x sin(πt²/2) dt numerically.

    Power series (integrate sin's Taylor expansion term-by-term):

      sin(πt²/2) = Σ_{n=0}^∞ (−1)^n · (π/2)^(2n+1) · t^(4n+2) / (2n+1)!

    Integrating 0→x gives:

      FresnelS(x) = Σ_{n=0}^∞ (−1)^n · (π/2)^(2n+1) · x^(4n+3) / ((2n+1)! · (4n+3))

    Recurrence ratio term_{n+1}/term_n:
      = (−1) · (π/2)² · x⁴ · (4n+3) / ((2n+2)(2n+3)(4n+7))
    """
    total = 0.0
    x2 = x * x
    x3 = x * x2
    pi_half = math.pi / 2.0
    pi_half_sq = pi_half * pi_half
    # term_n = (−1)^n · (π/2)^(2n+1) · x^(4n+3) / ((2n+1)! · (4n+3))
    t = pi_half * x3 / (1.0 * 3.0)  # n=0: (π/2)^1 · x^3 / (1! · 3)
    for n in range(200):
        total += t
        # Recurrence to next term (n → n+1):
        # ratio = (−1) · (π/2)² · x⁴ · (4n+3) / ((2n+2)(2n+3)(4n+7))
        # The (4n+3) factor comes from the 1/(4n+3) denominator in term_n.
        num = pi_half_sq * x2 * x2 * float(4 * n + 3)
        denom = float((2 * n + 2) * (2 * n + 3) * (4 * n + 7))
        t *= -num / denom
        if abs(t) < abs(total) * 1e-15:
            break
    return total


def fresnel_c_numeric(x: float) -> float:
    """Evaluate FresnelC(x) = ∫₀^x cos(πt²/2) dt numerically.

    Power series (integrate cos's Taylor expansion term-by-term):

      cos(πt²/2) = Σ_{n=0}^∞ (−1)^n · (π/2)^(2n) · t^(4n) / (2n)!

    Integrating 0→x gives:

      FresnelC(x) = Σ_{n=0}^∞ (−1)^n · (π/2)^(2n) · x^(4n+1) / ((2n)! · (4n+1))

    Recurrence ratio term_{n+1}/term_n:
      = (−1) · (π/2)² · x⁴ · (4n+1) / ((2n+1)(2n+2)(4n+5))
    """
    total = 0.0
    x2 = x * x
    pi_half = math.pi / 2.0
    pi_half_sq = pi_half * pi_half
    # term_n: (−1)^n · (π/2)^(2n) · x^(4n+1) / ((2n)! · (4n+1))
    t = x  # n=0: (π/2)^0 · x^1 / (0! · 1) = x
    for n in range(200):
        total += t
        # Recurrence to next term (n → n+1):
        # ratio = (−1) · (π/2)² · x⁴ · (4n+1) / ((2n+1)(2n+2)(4n+5))
        # The (4n+1) factor comes from the 1/(4n+1) denominator in term_n.
        num = pi_half_sq * x2 * x2 * float(4 * n + 1)
        denom = float((2 * n + 1) * (2 * n + 2) * (4 * n + 5))
        t *= -num / denom
        if abs(t) < abs(total) * 1e-15:
            break
    return total


def shi_numeric(x: float) -> float:
    """Evaluate Shi(x) = ∫₀^x sinh(t)/t dt numerically (Maclaurin series)."""
    total = 0.0
    term = x  # n=0: x^1 / (1 · 1!)
    x2 = x * x
    for n in range(200):
        total += term / (2 * n + 1)
        term *= x2 / ((2 * n + 2) * (2 * n + 3))
        if abs(term) < abs(total) * 1e-15:
            break
    return total


def chi_numeric(x: float) -> float:
    """Evaluate Chi(x) = γ + log(x) + ∫₀^x (cosh(t)−1)/t dt for x > 0."""
    if x <= 0.0:
        raise ValueError("chi_numeric: x must be positive")
    _EULER_MASCHERONI = 0.5772156649015328606
    # Series for ∫₀^x (cosh(t)−1)/t dt = Σ_{n=1}^∞ x^(2n) / (2n · (2n)!)
    integral = 0.0
    term = x * x / 2.0  # n=1: x^2 / (2 · 2!)
    x2 = x * x
    for n in range(1, 200):
        integral += term / (2 * n)
        term *= x2 / ((2 * n + 1) * (2 * n + 2))
        if abs(term) < abs(integral) * 1e-15 + 1e-30:
            break
    return _EULER_MASCHERONI + math.log(x) + integral
