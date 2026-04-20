"""Rothstein–Trager — the log part of rational-function integration.

After Hermite reduction (Phase 2c) the integrand is a rational function
``C(x)/E(x)`` with ``E`` squarefree. Its antiderivative is a sum of
logs:

    ∫ C/E dx  =  Σ_i  c_i · log(v_i(x))

This module produces that sum in closed form when every ``c_i`` happens
to lie in Q — which is the case for *every* rational integrand whose
denominator factors into distinct linear factors over Q (i.e. the
"simple partial fractions" class from a calculus textbook). When any
``c_i`` escapes Q (canonical example: ``1/(x² + 1)``, where the
resultant has roots ``±i/2``), we return ``None`` and let the handler
leave the piece as an unevaluated ``Integrate``.

The key trick is Rothstein's theorem: compute the one-variable
polynomial

    R(z)  =  res_x( C(x) − z · E'(x),  E(x) )   ∈  Q[z]

whose distinct roots are the values that appear as log coefficients.
For each rational root ``α`` of ``R``, the log factor is
``v_α = gcd(C(x) − α · E'(x), E(x))``. Rothstein guarantees the
``v_α`` multiply back to ``E`` and the derivative of the resulting log
sum is ``C/E`` — the proof is in Bronstein, *Symbolic Integration I*,
Chapter 2.

The only non-scalar step — the resultant in ``z`` — is handled by
**evaluation + Lagrange interpolation**. We sample the scalar resultant
at ``deg E + 1`` distinct ``z`` values, then interpolate back to a
polynomial in Q[z]. Every internal arithmetic thus stays scalar-over-Q
and the ``polynomial`` package's primitives are enough.

See ``code/specs/rothstein-trager.md`` for the scoped spec.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    add,
    deriv,
    gcd,
    monic,
    multiply,
    normalize,
    rational_roots,
    resultant,
    subtract,
)

LogPart = list[tuple[Fraction, Polynomial]]


def rothstein_trager(num: Polynomial, den: Polynomial) -> LogPart | None:
    """Return the log-part pairs ``[(c_i, v_i)]`` or ``None`` if not all-Q.

    The antiderivative of ``num/den`` — when the return value is a
    list — is ``Σ c_i · log(v_i(x))``. ``v_i`` is returned monic; ``c_i``
    is a ``Fraction``.

    Pre-conditions (enforced by the Hermite caller, not re-checked here):

    - ``den`` is squarefree (Hermite's guarantee).
    - ``deg num < deg den`` (caller has split off the polynomial part).
    - Both polynomials live in Q[x] (``Fraction`` coefficients — or
      anything that coerces through ``Fraction`` via arithmetic).

    The algorithm:

    1. Build ``R(z) = res_x(C − z · E', E) ∈ Q[z]`` by Lagrange
       interpolation from ``deg E + 1`` scalar-resultant samples.
    2. Find ``rational_roots(R)``. If the number of distinct rational
       roots is below the degree of ``R`` after squarefree reduction,
       some root lives outside Q — return ``None``.
    3. For each ``α``, compute ``v_α = gcd(C − α·E', E)`` and record
       ``(α, monic(v_α))``. Discard any degenerate ``v_α`` of degree 0
       (shouldn't happen on correct inputs, but the guard keeps the
       output well-formed).

    The universal correctness gate — used in every unit test — is the
    re-differentiation identity:

        d/dx ( Σ c_i · log(v_i) )  =  Σ c_i · v_i' / v_i  =  num/den.

    Since ``∏ v_i = den``, this reduces to the polynomial identity
    ``Σ c_i · v_i' · ∏_{j≠i} v_j == num`` — no logs appear in the check.
    """
    den_n = normalize(den)
    num_n = normalize(num)

    # Promote everything through Fraction so resultant arithmetic is
    # exact. Hermite already delivers Fraction tuples; the coercion is
    # defensive against ad-hoc callers with plain-int inputs.
    num_q = tuple(Fraction(c) for c in num_n)
    den_q = tuple(Fraction(c) for c in den_n)
    den_prime = deriv(den_q)

    # Step 1: build R(z) via evaluation + Lagrange interpolation.
    r_deg_bound = len(den_q) - 1  # deg R ≤ deg E
    sample_xs = [Fraction(i) for i in range(r_deg_bound + 1)]
    sample_ys = [
        resultant(_sub_z_times(num_q, den_prime, z), den_q) for z in sample_xs
    ]
    r_poly = normalize(_lagrange_interpolate(sample_xs, sample_ys))

    # Step 2: rational roots. Rothstein's theorem makes R's roots the
    # log coefficients; if any root is outside Q we must bail. The
    # check compares ``len(rational_roots(R))`` to the degree of R's
    # squarefree part — short means an irrational root hides among
    # the factors.
    from polynomial import squarefree as _squarefree
    roots = rational_roots(r_poly)
    sqfree_factors = _squarefree(r_poly)
    sqfree_deg = sum(len(f) - 1 for f in sqfree_factors)
    if len(roots) < sqfree_deg:
        return None

    # Step 3: build the log pairs. Rothstein's theorem guarantees each
    # v_α is a non-constant polynomial in Q[x] and the ``v_α`` multiply
    # back to ``monic(E)``, so no degeneracy guard is needed on valid
    # Hermite output.
    pairs: LogPart = []
    for alpha in roots:
        scaled = tuple(alpha * c for c in den_prime)
        shifted = subtract(num_q, scaled)
        v = monic(gcd(shifted, den_q))
        pairs.append((alpha, v))
    return pairs


# ---------------------------------------------------------------------------
# Local helpers
# ---------------------------------------------------------------------------


def _sub_z_times(
    c_poly: Polynomial, e_prime: Polynomial, z: Fraction
) -> Polynomial:
    """Return ``C(x) − z · E'(x)`` as a scalar polynomial in ``x``."""
    scaled = tuple(z * coef for coef in e_prime)
    return subtract(c_poly, scaled)


def _lagrange_interpolate(
    xs: list[Fraction], ys: list[Fraction]
) -> Polynomial:
    """Lagrange interpolation of the points ``(xs[i], ys[i])``.

    Returns the unique polynomial of degree ``≤ n − 1`` (n =
    ``len(xs)``) agreeing with the given values at every node. The
    ``xs`` must be distinct — we don't check, and the caller picks
    ``0, 1, 2, …`` so distinctness is automatic.

    The implementation builds each basis polynomial ``ℓ_i(x)`` as the
    product ``∏_{j≠i} (x − xs[j]) / (xs[i] − xs[j])`` and accumulates
    ``y_i · ℓ_i(x)``. Costs O(n²); fine for the ``n ≤ deg E + 1`` sizes
    RT actually uses.
    """
    n = len(xs)
    result: Polynomial = ()
    for i in range(n):
        xi, yi = xs[i], ys[i]
        # Build the i-th basis polynomial.
        numerator: Polynomial = (Fraction(1),)
        denom = Fraction(1)
        for j in range(n):
            if j == i:
                continue
            # Multiply numerator by (x − xs[j]) = (-xs[j], 1).
            numerator = multiply(numerator, (-xs[j], Fraction(1)))
            denom *= xi - xs[j]
        scale = yi / denom
        scaled_basis = tuple(scale * c for c in numerator)
        result = add(result, scaled_basis)
    return normalize(result)


__all__ = ["rothstein_trager", "LogPart"]
