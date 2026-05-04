"""Inverse Laplace transform table and partial-fraction engine.

The inverse Laplace transform (ILT) recovers f(t) from F(s) by:

1. Representing F(s) as a ratio P(s)/Q(s) of polynomials over Q.
2. Factoring the denominator Q(s) to find its poles.
3. Performing partial-fraction decomposition:
       F(s) = Σ_i  A_i / (s - r_i)^{m_i}
4. Applying the inverse table to each partial fraction term.

Inverse table
-------------
| F(s)                     | f(t) = L⁻¹{F}(s)          |
|--------------------------|----------------------------|
| 1/s                      | UnitStep(t)                |
| 1/(s-a)                  | exp(a·t)                   |
| 1/(s-a)^n  (n ≥ 2)       | t^{n-1}·exp(a·t) / (n-1)! |
| ω/(s²+ω²)                | sin(ω·t)                   |
| s/(s²+ω²)                | cos(ω·t)                   |
| ω/((s-a)²+ω²)            | exp(a·t)·sin(ω·t)          |
| (s-a)/((s-a)²+ω²)        | exp(a·t)·cos(ω·t)          |
| a/(s²-a²)                | sinh(a·t)                  |
| s/(s²-a²)                | cosh(a·t)                  |

Partial-fraction decomposition
-------------------------------
For a proper rational function P(s)/Q(s) with distinct rational poles
r_1, ..., r_n, the residue formula gives:

    A_i = P(r_i) / Q'(r_i)

where Q'(s) is the derivative of Q(s). This is implemented using Python's
``fractions.Fraction`` for exact rational arithmetic.

For the special case of a denominator that contains a simple pole at s=0
(contributing a UnitStep term) plus other poles, the decomposition is
applied uniformly.

Limitations (Phase 1)
---------------------
This implementation handles:
- Proper fractions (deg P < deg Q)
- Denominators with all distinct rational roots (simple poles only)
- The special irreducible quadratic s²+ω² → sin/cos pairs
- The s·/(s²+ω²) → cos form

It does NOT yet handle:
- Repeated complex poles
- Higher-order irreducible quadratics
- Improper fractions (partial polynomial part)
"""

from __future__ import annotations

import math
from fractions import Fraction
from typing import Any

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    EXP,
    MUL,
    POW,
    SIN,
    SINH,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_laplace.heads import UNIT_STEP

# ---------------------------------------------------------------------------
# Polynomial arithmetic over Fraction (rational coefficients)
# ---------------------------------------------------------------------------
#
# Polynomials are represented as tuples of Fraction in ASCENDING degree order:
#   (c_0, c_1, ..., c_n)  represents  c_0 + c_1*s + ... + c_n*s^n
#
# This convention matches the polynomial package used elsewhere in this repo.

Poly = tuple[Fraction, ...]
_ZERO_POLY: Poly = (Fraction(0),)
_ONE_POLY: Poly = (Fraction(1),)


def _poly_degree(p: Poly) -> int:
    """Degree of a polynomial (length - 1)."""
    return len(p) - 1


def _poly_evaluate(p: Poly, x: Fraction) -> Fraction:
    """Evaluate polynomial p at x using Horner's method."""
    result = Fraction(0)
    for coeff in reversed(p):
        result = result * x + coeff
    return result


def _poly_deriv(p: Poly) -> Poly:
    """Formal derivative of polynomial p."""
    if len(p) <= 1:
        return _ZERO_POLY
    return tuple(Fraction(i) * p[i] for i in range(1, len(p)))


def _poly_normalize(p: Poly) -> Poly:
    """Strip trailing zeros from the coefficient tuple."""
    coeffs = list(p)
    while len(coeffs) > 1 and coeffs[-1] == 0:
        coeffs.pop()
    return tuple(coeffs)


def _poly_gcd(a: Poly, b: Poly) -> Poly:
    """GCD of two polynomials over Q using the Euclidean algorithm."""
    a = _poly_normalize(a)
    b = _poly_normalize(b)
    while _poly_normalize(b) != _ZERO_POLY:
        _, r = _poly_divmod(a, b)
        a, b = b, r
    # Make monic
    if a[-1] != 0:
        lead = a[-1]
        a = tuple(c / lead for c in a)
    return _poly_normalize(a)


def _poly_divmod(num: Poly, den: Poly) -> tuple[Poly, Poly]:
    """Polynomial division: returns (quotient, remainder).

    Standard long division of polynomials over Q.
    """
    num = list(_poly_normalize(num))
    den = list(_poly_normalize(den))
    deg_num = len(num) - 1
    deg_den = len(den) - 1

    if deg_num < deg_den:
        return _ZERO_POLY, tuple(num)

    q: list[Fraction] = [Fraction(0)] * (deg_num - deg_den + 1)
    remainder = list(num)

    for i in range(deg_num - deg_den, -1, -1):
        if len(remainder) - 1 >= i + deg_den:
            coeff = remainder[i + deg_den] / den[deg_den]
            q[i] = coeff
            for j in range(deg_den + 1):
                remainder[i + j] -= coeff * den[j]

    return tuple(q), _poly_normalize(tuple(remainder))


def _rational_roots(p: Poly) -> list[Fraction]:
    """Find all rational roots of polynomial p.

    Uses the rational root theorem: any rational root p/q of a polynomial
    with integer coefficients must have p | a_0 (constant term) and q | a_n
    (leading coefficient). We test all such candidates.

    For polynomials with Fraction coefficients, we first clear denominators
    to get an integer-coefficient polynomial, then apply the theorem.
    """
    p = _poly_normalize(p)
    if len(p) == 1:
        return []

    # Clear denominators to get integer coefficients
    lcm_denom = 1
    for coeff in p:
        lcm_denom = lcm_denom * coeff.denominator // math.gcd(
            lcm_denom, coeff.denominator
        )
    int_coeffs = [int(c * lcm_denom) for c in p]

    constant_term = abs(int_coeffs[0])
    leading_coeff = abs(int_coeffs[-1])

    if constant_term == 0:
        # s=0 is a root; divide it out recursively
        # Find multiplicity
        roots = [Fraction(0)]
        # We just return 0 once here; the caller will do root extraction
        return roots

    # Divisors of constant term and leading coefficient
    def _divisors(n: int) -> list[int]:
        n = abs(n)
        if n == 0:
            return [0]
        divs = []
        for i in range(1, int(n**0.5) + 1):
            if n % i == 0:
                divs.append(i)
                if i != n // i:
                    divs.append(n // i)
        return divs

    p_divs = _divisors(constant_term)
    q_divs = _divisors(leading_coeff)

    roots = []
    seen = set()
    for p_val in p_divs:
        for q_val in q_divs:
            for sign in [1, -1]:
                candidate = Fraction(sign * p_val, q_val)
                if candidate not in seen:
                    seen.add(candidate)
                    val = _poly_evaluate(p, candidate)
                    if val == 0:
                        roots.append(candidate)
    return roots


def _extract_all_rational_roots(p: Poly) -> list[Fraction]:
    """Extract ALL rational roots of p (with multiplicity as separate entries).

    Uses repeated division: find a root, divide it out, recurse.
    """
    p = _poly_normalize(p)
    if _poly_degree(p) == 0:
        return []

    roots = []
    remaining = p
    while _poly_degree(remaining) >= 1:
        found = _rational_roots(remaining)
        if not found:
            break
        # Take the first root and extract ALL its copies via repeated division
        root = found[0]
        while _poly_evaluate(remaining, root) == 0 and _poly_degree(remaining) >= 1:
            roots.append(root)
            # Divide by (s - root)
            # (s - root) in ascending order: (-root, 1)
            linear = (-root, Fraction(1))
            remaining, _rem = _poly_divmod(remaining, linear)
            remaining = _poly_normalize(remaining)
    return roots


# ---------------------------------------------------------------------------
# Inverse table: match F(s) patterns, return f(t)
# ---------------------------------------------------------------------------


def _make_exp(a_node: IRNode, t_sym: IRSymbol) -> IRNode:
    """Build exp(a*t) as IR.  If a=1, returns Exp(t)."""
    if isinstance(a_node, IRInteger) and a_node.value == 1:
        return IRApply(EXP, (t_sym,))
    return IRApply(EXP, (IRApply(MUL, (a_node, t_sym)),))


def _frac_to_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to IRInteger or IRRational."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _ilt_simple_pole(
    A: Fraction, a: Fraction, t_sym: IRSymbol
) -> IRNode:
    """Inverse of  A / (s - a):  returns A · exp(a·t).

    Special case: if a=0, returns A · UnitStep(t) (i.e. just the constant A).
    """
    if a == 0:
        # L⁻¹{A/s} = A · UnitStep(t)
        step = IRApply(UNIT_STEP, (t_sym,))
        if A == 1:
            return step
        return IRApply(MUL, (_frac_to_ir(A), step))

    exp_term = _make_exp(_frac_to_ir(a), t_sym)
    if A == 1:
        return exp_term
    if A == -1:
        from symbolic_ir import NEG
        return IRApply(NEG, (exp_term,))
    return IRApply(MUL, (_frac_to_ir(A), exp_term))


def _ilt_repeated_pole(
    A: Fraction, a: Fraction, n: int, t_sym: IRSymbol
) -> IRNode:
    """Inverse of  A / (s - a)^n  for n ≥ 2.

    L⁻¹{A/(s-a)^n} = A · t^{n-1} · exp(a·t) / (n-1)!
    """
    factorial_nm1 = math.factorial(n - 1)
    # Coefficient = A / (n-1)!
    coeff = A / Fraction(factorial_nm1)
    coeff_node = _frac_to_ir(coeff)

    # t^{n-1}
    if n - 1 == 1:
        t_pow: IRNode = t_sym
    else:
        t_pow = IRApply(POW, (t_sym, IRInteger(n - 1)))

    if a == 0:
        # exp(0) = 1, so just A/(n-1)! · t^{n-1}
        if coeff == 1:
            return t_pow
        return IRApply(MUL, (coeff_node, t_pow))

    exp_term = _make_exp(_frac_to_ir(a), t_sym)
    inner = IRApply(MUL, (t_pow, exp_term))

    if coeff == 1:
        return inner
    return IRApply(MUL, (coeff_node, inner))


# ---------------------------------------------------------------------------
# Pattern matching on F(s) for the inverse table (direct forms)
# ---------------------------------------------------------------------------


def _match_inv_one_over_s(
    F: IRNode, s_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = 1/s. Returns {} if matched."""
    if (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
        and isinstance(F.args[0], IRInteger)
        and F.args[0].value == 1
        and isinstance(F.args[1], IRSymbol)
        and F.args[1].name == s_sym.name
    ):
        return {}
    return None


def _match_inv_exp_form(
    F: IRNode, s_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = A/(s - a) for rational A, a. Returns {"A": A, "a": a}."""
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
    ):
        return None
    num, den = F.args
    # A must be a rational literal
    if isinstance(num, IRInteger):
        A_frac = Fraction(num.value)
    elif isinstance(num, IRRational):
        A_frac = Fraction(num.numer, num.denom)
    else:
        return None

    # Denominator must be (s - a) or (s + a) = Sub(s, a) or Add(s, const)
    if (
        isinstance(den, IRSymbol)
        and den.name == s_sym.name
    ):
        # 1/s — handled by _match_inv_one_over_s, but catches here as a=0
        return {"A": A_frac, "a": Fraction(0)}

    if (
        isinstance(den, IRApply)
        and isinstance(den.head, IRSymbol)
        and den.head.name == "Sub"
        and len(den.args) == 2
        and isinstance(den.args[0], IRSymbol)
        and den.args[0].name == s_sym.name
    ):
        a_node = den.args[1]
        if isinstance(a_node, IRInteger):
            return {"A": A_frac, "a": Fraction(a_node.value)}
        if isinstance(a_node, IRRational):
            return {"A": A_frac, "a": Fraction(a_node.numer, a_node.denom)}

    if (
        isinstance(den, IRApply)
        and isinstance(den.head, IRSymbol)
        and den.head.name == "Add"
        and len(den.args) == 2
    ):
        s_node, const_node = den.args
        if not (isinstance(s_node, IRSymbol) and s_node.name == s_sym.name):
            s_node, const_node = const_node, s_node
        if isinstance(s_node, IRSymbol) and s_node.name == s_sym.name:
            if isinstance(const_node, IRInteger):
                # s + b  =  s - (-b)
                return {"A": A_frac, "a": Fraction(-const_node.value)}
            if isinstance(const_node, IRRational):
                return {
                    "A": A_frac,
                    "a": Fraction(-const_node.numer, const_node.denom),
                }
    return None


def _match_inv_trig_form(
    F: IRNode, s_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = omega/(s^2 + omega^2) or F = s/(s^2 + omega^2).

    Returns {"type": "sin", "omega": omega_frac}  or
            {"type": "cos", "omega": omega_frac}.
    """
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
    ):
        return None
    num, den = F.args

    # Denominator must be s^2 + omega^2 = Add(Pow(s, 2), Pow(omega, 2))
    def _is_s2_plus_w2(node: IRNode) -> Fraction | None:
        """Return omega^2 as Fraction if node is s^2 + omega^2, else None."""
        if not (
            isinstance(node, IRApply)
            and isinstance(node.head, IRSymbol)
            and node.head.name == "Add"
            and len(node.args) == 2
        ):
            return None
        # Try both orderings: s^2+w^2 and w^2+s^2
        for s2_node, w2_node in [node.args, reversed(node.args)]:
            if not (
                isinstance(s2_node, IRApply)
                and isinstance(s2_node.head, IRSymbol)
                and s2_node.head.name == "Pow"
                and len(s2_node.args) == 2
                and isinstance(s2_node.args[0], IRSymbol)
                and s2_node.args[0].name == s_sym.name
                and isinstance(s2_node.args[1], IRInteger)
                and s2_node.args[1].value == 2
            ):
                continue
            # w2_node must be a positive rational (omega^2 as a literal)
            # OR Pow(omega_sym, 2) if omega is a constant — but for our
            # table lookups, omega is an IRInteger or IRRational.
            if isinstance(w2_node, IRInteger) and w2_node.value > 0:
                return Fraction(w2_node.value)
            if isinstance(w2_node, IRRational) and w2_node.numer > 0:
                return Fraction(w2_node.numer, w2_node.denom)
            # Pow(omega, 2) form — omega is itself a literal
            if (
                isinstance(w2_node, IRApply)
                and isinstance(w2_node.head, IRSymbol)
                and w2_node.head.name == "Pow"
                and len(w2_node.args) == 2
                and isinstance(w2_node.args[1], IRInteger)
                and w2_node.args[1].value == 2
            ):
                omega_node = w2_node.args[0]
                if isinstance(omega_node, IRInteger):
                    return Fraction(omega_node.value * omega_node.value)
                if isinstance(omega_node, IRRational):
                    return Fraction(omega_node.numer * omega_node.numer,
                                    omega_node.denom * omega_node.denom)
        return None

    w2_frac = _is_s2_plus_w2(den)
    if w2_frac is None:
        return None

    # omega = sqrt(omega^2) — must be a perfect rational square
    # For integer omega^2 = n^2, omega = n
    # We use a quick check: is w2_frac a perfect square of a rational?
    omega_frac = _rational_sqrt(w2_frac)
    if omega_frac is None:
        return None

    # Numerator determines sin vs cos
    if isinstance(num, IRSymbol) and num.name == s_sym.name:
        return {"type": "cos", "omega": omega_frac}
    if isinstance(num, IRInteger):
        num_frac = Fraction(num.value)
        if num_frac == omega_frac:
            return {"type": "sin", "omega": omega_frac}
        # A * omega / (s^2 + omega^2) — scaled sin
        A = num_frac / omega_frac
        return {"type": "sin_scaled", "omega": omega_frac, "A": A}
    if isinstance(num, IRRational):
        num_frac = Fraction(num.numer, num.denom)
        if num_frac == omega_frac:
            return {"type": "sin", "omega": omega_frac}
        A = num_frac / omega_frac
        return {"type": "sin_scaled", "omega": omega_frac, "A": A}
    return None


def _match_inv_hyp_form(
    F: IRNode, s_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = a/(s^2 - a^2) or F = s/(s^2 - a^2).

    Returns {"type": "sinh", "a": a_frac} or {"type": "cosh", "a": a_frac}.
    """
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
    ):
        return None
    num, den = F.args

    # Denominator must be s^2 - a^2 = Sub(Pow(s,2), Pow(a,2))
    def _is_s2_minus_a2(node: IRNode) -> Fraction | None:
        if not (
            isinstance(node, IRApply)
            and isinstance(node.head, IRSymbol)
            and node.head.name == "Sub"
            and len(node.args) == 2
        ):
            return None
        s2_node, a2_node = node.args
        if not (
            isinstance(s2_node, IRApply)
            and isinstance(s2_node.head, IRSymbol)
            and s2_node.head.name == "Pow"
            and len(s2_node.args) == 2
            and isinstance(s2_node.args[0], IRSymbol)
            and s2_node.args[0].name == s_sym.name
            and isinstance(s2_node.args[1], IRInteger)
            and s2_node.args[1].value == 2
        ):
            return None
        if isinstance(a2_node, IRInteger) and a2_node.value > 0:
            return Fraction(a2_node.value)
        if isinstance(a2_node, IRRational) and a2_node.numer > 0:
            return Fraction(a2_node.numer, a2_node.denom)
        if (
            isinstance(a2_node, IRApply)
            and isinstance(a2_node.head, IRSymbol)
            and a2_node.head.name == "Pow"
            and isinstance(a2_node.args[1], IRInteger)
            and a2_node.args[1].value == 2
        ):
            anode = a2_node.args[0]
            if isinstance(anode, IRInteger):
                return Fraction(anode.value * anode.value)
            if isinstance(anode, IRRational):
                return Fraction(anode.numer ** 2, anode.denom ** 2)
        return None

    a2_frac = _is_s2_minus_a2(den)
    if a2_frac is None:
        return None

    a_frac = _rational_sqrt(a2_frac)
    if a_frac is None:
        return None

    if isinstance(num, IRSymbol) and num.name == s_sym.name:
        return {"type": "cosh", "a": a_frac}
    if isinstance(num, IRInteger):
        num_frac = Fraction(num.value)
        if num_frac == a_frac:
            return {"type": "sinh", "a": a_frac}
        # Scaled: A*a/(s^2-a^2)
        A = num_frac / a_frac
        return {"type": "sinh_scaled", "a": a_frac, "A": A}
    if isinstance(num, IRRational):
        num_frac = Fraction(num.numer, num.denom)
        if num_frac == a_frac:
            return {"type": "sinh", "a": a_frac}
        A = num_frac / a_frac
        return {"type": "sinh_scaled", "a": a_frac, "A": A}
    return None


def _rational_sqrt(f: Fraction) -> Fraction | None:
    """Return sqrt(f) as a Fraction if f is a perfect rational square, else None.

    Examples::

        _rational_sqrt(Fraction(4))     → Fraction(2)
        _rational_sqrt(Fraction(1, 4))  → Fraction(1, 2)
        _rational_sqrt(Fraction(2))     → None
    """
    p, q = f.numerator, f.denominator
    sp = _isqrt_exact(p)
    sq = _isqrt_exact(q)
    if sp is not None and sq is not None:
        return Fraction(sp, sq)
    return None


def _isqrt_exact(n: int) -> int | None:
    """Return exact integer square root of n, or None if not a perfect square."""
    if n < 0:
        return None
    if n == 0:
        return 0
    r = math.isqrt(n)
    if r * r == n:
        return r
    return None


# ---------------------------------------------------------------------------
# Main inverse transform routine
# ---------------------------------------------------------------------------


def inverse_laplace(
    F_ir: IRNode,
    s_sym: IRSymbol,
    t_sym: IRSymbol,
) -> IRNode:
    """Compute the inverse Laplace transform of F_ir w.r.t. s_sym.

    Strategy
    --------
    1. Try direct pattern matching (1/s, A/(s-a), omega/(s^2+omega^2), etc.).
    2. If F_ir is a rational function P(s)/Q(s), attempt partial-fraction
       decomposition followed by inverse table lookup on each term.
    3. Fall through to the unevaluated ILT(F, s, t) form.

    Parameters
    ----------
    F_ir:
        The IR expression for F(s) — the function to invert.
    s_sym:
        The complex frequency variable.
    t_sym:
        The time variable.

    Returns
    -------
    IRNode
        The inverse transform f(t), or the unevaluated ILT(F, s, t).
    """
    from cas_laplace.heads import ILT

    # ------------------------------------------------------------------
    # Step 1: Direct pattern matching for well-known forms.
    # ------------------------------------------------------------------

    # 1/s → UnitStep(t)
    if _match_inv_one_over_s(F_ir, s_sym) is not None:
        return IRApply(UNIT_STEP, (t_sym,))

    # A/(s-a) → A*exp(at)
    exp_match = _match_inv_exp_form(F_ir, s_sym)
    if exp_match is not None:
        return _ilt_simple_pole(exp_match["A"], exp_match["a"], t_sym)

    # omega/(s^2+omega^2) → sin(omega*t),  s/(s^2+omega^2) → cos(omega*t)
    trig_match = _match_inv_trig_form(F_ir, s_sym)
    if trig_match is not None:
        return _ilt_from_trig_match(trig_match, t_sym)

    # a/(s^2-a^2) → sinh(at),  s/(s^2-a^2) → cosh(at)
    hyp_match = _match_inv_hyp_form(F_ir, s_sym)
    if hyp_match is not None:
        return _ilt_from_hyp_match(hyp_match, t_sym)

    # ------------------------------------------------------------------
    # Step 2: Partial fraction decomposition.
    # ------------------------------------------------------------------
    result = _ilt_via_partial_fractions(F_ir, s_sym, t_sym)
    if result is not None:
        return result

    # ------------------------------------------------------------------
    # Step 3: Fall through.
    # ------------------------------------------------------------------
    return IRApply(ILT, (F_ir, s_sym, t_sym))


def _ilt_from_trig_match(m: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """Build f(t) from a matched trig-form F(s)."""
    omega_frac = m["type_omega"] if "type_omega" in m else m["omega"]
    omega_frac = m["omega"]
    omega_node = _frac_to_ir(omega_frac)
    if m["type"] == "sin":
        # sin(omega*t)
        if omega_frac == 1:
            arg: IRNode = t_sym
        else:
            arg = IRApply(MUL, (omega_node, t_sym))
        return IRApply(SIN, (arg,))
    if m["type"] == "sin_scaled":
        # A * sin(omega*t)
        A_node = _frac_to_ir(m["A"])
        arg = t_sym if omega_frac == 1 else IRApply(MUL, (omega_node, t_sym))
        sin_term = IRApply(SIN, (arg,))
        if m["A"] == 1:
            return sin_term
        return IRApply(MUL, (A_node, sin_term))
    # cos
    arg = t_sym if omega_frac == 1 else IRApply(MUL, (omega_node, t_sym))
    return IRApply(COS, (arg,))


def _ilt_from_hyp_match(m: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """Build f(t) from a matched hyperbolic-form F(s)."""
    a_frac = m["a"]
    a_node = _frac_to_ir(a_frac)
    if m["type"] == "sinh":
        if a_frac == 1:
            arg: IRNode = t_sym
        else:
            arg = IRApply(MUL, (a_node, t_sym))
        return IRApply(SINH, (arg,))
    if m["type"] == "sinh_scaled":
        A_node = _frac_to_ir(m["A"])
        arg = t_sym if a_frac == 1 else IRApply(MUL, (a_node, t_sym))
        sinh_term = IRApply(SINH, (arg,))
        if m["A"] == 1:
            return sinh_term
        return IRApply(MUL, (A_node, sinh_term))
    # cosh
    arg = t_sym if a_frac == 1 else IRApply(MUL, (a_node, t_sym))
    return IRApply(COSH, (arg,))


# ---------------------------------------------------------------------------
# Polynomial representation of rational functions from IR
# ---------------------------------------------------------------------------


def _ir_to_rational(
    node: IRNode, s_sym: IRSymbol
) -> tuple[Poly, Poly] | None:
    """Convert an IR node to (numerator_poly, denominator_poly) over Fraction.

    Returns None if the node is not representable as a ratio of polynomials
    in s with rational coefficients.

    Supported forms:
    - IRInteger, IRRational → constant polynomials
    - IRSymbol(s) → polynomial (0, 1) = s
    - IRApply(Pow, (s, n)) → s^n
    - IRApply(Add, (a, b))
    - IRApply(Sub, (a, b))
    - IRApply(Mul, (a, b))
    - IRApply(Div, (a, b))
    - IRApply(Neg, (a,))
    """
    if isinstance(node, IRInteger):
        return (Fraction(node.value),), _ONE_POLY
    if isinstance(node, IRRational):
        return (Fraction(node.numer, node.denom),), _ONE_POLY
    if isinstance(node, IRSymbol):
        if node.name == s_sym.name:
            # s = 0 + 1*s
            return (Fraction(0), Fraction(1)), _ONE_POLY
        # Unknown symbol — not representable as a polynomial in s
        return None

    if not isinstance(node, IRApply):
        return None
    if not isinstance(node.head, IRSymbol):
        return None
    head_name = node.head.name

    if head_name == "Add" and len(node.args) == 2:
        r1 = _ir_to_rational(node.args[0], s_sym)
        r2 = _ir_to_rational(node.args[1], s_sym)
        if r1 is None or r2 is None:
            return None
        n1, d1 = r1
        n2, d2 = r2
        # (n1/d1) + (n2/d2) = (n1*d2 + n2*d1) / (d1*d2)
        num = _poly_add(_poly_mul(n1, d2), _poly_mul(n2, d1))
        den = _poly_mul(d1, d2)
        return _poly_normalize(num), _poly_normalize(den)

    if head_name == "Sub" and len(node.args) == 2:
        r1 = _ir_to_rational(node.args[0], s_sym)
        r2 = _ir_to_rational(node.args[1], s_sym)
        if r1 is None or r2 is None:
            return None
        n1, d1 = r1
        n2, d2 = r2
        num = _poly_add(_poly_mul(n1, d2), _poly_mul(_poly_neg(n2), d1))
        den = _poly_mul(d1, d2)
        return _poly_normalize(num), _poly_normalize(den)

    if head_name == "Mul" and len(node.args) == 2:
        r1 = _ir_to_rational(node.args[0], s_sym)
        r2 = _ir_to_rational(node.args[1], s_sym)
        if r1 is None or r2 is None:
            return None
        n1, d1 = r1
        n2, d2 = r2
        return _poly_normalize(_poly_mul(n1, n2)), _poly_normalize(_poly_mul(d1, d2))

    if head_name == "Div" and len(node.args) == 2:
        r1 = _ir_to_rational(node.args[0], s_sym)
        r2 = _ir_to_rational(node.args[1], s_sym)
        if r1 is None or r2 is None:
            return None
        n1, d1 = r1
        n2, d2 = r2
        # (n1/d1) / (n2/d2) = (n1*d2) / (d1*n2)
        return _poly_normalize(_poly_mul(n1, d2)), _poly_normalize(_poly_mul(d1, n2))

    if head_name == "Neg" and len(node.args) == 1:
        r1 = _ir_to_rational(node.args[0], s_sym)
        if r1 is None:
            return None
        n1, d1 = r1
        return _poly_normalize(_poly_neg(n1)), d1

    if head_name == "Pow" and len(node.args) == 2:
        base = node.args[0]
        exp = node.args[1]
        if not isinstance(exp, IRInteger):
            return None
        n_exp = exp.value
        if n_exp < 0:
            # s^{-n} = 1/s^n → num=1, den=s^n
            r1 = _ir_to_rational(base, s_sym)
            if r1 is None:
                return None
            n1, d1 = r1
            pos_n = -n_exp
            num = _poly_pow(d1, pos_n)
            den = _poly_pow(n1, pos_n)
            return _poly_normalize(num), _poly_normalize(den)
        r1 = _ir_to_rational(base, s_sym)
        if r1 is None:
            return None
        n1, d1 = r1
        return (
            _poly_normalize(_poly_pow(n1, n_exp)),
            _poly_normalize(_poly_pow(d1, n_exp)),
        )

    return None


def _poly_add(a: Poly, b: Poly) -> Poly:
    """Add two polynomials, padding with zeros as needed."""
    n = max(len(a), len(b))
    result = []
    for i in range(n):
        ca = a[i] if i < len(a) else Fraction(0)
        cb = b[i] if i < len(b) else Fraction(0)
        result.append(ca + cb)
    return tuple(result)


def _poly_neg(p: Poly) -> Poly:
    """Negate a polynomial."""
    return tuple(-c for c in p)


def _poly_mul(a: Poly, b: Poly) -> Poly:
    """Multiply two polynomials (standard convolution)."""
    result = [Fraction(0)] * (len(a) + len(b) - 1)
    for i, ca in enumerate(a):
        for j, cb in enumerate(b):
            result[i + j] += ca * cb
    return tuple(result)


def _poly_pow(p: Poly, n: int) -> Poly:
    """Raise polynomial p to power n (repeated squaring)."""
    if n == 0:
        return _ONE_POLY
    if n == 1:
        return p
    result = _ONE_POLY
    base = p
    while n > 0:
        if n & 1:
            result = _poly_mul(result, base)
        base = _poly_mul(base, base)
        n >>= 1
    return result


def _poly_scale(p: Poly, c: Fraction) -> Poly:
    """Multiply every coefficient of p by c."""
    return tuple(coeff * c for coeff in p)


# ---------------------------------------------------------------------------
# Partial-fraction decomposition
# ---------------------------------------------------------------------------


def _partial_fractions_simple_poles(
    num: Poly, den: Poly, roots: list[Fraction]
) -> list[tuple[Fraction, Fraction, int]] | None:
    """Compute partial-fraction residues for a proper rational function.

    All poles must be simple (distinct roots). Returns a list of
    (A_i, r_i, 1) tuples meaning A_i / (s - r_i).

    Uses the residue formula:  A_i = P(r_i) / Q'(r_i).
    """
    den_deriv = _poly_deriv(den)
    terms: list[tuple[Fraction, Fraction, int]] = []
    for r in roots:
        p_at_r = _poly_evaluate(num, r)
        qd_at_r = _poly_evaluate(den_deriv, r)
        if qd_at_r == 0:
            return None  # repeated root
        A = p_at_r / qd_at_r
        terms.append((A, r, 1))
    return terms


def _ilt_via_partial_fractions(
    F_ir: IRNode, s_sym: IRSymbol, t_sym: IRSymbol
) -> IRNode | None:
    """Attempt ILT via partial-fraction decomposition.

    Returns the sum of inverse transforms, or None if decomposition fails.
    """
    rational = _ir_to_rational(F_ir, s_sym)
    if rational is None:
        return None

    num, den = rational
    num = _poly_normalize(num)
    den = _poly_normalize(den)

    # Must be a proper fraction
    if _poly_degree(num) >= _poly_degree(den):
        return None

    # Find all rational roots of the denominator
    roots = _extract_all_rational_roots(den)

    # Roots must account for ALL factors (i.e., we need exactly deg(den) roots)
    if len(roots) != _poly_degree(den):
        return None  # Irreducible factors remain

    # Compute residues
    terms = _partial_fractions_simple_poles(num, den, roots)
    if terms is None:
        return None

    # Build inverse transform for each term
    ir_terms: list[IRNode] = []
    for A, r, mult in terms:
        if mult == 1:
            ir_terms.append(_ilt_simple_pole(A, r, t_sym))
        else:
            ir_terms.append(_ilt_repeated_pole(A, r, mult, t_sym))

    if not ir_terms:
        return IRInteger(0)
    if len(ir_terms) == 1:
        return ir_terms[0]

    # Sum all terms
    result = ir_terms[0]
    for term in ir_terms[1:]:
        result = IRApply(ADD, (result, term))
    return result
