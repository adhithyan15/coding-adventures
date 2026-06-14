"""Durand–Kerner (Weierstrass) method for numeric polynomial root-finding.

Given a monic polynomial of degree ≥ 1 (coefficients as ``float``), the
algorithm simultaneously iterates all roots toward convergence.

Algorithm
---------
The Weierstrass / Durand-Kerner iteration starts with well-spread initial
guesses on a circle of radius ``r ≈ 1 + max|aₖ/a_n|^(1/(n-k))`` and
updates each approximation::

    z_i ← z_i − p(z_i) / ∏_{j ≠ i} (z_i − z_j)

Convergence is quadratic (like Newton) globally from the circle start.

Public API
----------
``nsolve_poly(coeffs, max_iter, tol)``
    Accepts a list of *monic* polynomial coefficients in **decreasing
    degree** order (i.e. ``[1, a_{n-1}, ..., a_1, a_0]``) and returns a
    list of ``complex`` roots.
"""

from __future__ import annotations

import cmath
import math
from fractions import Fraction

from symbolic_ir import IRApply, IRFloat, IRNode, IRSymbol

LIST_SYMBOL = IRSymbol("List")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def nsolve_poly(
    coeffs: list[float | complex],
    max_iter: int = 200,
    tol: float = 1e-12,
) -> list[complex]:
    """Find all roots of a monic polynomial numerically.

    Parameters
    ----------
    coeffs:
        Polynomial coefficients in *decreasing* degree order.  The leading
        coefficient must be non-zero; it is normalised to 1 internally.
    max_iter:
        Maximum number of Durand-Kerner iterations.
    tol:
        Convergence tolerance: iteration stops when all |Δz_i| < ``tol``.

    Returns
    -------
    A list of ``complex`` roots (unsorted).
    """
    # Degree
    n = len(coeffs) - 1
    if n <= 0:
        return []

    # Normalise: divide by leading coefficient
    lead = complex(coeffs[0])
    poly = [complex(c) / lead for c in coeffs]

    # Initial guesses on a circle of appropriate radius
    # Use the Cauchy bound: r = 1 + max(|a_k / a_n|) for k < n
    if n == 1:
        # Linear: one root directly
        return [-poly[1] / poly[0] if poly[0] != 0 else complex(0)]

    radius = _initial_radius(poly)
    # Spread n roots evenly on circle(0, radius) with a slight phase offset
    # to avoid symmetry issues
    phase = complex(0.4, 0.9)  # off-unit-circle seed
    z = [radius * (phase ** k) / abs(phase ** k) for k in range(n)]
    # Re-scale to lie on the circle
    z = [radius * cmath.exp(2j * cmath.pi * k / n + 0.1j) for k in range(n)]

    for _ in range(max_iter):
        max_delta = 0.0
        new_z = list(z)
        for i in range(n):
            pval = _eval_poly(poly, z[i])
            denom = complex(1)
            for j in range(n):
                if j != i:
                    diff = z[i] - z[j]
                    if abs(diff) < 1e-300:
                        diff = complex(1e-300)
                    denom *= diff
            delta = pval / denom
            new_z[i] = z[i] - delta
            max_delta = max(max_delta, abs(delta))
        z = new_z
        if max_delta < tol:
            break

    return z


def roots_to_ir(roots: list[complex]) -> list[IRNode]:
    """Convert numeric complex roots to IR nodes.

    Pure-real roots (imaginary part < ``1e-10``) become ``IRFloat(real)``.
    Complex roots become ``IRFloat(real)`` + ``IRFloat(imag)`` pairs wrapped
    in an ``Add(re, Mul(im, %i))`` structure — but for simplicity we just
    return all as ``IRFloat`` real values or tagged complex IRApply nodes.

    For the purpose of ``NSolve`` output we return each root as a single
    ``IRFloat(real)`` if nearly real, or an ``IRApply(Add, (IRFloat(re),
    IRApply(Mul, (IRFloat(im), %i))))`` for complex.
    """
    I = IRSymbol("%i")
    ADD = IRSymbol("Add")
    MUL = IRSymbol("Mul")
    result: list[IRNode] = []
    for z in roots:
        re = z.real
        im = z.imag
        if abs(im) < 1e-10:
            result.append(IRFloat(re))
        else:
            imag_part: IRNode = IRApply(MUL, (IRFloat(im), I))
            result.append(IRApply(ADD, (IRFloat(re), imag_part)))
    return result


def nsolve_fraction_poly(
    coeffs_frac: list[Fraction],
) -> list[IRNode]:
    """Convenience wrapper: accepts ``Fraction`` coefficients (high→low degree),
    runs ``nsolve_poly``, and returns IR nodes."""
    float_coeffs = [float(c) for c in coeffs_frac]
    roots = nsolve_poly(float_coeffs)
    return roots_to_ir(roots)


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _eval_poly(poly: list[complex], z: complex) -> complex:
    """Evaluate polynomial at ``z`` (Horner's method)."""
    val = complex(0)
    for c in poly:
        val = val * z + c
    return val


def _initial_radius(poly: list[complex]) -> float:
    """Estimate a good circle radius for initial guesses.

    Uses the Cauchy bound: radius ≤ 1 + max|a_k| for monic polynomial
    with leading 1.  A tighter estimate uses n-th root of the constant term.
    """
    n = len(poly) - 1
    if n <= 0:
        return 1.0
    # Cauchy bound
    cauchy = 1.0 + max(abs(poly[k]) for k in range(1, n + 1))
    # Nth-root-of-constant bound (Lagrange)
    if abs(poly[-1]) > 1e-300:
        lagrange = abs(poly[-1]) ** (1.0 / n)
    else:
        lagrange = 1.0
    return max(min(cauchy, 10.0), lagrange, 0.5)
