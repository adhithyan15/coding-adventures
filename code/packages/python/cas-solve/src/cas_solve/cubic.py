"""Cubic-equation closed form using rational roots + Cardano's formula.

For ``a x³ + b x² + c x + d = 0`` with rational coefficients:

1. **Rational-root theorem**: candidate roots are ±p/q where p | |d| and
   q | |a| (both scaled to integers).  When a rational root ``r`` is found,
   the cubic is deflated to a quadratic and solved via
   :func:`~cas_solve.quadratic.solve_quadratic`.

2. **Cardano's formula** (fallback when no rational root exists): depress
   the cubic to ``t³ + pt + q = 0`` via ``x = t − b/(3a)``, then:

   - ``D_cardano = q²/4 + p³/27``.
   - ``D_cardano > 0``: one real root + two complex-conjugate roots,
     expressed using ``Sqrt``, ``Cbrt``, and ``ImaginaryUnit`` IR heads.
   - ``D_cardano = 0``: repeated-root case (two distinct roots).
   - ``D_cardano < 0``: *casus irreducibilis* — three distinct real roots
     that cannot be expressed with real nested radicals. Returns ``[]``
     (caller should treat the equation as unevaluated).
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    DIV,
    MUL,
    NEG,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_solve.quadratic import solve_quadratic

# Cube-root head — not in standard symbolic_ir yet; treated as an
# unknown head by the VM (returns unevaluated) until a handler is wired.
CBRT = IRSymbol("Cbrt")

# Imaginary unit — shared with quadratic.py convention.
I_UNIT = IRSymbol("%i")

# sqrt(3) — used in complex-root expression for Cardano.
_SQRT3: IRNode = IRApply(SQRT, (IRInteger(3),))


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def solve_cubic(
    a: Fraction, b: Fraction, c: Fraction, d: Fraction
) -> list[IRNode] | str:
    """Solve ``a x³ + b x² + c x + d = 0`` over Q (with complex if needed).

    Returns:
        - A list of IR roots (1–3 elements).
        - An empty list if the cubic has three irrational real roots
          (casus irreducibilis) — the caller should treat the expression
          as unevaluated.
        - ``"ALL"`` is not possible for a cubic (degree ≥ 1).

    Roots may include ``%i`` (the imaginary unit) for complex pairs.
    """
    if a == 0:
        return solve_quadratic(b, c, d)

    # ------------------------------------------------------------------
    # Step 1: rational root theorem
    # ------------------------------------------------------------------
    r = _find_rational_root(a, b, c, d)
    if r is not None:
        # Deflate by (x - r): synthetic division gives ax² + b2x + c2.
        b2 = b + a * r
        c2 = c + b2 * r
        # Verification: d + c2 * r == 0 (guard against float errors)
        remainder = d + c2 * r
        if remainder != 0:
            # Floating-point artefact — discard candidate.
            pass
        else:
            remaining = solve_quadratic(a, b2, c2)
            r_ir = _fraction_to_ir(r)
            if isinstance(remaining, str):
                # "ALL" — shouldn't happen for a quadratic residual but guard.
                return [r_ir]
            return _dedup_and_sort([r_ir] + remaining)

    # ------------------------------------------------------------------
    # Step 2: Cardano's formula — depress the cubic
    # ------------------------------------------------------------------
    # Substitute x = t - b/(3a):
    # p = c/a - b²/(3a²)
    # q = d/a - b·c/(3a²) + 2b³/(27a³)
    a_inv = Fraction(1, 1) / a
    p = c * a_inv - b * b * a_inv * a_inv / 3
    q = d * a_inv - b * c * a_inv * a_inv / 3 + 2 * b ** 3 * a_inv ** 3 / 27

    shift = -b / (3 * a)  # x = t + shift

    # Cardano discriminant: D = q²/4 + p³/27
    d_card = q * q / 4 + p ** 3 / 27

    if d_card > 0:
        return _cardano_one_real_two_complex(p, q, shift, d_card)

    if d_card == 0:
        return _cardano_repeated(p, q, shift)

    # d_card < 0: casus irreducibilis (3 distinct real roots, no real radical form)
    return []


# ---------------------------------------------------------------------------
# Cardano helpers
# ---------------------------------------------------------------------------


def _cardano_one_real_two_complex(
    p: Fraction, q: Fraction, shift: Fraction, d_card: Fraction
) -> list[IRNode]:
    """Cardano formula for D > 0: one real + two complex conjugate roots.

    Depressed cubic ``t³ + pt + q = 0`` with Cardano discriminant
    ``D = q²/4 + p³/27 > 0``.

    Real root:     ``t₁ = cbrt(A) + cbrt(B)``
    Complex roots: ``t₂,₃ = -(cbrt(A) + cbrt(B))/2 ± (cbrt(A) - cbrt(B))·√3/2·i``

    where ``A = -q/2 + sqrt(D)``, ``B = -q/2 - sqrt(D)``.
    """
    neg_q_half = -q / 2

    # Try exact computation if D is a perfect square and cube-roots are rational.
    sqrt_d = _try_exact_sqrt(d_card)
    if isinstance(sqrt_d, Fraction):
        # A and B are rational; check whether cube roots are rational.
        A = neg_q_half + sqrt_d
        B = neg_q_half - sqrt_d
        cbrt_A = _try_exact_cbrt(A)
        cbrt_B = _try_exact_cbrt(B)
        if cbrt_A is not None and cbrt_B is not None:
            t1 = cbrt_A + cbrt_B
            x1 = t1 + shift
            # Complex pair
            half_sum = -(cbrt_A + cbrt_B) / 2
            half_diff = (cbrt_A - cbrt_B) / 2
            roots: list[IRNode] = [_fraction_to_ir(x1)]
            if half_diff == 0:
                # Imaginary part is zero — three real roots (repeated?).
                roots.append(_fraction_to_ir(half_sum + shift))
                roots.append(_fraction_to_ir(half_sum + shift))
            else:
                # Complex conjugate pair
                real_part = _fraction_to_ir(half_sum + shift)
                imag_coef = half_diff
                imag_ir = _imag_term(imag_coef)
                roots.append(IRApply(ADD, (real_part, imag_ir)))
                roots.append(IRApply(SUB, (real_part, imag_ir)))
            return roots

    # Symbolic Cardano: build IR expressions.
    sqrt_d_ir = _sqrt_ir(d_card)
    neg_q_half_ir = _fraction_to_ir(neg_q_half)

    if neg_q_half == 0:
        cbrt_A_ir: IRNode = IRApply(CBRT, (sqrt_d_ir,))
        cbrt_B_ir: IRNode = IRApply(CBRT, (IRApply(NEG, (sqrt_d_ir,)),))
    else:
        cbrt_A_ir = IRApply(CBRT, (IRApply(ADD, (neg_q_half_ir, sqrt_d_ir)),))
        cbrt_B_ir = IRApply(CBRT, (IRApply(SUB, (neg_q_half_ir, sqrt_d_ir)),))

    # t₁ = cbrt(A) + cbrt(B)
    t1_ir: IRNode = IRApply(ADD, (cbrt_A_ir, cbrt_B_ir))
    x1_ir = _add_shift(t1_ir, shift)

    # t₂,₃ = -(t₁)/2 ± (cbrt(A) - cbrt(B))·√3/2·i
    minus_t1_half: IRNode = IRApply(
        DIV, (IRApply(NEG, (IRApply(ADD, (cbrt_A_ir, cbrt_B_ir)),)), IRInteger(2))
    )
    diff_ir: IRNode = IRApply(SUB, (cbrt_A_ir, cbrt_B_ir))
    # (cbrt(A) - cbrt(B)) * sqrt(3) / 2 * i
    imag_part_ir: IRNode = IRApply(
        MUL,
        (
            IRApply(
                DIV,
                (IRApply(MUL, (diff_ir, _SQRT3)), IRInteger(2)),
            ),
            I_UNIT,
        ),
    )
    real_part_ir = _add_shift(minus_t1_half, shift)
    x2_ir: IRNode = IRApply(ADD, (real_part_ir, imag_part_ir))
    x3_ir: IRNode = IRApply(SUB, (real_part_ir, imag_part_ir))

    return [x1_ir, x2_ir, x3_ir]


def _cardano_repeated(
    p: Fraction, q: Fraction, shift: Fraction
) -> list[IRNode]:
    """Cardano formula for D = 0: repeated root(s).

    If ``p = q = 0``: triple root at ``x = shift``.
    Otherwise: a simple root at ``2·cbrt(-q/2)`` and a double root at
    ``-cbrt(-q/2)``, both shifted by ``shift``.
    """
    if p == 0 and q == 0:
        return [_fraction_to_ir(shift)]

    neg_q_half = -q / 2
    cbrt_val = _try_exact_cbrt(neg_q_half)
    if cbrt_val is not None:
        t1 = 2 * cbrt_val
        t2 = -cbrt_val
        x1 = t1 + shift
        x2 = t2 + shift
        return _dedup_and_sort([_fraction_to_ir(x1), _fraction_to_ir(x2)])

    # Symbolic fallback
    neg_q_half_ir = _fraction_to_ir(neg_q_half)
    cbrt_ir: IRNode = IRApply(CBRT, (neg_q_half_ir,))
    t1_ir: IRNode = IRApply(MUL, (IRInteger(2), cbrt_ir))
    t2_ir: IRNode = IRApply(NEG, (cbrt_ir,))
    return [_add_shift(t1_ir, shift), _add_shift(t2_ir, shift)]


# ---------------------------------------------------------------------------
# Rational root search
# ---------------------------------------------------------------------------


def _find_rational_root(
    a: Fraction, b: Fraction, c: Fraction, d: Fraction
) -> Fraction | None:
    """Return a rational root of ``ax³ + bx² + cx + d`` if one exists.

    Uses the rational root theorem: candidates are ±p/q where p | d_int
    and q | a_int (with both scaled to minimal integers).
    Returns ``None`` if no rational root is found.
    """
    # Scale all coefficients so a is a positive integer, preserving roots.
    lcm = _fraction_lcm(a, b, c, d)
    A = int(a * lcm)
    D = int(d * lcm)

    if D == 0:
        # 0 is always a root.
        return Fraction(0)

    p_divs = _divisors(abs(D))
    q_divs = _divisors(abs(A))

    for p_val in p_divs:
        for q_val in q_divs:
            for sign in (1, -1):
                r = Fraction(sign * p_val, q_val)
                if _eval_cubic(a, b, c, d, r) == 0:
                    return r
    return None


def _eval_cubic(
    a: Fraction, b: Fraction, c: Fraction, d: Fraction, x: Fraction
) -> Fraction:
    """Evaluate ``ax³ + bx² + cx + d`` at ``x``."""
    return a * x**3 + b * x**2 + c * x + d


def _divisors(n: int) -> list[int]:
    """Return all positive divisors of n (for n ≥ 0)."""
    if n == 0:
        return [0]
    divs: list[int] = []
    i = 1
    while i * i <= n:
        if n % i == 0:
            divs.append(i)
            if i != n // i:
                divs.append(n // i)
        i += 1
    return sorted(divs)


def _fraction_lcm(*fracs: Fraction) -> int:
    """LCM of denominators of a sequence of fractions, for scaling."""
    result = 1
    for f in fracs:
        d = f.denominator
        result = result * d // _gcd(result, d)
    return result


def _gcd(a: int, b: int) -> int:
    while b:
        a, b = b, a % b
    return a


# ---------------------------------------------------------------------------
# Exact arithmetic helpers
# ---------------------------------------------------------------------------


def _try_exact_sqrt(value: Fraction) -> Fraction | None:
    """Return the exact rational square root of ``value`` if it exists."""
    if value < 0:
        return None
    num, den = value.numerator, value.denominator
    rn = _isqrt(num)
    rd = _isqrt(den)
    if rn is not None and rd is not None:
        return Fraction(rn, rd)
    return None


def _try_exact_cbrt(value: Fraction) -> Fraction | None:
    """Return the exact rational cube root of ``value`` if it exists."""
    if value == 0:
        return Fraction(0)
    sign = 1 if value > 0 else -1
    abs_val = abs(value)
    num, den = abs_val.numerator, abs_val.denominator
    rn = _icbrt(num)
    rd = _icbrt(den)
    if rn is not None and rd is not None:
        return Fraction(sign * rn, rd)
    return None


def _isqrt(n: int) -> int | None:
    """Integer square root if ``n`` is a perfect square."""
    if n < 0:
        return None
    r = int(n**0.5)
    for cand in (r - 1, r, r + 1):
        if cand >= 0 and cand * cand == n:
            return cand
    return None


def _icbrt(n: int) -> int | None:
    """Integer cube root if ``n`` is a perfect cube."""
    if n < 0:
        return None
    if n == 0:
        return 0
    r = round(n ** (1.0 / 3))
    for cand in (r - 1, r, r + 1):
        if cand >= 0 and cand * cand * cand == n:
            return cand
    return None


# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _fraction_to_ir(f: Fraction) -> IRNode:
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _sqrt_ir(value: Fraction) -> IRNode:
    """Build ``Sqrt(value)`` as IR, trying to simplify."""
    exact = _try_exact_sqrt(value)
    if exact is not None:
        return _fraction_to_ir(exact)
    return IRApply(SQRT, (_fraction_to_ir(value),))


def _imag_term(coef: Fraction) -> IRNode:
    """Build ``coef * %i`` as IR (simplified when coef is ±1)."""
    if coef == 1:
        return I_UNIT
    if coef == -1:
        return IRApply(NEG, (I_UNIT,))
    return IRApply(MUL, (_fraction_to_ir(coef), I_UNIT))


def _add_shift(node: IRNode, shift: Fraction) -> IRNode:
    """Add a rational shift to an IR node. No-op when shift is zero."""
    if shift == 0:
        return node
    shift_ir = _fraction_to_ir(shift)
    if isinstance(shift_ir, IRInteger) and shift_ir.value < 0:
        return IRApply(SUB, (node, IRInteger(-shift_ir.value)))
    return IRApply(ADD, (node, shift_ir))


def _dedup_and_sort(roots: list[IRNode]) -> list[IRNode]:
    """Return roots with duplicates removed (by equality check), order preserved."""
    seen: list[IRNode] = []
    for r in roots:
        if r not in seen:
            seen.append(r)
    return seen
