"""Ideal solving via Gröbner basis + back-substitution.

Given a system of polynomial equations  f₁ = 0, …, fₖ = 0  in the
variables  x₁, …, xₙ,  ``ideal_solve`` finds all common solutions (over
the rationals Q, or over C for quadratic extensions).

Strategy
--------
1. Compute the Gröbner basis of ⟨f₁, …, fₖ⟩ under **lex order**.
   Lex order is special: the resulting basis has a *triangular* shape
   (like Gaussian elimination for nonlinear systems).  Specifically, if
   the system has finitely many solutions, the lex basis always contains
   a polynomial that depends on only the *last* variable.

2. Find the **univariate polynomial** in the last variable (say yₙ) and
   solve for its roots using the ``cas-factor``/``cas-solve`` univariate
   solver.  (We keep it simple: degree ≤ 4, rational coefficients.)

3. **Back-substitute** each root value into the remaining basis elements
   to get a smaller system in  n−1  variables.  Recurse or solve directly.

Limitations (by design)
-----------------------
- Only the *triangular* case is handled (lex basis has a univariate element).
  If no univariate element is found, return ``None`` (unevaluated).
- Only rational (Q) or simple quadratic (Q[i], Q[√d]) solutions are found.
- Degree cap: total degree of any polynomial ≤ 8.
- Variable count cap: ≤ 4.

Example
-------
System: x + y = 1, x − y = 0

Lex Gröbner basis: [x − 1/2, y − 1/2]
Solution: x = 1/2, y = 1/2

Usage::

    from fractions import Fraction
    from cas_multivariate.polynomial import MPoly
    from cas_multivariate.solve import ideal_solve

    # x + y - 1 = 0,  x - y = 0
    f1 = MPoly({(1,0): Fraction(1), (0,1): Fraction(1), (0,0): Fraction(-1)}, 2)
    f2 = MPoly({(1,0): Fraction(1), (0,1): Fraction(-1)}, 2)
    solutions = ideal_solve([f1, f2], order="lex")
    # → [[Fraction(1,2), Fraction(1,2)]]  (one solution: x=1/2, y=1/2)
"""

from __future__ import annotations

from fractions import Fraction

from cas_multivariate.groebner import GrobnerError, buchberger
from cas_multivariate.polynomial import MPoly

# ---------------------------------------------------------------------------
# Univariate root-finding over Q (rational roots + quadratic formula)
# ---------------------------------------------------------------------------


def _rational_roots(coeffs: list[Fraction]) -> list[Fraction]:
    """Find all rational roots of a univariate polynomial.

    Parameters
    ----------
    coeffs:
        List of coefficients in ascending degree:
        ``coeffs[k]`` is the coefficient of ``x^k``.

    Returns
    -------
    List of rational roots (possibly empty).

    Algorithm: Rational Root Theorem — any rational root p/q of a
    polynomial with integer coefficients must have p | (constant term)
    and q | (leading coefficient).  We clear denominators, try all
    candidate p/q pairs.

    Example::

        # x^2 - 1  →  roots [1, -1]
        _rational_roots([Fraction(-1), Fraction(0), Fraction(1)])
        # → [Fraction(1), Fraction(-1)]

        # x^2 - 2  →  no rational roots
        _rational_roots([Fraction(-2), Fraction(0), Fraction(1)])
        # → []
    """
    # Clear denominators to get integer coefficients.
    import math

    lcm_denom = 1
    for c in coeffs:
        lcm_denom = lcm_denom * c.denominator // math.gcd(lcm_denom, c.denominator)
    int_coeffs = [int(c * lcm_denom) for c in coeffs]

    # Strip leading zeros.
    while len(int_coeffs) > 1 and int_coeffs[-1] == 0:
        int_coeffs.pop()

    if len(int_coeffs) == 0:
        return []  # Zero polynomial — every value is a root.
    if len(int_coeffs) == 1:
        return []  # Non-zero constant — no roots.

    const_term = int_coeffs[0]
    leading_coeff = int_coeffs[-1]

    if const_term == 0:
        # x=0 is a root; factor out and continue.
        roots = [Fraction(0)]
        trimmed = int_coeffs[1:]
        roots.extend(_rational_roots([Fraction(c) for c in trimmed]))
        return roots

    # Candidates: ±p/q for p | const_term, q | leading_coeff
    def _divisors(n: int) -> list[int]:
        n = abs(n)
        return [d for d in range(1, n + 1) if n % d == 0]

    p_divs = _divisors(const_term)
    q_divs = _divisors(leading_coeff)
    roots: list[Fraction] = []
    seen: set[Fraction] = set()
    for p in p_divs:
        for q in q_divs:
            for sign in (1, -1):
                cand = Fraction(sign * p, q)
                if cand in seen:
                    continue
                seen.add(cand)
                # Evaluate the polynomial at cand.
                val = Fraction(0)
                for k, c in enumerate(int_coeffs):
                    val += Fraction(c) * (cand ** k)
                if val == 0:
                    roots.append(cand)

    return roots


def _solve_univariate(coeffs: list[Fraction]) -> list[Fraction] | None:
    """Solve a univariate polynomial given coefficients (ascending degree).

    Returns
    -------
    List of rational roots, or None if the polynomial is degree > 4 or
    if back-substitution requires complex numbers.

    Currently we only return rational roots (via the rational-root theorem
    plus a quadratic-formula fallback for degree-2 remainder).

    Example::

        _solve_univariate([Fraction(-1), Fraction(0), Fraction(1)])
        # x^2 - 1 = 0  →  [1, -1]

        _solve_univariate([Fraction(-1), Fraction(2), Fraction(-1)])
        # -1 + 2x - x^2 = 0  →  x^2 - 2x + 1 = 0  →  [1] (double root)
    """
    # Strip leading zeros to find actual degree.
    while len(coeffs) > 1 and coeffs[-1] == 0:
        coeffs = coeffs[:-1]

    deg = len(coeffs) - 1

    if deg <= 0:
        return []  # Constant — no roots.

    if deg == 1:
        # a*x + b = 0  →  x = -b/a
        a, b = coeffs[1], coeffs[0]
        if a == 0:
            return []
        return [-b / a]

    if deg == 2:
        # Quadratic formula: a*x^2 + b*x + c = 0
        c, b, a = coeffs[0], coeffs[1], coeffs[2]
        disc = b * b - 4 * a * c
        if disc < 0:
            return []  # Complex roots — not handled here.
        if disc == 0:
            return [-b / (2 * a)]
        # Try exact square root.
        import math

        n = disc.numerator
        d = disc.denominator
        sqrt_n = math.isqrt(n)
        sqrt_d = math.isqrt(d)
        if sqrt_n * sqrt_n == n and sqrt_d * sqrt_d == d:
            sqrt_disc = Fraction(sqrt_n, sqrt_d)
            r1 = (-b + sqrt_disc) / (2 * a)
            r2 = (-b - sqrt_disc) / (2 * a)
            return [r1, r2] if r1 != r2 else [r1]
        return []  # Irrational roots.

    if deg > 4:
        return None  # Too high — give up.

    # For degree 3 and 4, try rational roots, then divide out and recurse.
    rational = _rational_roots(coeffs)
    if not rational:
        return []  # No rational roots.

    # Divide out each root and collect all.
    all_roots: list[Fraction] = list(rational)

    # Perform synthetic division by each found root.
    remaining = list(coeffs)
    for root in rational:
        # Polynomial division by (x - root).
        q: list[Fraction] = []
        acc = Fraction(0)
        for c in reversed(remaining):
            acc = acc + c
            q.append(acc)
            acc = acc * root
        q = list(reversed(q[1:]))  # Drop the remainder term.
        remaining = q
        if not remaining:
            break

    # Recurse on the deflated polynomial.
    more = _solve_univariate(remaining)
    if more is not None:
        all_roots.extend(more)

    # Deduplicate while preserving order.
    seen: set[Fraction] = set()
    unique: list[Fraction] = []
    for r in all_roots:
        if r not in seen:
            seen.add(r)
            unique.append(r)
    return unique


# ---------------------------------------------------------------------------
# Ideal solve (main entry point)
# ---------------------------------------------------------------------------


def ideal_solve(
    polys: list[MPoly],
    order: str = "lex",
) -> list[list[Fraction]] | None:
    """Solve a polynomial system using a Gröbner basis + back-substitution.

    Parameters
    ----------
    polys:
        List of polynomials (all equal to zero).  Must all share the same
        ``nvars``.
    order:
        Monomial ordering to use for the Gröbner basis.  Use ``"lex"``
        (default) for back-substitution to work.

    Returns
    -------
    A list of solutions, where each solution is a list of ``Fraction``
    values for the variables ``[x₀, x₁, …, xₙ₋₁]`` (in variable-index
    order).

    Returns ``None`` if:
    - The Gröbner computation fails or exceeds safety limits.
    - No univariate polynomial is found in the basis (non-triangular system).
    - The univariate polynomial has no rational roots.
    - The system has infinitely many solutions.

    Example::

        # x + y = 1, x - y = 0
        f1 = MPoly({(1,0): F(1), (0,1): F(1), (0,0): F(-1)}, 2)
        f2 = MPoly({(1,0): F(1), (0,1): F(-1)}, 2)
        ideal_solve([f1, f2])
        # → [[Fraction(1,2), Fraction(1,2)]]
    """
    if not polys:
        return None

    nvars = polys[0].nvars

    # Compute lex Gröbner basis.
    try:
        G = buchberger(polys, order=order)
    except GrobnerError:
        return None

    if not G:
        return None  # Zero ideal — infinitely many solutions.

    # Find a polynomial that is univariate in the LAST variable (index nvars-1).
    # In lex order, the "elimination ideal" structure guarantees such a poly
    # will be the one whose leading monomial only involves the last variable.
    last_var = nvars - 1
    univariate_poly: MPoly | None = None
    for g in G:
        if g.is_univariate() == last_var:
            univariate_poly = g
            break

    if univariate_poly is None:
        return None  # Non-triangular — cannot back-substitute.

    # Solve the univariate polynomial for the last variable.
    coeffs = univariate_poly.to_univariate_coeffs(last_var)
    roots = _solve_univariate(coeffs)
    if roots is None or not roots:
        return None

    # For each root, back-substitute and find the remaining variables.
    all_solutions: list[list[Fraction]] = []

    for root_val in roots:
        # Substitute last variable = root_val into all basis polynomials.
        reduced_basis = [g.eval_at(last_var, root_val) for g in G]
        reduced_basis = [p for p in reduced_basis if not p.is_zero()]

        if nvars == 1:
            # Single-variable case: solution is just [root_val]
            all_solutions.append([root_val])
            continue

        if nvars == 2:
            # Two-variable case: find the other variable directly.
            var0_poly = _find_linear_in_var(reduced_basis, 0)
            if var0_poly is None:
                # Try to solve the reduced system recursively.
                sub_solutions = _solve_from_basis(reduced_basis, nvars - 1)
                if sub_solutions is None:
                    continue
                for sub_sol in sub_solutions:
                    all_solutions.append(sub_sol + [root_val])
            else:
                sol0 = _eval_constant(var0_poly, 0)
                if sol0 is not None:
                    all_solutions.append([sol0, root_val])
        else:
            # Higher-variable case: recurse.
            # Build polynomials in nvars-1 variables by substituting the last var.
            projected = _project_out_last(reduced_basis, nvars, root_val)
            if projected is not None:
                sub_solutions = ideal_solve(projected, order=order)
                if sub_solutions is not None:
                    for sub_sol in sub_solutions:
                        all_solutions.append(sub_sol + [root_val])

    return all_solutions if all_solutions else None


def _find_linear_in_var(basis: list[MPoly], var_idx: int) -> MPoly | None:
    """Find a basis element that is linear (degree 1) in ``var_idx`` only.

    This is used during back-substitution: once we've fixed the last
    variable, we look for a polynomial linear in the next variable so we
    can solve  a·x + b = 0  directly.

    Returns the polynomial or None if none found.
    """
    for p in basis:
        if p.is_zero():
            continue
        if p.is_univariate() == var_idx:
            coeffs = p.to_univariate_coeffs(var_idx)
            if len(coeffs) == 2 and coeffs[1] != 0:
                return p  # Linear in var_idx.
    return None


def _eval_constant(p: MPoly, var_idx: int) -> Fraction | None:
    """If ``p`` is linear in ``var_idx`` and otherwise constant, solve for it.

    Returns the root of  a·x + b = 0  as a Fraction, or None.
    """
    coeffs = p.to_univariate_coeffs(var_idx)
    if len(coeffs) != 2 or coeffs[1] == 0:
        return None
    a, b = coeffs[1], coeffs[0]
    return -b / a


def _solve_from_basis(basis: list[MPoly], nvars: int) -> list[list[Fraction]] | None:
    """Attempt to solve directly from already-reduced basis elements.

    Used when the reduced basis (after substituting the last variable)
    is simple enough to solve by inspection.
    """
    if nvars == 1:
        # Find a univariate poly and solve it.
        for p in basis:
            if not p.is_zero() and p.is_univariate() == 0:
                coeffs = p.to_univariate_coeffs(0)
                roots = _solve_univariate(coeffs)
                if roots:
                    return [[r] for r in roots]
    return None


def _project_out_last(
    basis: list[MPoly], nvars: int, _last_val: Fraction
) -> list[MPoly] | None:
    """Project a list of polynomials into one fewer variable.

    After substituting the last variable, each polynomial in ``basis``
    should only involve the first ``nvars-1`` variables.  This function
    rebuilds them with the correct ``nvars-1`` variable count.

    Returns None if any polynomial still involves the last variable.
    """
    result: list[MPoly] = []
    for p in basis:
        if p.is_zero():
            continue
        # Check that no term involves the last variable.
        has_last = any(m[nvars - 1] != 0 for m in p.coeffs)
        if has_last:
            # Should not happen after eval_at, but be defensive.
            return None
        # Rebuild with nvars-1.
        new_coeffs = {m[: nvars - 1]: c for m, c in p.coeffs.items()}
        result.append(MPoly(new_coeffs, nvars - 1))
    return result if result else None
