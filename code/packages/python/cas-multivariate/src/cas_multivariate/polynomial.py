"""Multivariate polynomial class over Q (rational coefficients).

A multivariate polynomial is a finite linear combination of monomials
with rational coefficients.  For example:

    3x²y − (1/2)y³ + 1

is represented as::

    {(2, 1): Fraction(3),
     (0, 3): Fraction(-1, 2),
     (0, 0): Fraction(1)}

We use a sparse ``dict`` representation (keys are exponent tuples, values
are :class:`~fractions.Fraction` coefficients) because most polynomials
arising in Gröbner-basis computations are sparse — high-degree monomials
with zero coefficients are simply absent from the dict.

Design decisions
----------------
* **Zero terms omitted** — coefficients equal to zero are removed on any
  arithmetic operation.  This keeps the dict lean and makes ``is_zero()``
  trivially ``not self.coeffs``.

* **Immutable interface** — arithmetic operations return new ``MPoly``
  objects; ``MPoly`` is *not* frozen but should not be mutated externally.

* **Fraction coefficients** — We stay entirely in Q (rationals) so that
  Gröbner-basis computation is exact.  Floating-point cancellations would
  make Buchberger unreliable.

* **``nvars``** — every polynomial carries its variable count so that
  monomial tuples always have the same length.  This catches mixing
  polynomials in different rings at assertion time.

Usage::

    from fractions import Fraction
    from cas_multivariate.polynomial import MPoly

    x2y = MPoly({(2, 1): Fraction(1)}, nvars=2)   # x^2 * y
    xy2 = MPoly({(1, 2): Fraction(1)}, nvars=2)   # x * y^2
    p   = x2y + xy2                                 # x^2*y + x*y^2

    p.lm("grlex")   # → (2, 1)  — leading monomial in grlex order
    p.lc("grlex")   # → Fraction(1)
"""

from __future__ import annotations

from fractions import Fraction

from cas_multivariate.monomial import (
    Monomial,
    cmp_monomials,
    div_monomial,
    divides,
    monomial_key,
    total_degree,
)

# ---------------------------------------------------------------------------
# Helper: normalise a coefficient dict (remove zeros)
# ---------------------------------------------------------------------------


def _clean(d: dict[Monomial, Fraction]) -> dict[Monomial, Fraction]:
    """Return a copy of ``d`` with all zero-coefficient entries removed.

    Zero terms can arise after subtraction or multiplication and should
    not appear in the canonical representation.

    Example::

        _clean({(1, 0): Fraction(3), (0, 1): Fraction(0)})
        # → {(1, 0): Fraction(3)}
    """
    return {m: c for m, c in d.items() if c != 0}


# ---------------------------------------------------------------------------
# MPoly class
# ---------------------------------------------------------------------------


class MPoly:
    """A multivariate polynomial with rational (Q) coefficients.

    Attributes
    ----------
    coeffs : dict[Monomial, Fraction]
        Sparse coefficient map.  Zero coefficients are absent.
    nvars : int
        Number of variables (dimension of each exponent tuple).

    Constructors
    ------------
    Pass the coefficient dict and variable count directly::

        p = MPoly({(2, 0): Fraction(1), (0, 0): Fraction(-1)}, nvars=2)
        # represents x^2 - 1

    Or use the class-level factory methods:

        MPoly.zero(n)      — the zero polynomial in n variables
        MPoly.constant(c, n) — a rational constant as an n-variable poly
        MPoly.monomial(exp, c, n) — c · x^exp
    """

    # ------------------------------------------------------------------
    # Construction
    # ------------------------------------------------------------------

    def __init__(
        self,
        coeffs: dict[Monomial, Fraction],
        nvars: int,
    ) -> None:
        """Initialise from a coefficient dict.

        Parameters
        ----------
        coeffs:
            Mapping exponent tuple → coefficient.  Zero entries are
            silently discarded.
        nvars:
            Number of variables.  All exponent tuples must have exactly
            this length.
        """
        self.nvars = nvars
        self.coeffs: dict[Monomial, Fraction] = _clean(coeffs)

    @classmethod
    def zero(cls, nvars: int) -> MPoly:
        """Return the additive identity (the zero polynomial).

        Example::

            p = MPoly.zero(3)
            p.is_zero()  # True
        """
        return cls({}, nvars)

    @classmethod
    def constant(cls, c: Fraction | int, nvars: int) -> MPoly:
        """Return the polynomial equal to the rational constant ``c``.

        Example::

            MPoly.constant(Fraction(3, 2), 2)
            # represents 3/2 in Q[x, y]
        """
        c = Fraction(c)
        if c == 0:
            return cls.zero(nvars)
        zero_exp: Monomial = (0,) * nvars
        return cls({zero_exp: c}, nvars)

    @classmethod
    def monomial_poly(cls, exp: Monomial, c: Fraction | int, nvars: int) -> MPoly:
        """Return the polynomial ``c · x^exp``.

        Example::

            MPoly.monomial_poly((2, 1), Fraction(3), 2)
            # represents 3 x^2 y
        """
        c = Fraction(c)
        if c == 0:
            return cls.zero(nvars)
        return cls({exp: c}, nvars)

    # ------------------------------------------------------------------
    # Zero / bool checks
    # ------------------------------------------------------------------

    def is_zero(self) -> bool:
        """Return True if this polynomial is the zero polynomial.

        A polynomial is zero iff its coefficient dict is empty (all
        coefficients are zero and were removed by ``_clean``).

        Example::

            MPoly.zero(2).is_zero()  # True
            MPoly({(1, 0): Fraction(1)}, 2).is_zero()  # False
        """
        return not self.coeffs

    def __bool__(self) -> bool:
        """Return True when the polynomial is *non-zero*.

        Follows Python's convention: falsy iff zero.

        Example::

            if not p.is_zero():
                ...  # equivalent to `if p:`
        """
        return bool(self.coeffs)

    # ------------------------------------------------------------------
    # Leading term, monomial, coefficient
    # ------------------------------------------------------------------

    def lm(self, order: str = "grlex") -> Monomial:
        """Return the leading monomial under ``order``.

        The leading monomial is the largest monomial (under the given
        ordering) with a non-zero coefficient.

        Raises ``ValueError`` for the zero polynomial (no leading term).

        Example::

            p = MPoly({(2, 1): Fraction(1), (0, 3): Fraction(2)}, nvars=2)
            p.lm("grlex")   # (2, 1) — both degree 3, but (2,1) > (0,3) lex
            p.lm("lex")     # (2, 1) — (2,…) > (0,…)
        """
        if not self.coeffs:
            raise ValueError("Leading monomial of the zero polynomial is undefined.")
        key = monomial_key(order)
        return max(self.coeffs, key=key)

    def lc(self, order: str = "grlex") -> Fraction:
        """Return the leading coefficient under ``order``.

        Example::

            p = MPoly({(2, 1): Fraction(3), (0, 3): Fraction(2)}, nvars=2)
            p.lc("grlex")   # Fraction(3)
        """
        return self.coeffs[self.lm(order)]

    def lt(self, order: str = "grlex") -> MPoly:
        """Return the leading term as a single-term polynomial.

        The leading term is ``lc(order) · lm(order)``.  It is returned as
        an ``MPoly`` so arithmetic operations compose naturally.

        Example::

            p = MPoly({(2, 1): Fraction(3), (0, 1): Fraction(1)}, nvars=2)
            p.lt("grlex")
            # MPoly({(2, 1): Fraction(3)}, nvars=2)
        """
        m = self.lm(order)
        return MPoly({m: self.coeffs[m]}, self.nvars)

    def total_degree(self) -> int:
        """Return the total degree of this polynomial (max over all monomials).

        Example::

            MPoly({(2, 1): Fraction(1), (0, 3): Fraction(1)}, 2).total_degree()
            # → 3  (max of sum(2,1)=3 and sum(0,3)=3)
        """
        if not self.coeffs:
            return 0
        return max(total_degree(m) for m in self.coeffs)

    # ------------------------------------------------------------------
    # Arithmetic
    # ------------------------------------------------------------------

    def __add__(self, other: MPoly) -> MPoly:
        """Return self + other.

        Add coefficient by coefficient; terms with zero result are
        discarded by ``_clean``.

        Example::

            p = MPoly({(1, 0): Fraction(2)}, 2)
            q = MPoly({(1, 0): Fraction(-1), (0, 1): Fraction(3)}, 2)
            (p + q).coeffs
            # {(1, 0): Fraction(1), (0, 1): Fraction(3)}
        """
        assert self.nvars == other.nvars, "Variable count mismatch in MPoly addition"
        result: dict[Monomial, Fraction] = dict(self.coeffs)
        for m, c in other.coeffs.items():
            result[m] = result.get(m, Fraction(0)) + c
        return MPoly(_clean(result), self.nvars)

    def __neg__(self) -> MPoly:
        """Return -self (negate all coefficients).

        Example::

            -MPoly({(1, 0): Fraction(2), (0, 0): Fraction(-1)}, 2)
            # → {(1, 0): -2, (0, 0): 1}
        """
        return MPoly({m: -c for m, c in self.coeffs.items()}, self.nvars)

    def __sub__(self, other: MPoly) -> MPoly:
        """Return self - other.

        Example::

            p = MPoly({(1, 0): Fraction(3)}, 2)
            q = MPoly({(1, 0): Fraction(1), (0, 1): Fraction(2)}, 2)
            (p - q).coeffs
            # {(1, 0): Fraction(2), (0, 1): Fraction(-2)}
        """
        return self + (-other)

    def __mul__(self, other: MPoly) -> MPoly:
        """Return self * other (polynomial multiplication).

        Multiply each pair of terms, add up coefficients for equal
        monomials.

        Example::

            # (x + 1)(x − 1) = x^2 − 1
            p = MPoly({(1, 0): Fraction(1), (0, 0): Fraction(1)}, 2)   # x+1 (in 2 vars)
            q = MPoly({(1, 0): Fraction(1), (0, 0): Fraction(-1)}, 2)  # x−1
            (p * q).coeffs
            # {(2, 0): 1, (0, 0): -1}
        """
        assert self.nvars == other.nvars, (
            "Variable count mismatch in MPoly multiplication"
        )
        result: dict[Monomial, Fraction] = {}
        for ma, ca in self.coeffs.items():
            for mb, cb in other.coeffs.items():
                mc: Monomial = tuple(a + b for a, b in zip(ma, mb, strict=True))
                result[mc] = result.get(mc, Fraction(0)) + ca * cb
        return MPoly(_clean(result), self.nvars)

    def scale(self, c: Fraction | int) -> MPoly:
        """Return ``c * self`` (scalar multiplication).

        Example::

            p = MPoly({(2, 0): Fraction(2)}, 2)
            p.scale(Fraction(3, 2)).coeffs  # {(2, 0): Fraction(3)}
        """
        c = Fraction(c)
        if c == 0:
            return MPoly.zero(self.nvars)
        return MPoly({m: c * coeff for m, coeff in self.coeffs.items()}, self.nvars)

    def mul_monomial(self, exp: Monomial, c: Fraction | int = 1) -> MPoly:
        """Return ``c · x^exp · self`` (multiply by a single monomial term).

        Used internally by the reduction algorithm to scale and shift a
        polynomial when subtracting a multiple of a divisor.

        Example::

            p = MPoly({(1, 0): Fraction(1), (0, 0): Fraction(1)}, 2)  # x+1
            p.mul_monomial((1, 0))   # x(x+1) = x^2 + x
        """
        c = Fraction(c)
        if c == 0:
            return MPoly.zero(self.nvars)
        new_coeffs: dict[Monomial, Fraction] = {}
        for m, coeff in self.coeffs.items():
            new_m: Monomial = tuple(ei + ej for ei, ej in zip(exp, m, strict=True))
            new_coeffs[new_m] = coeff * c
        return MPoly(_clean(new_coeffs), self.nvars)

    # ------------------------------------------------------------------
    # Equality and hashing
    # ------------------------------------------------------------------

    def __eq__(self, other: object) -> bool:
        """Test polynomial equality.

        Two polynomials are equal iff they have the same variable count and
        the same (cleaned) coefficient map.

        Example::

            p = MPoly({(2, 0): Fraction(1), (0, 0): Fraction(-1)}, 2)
            q = MPoly({(2, 0): Fraction(1), (0, 0): Fraction(-1)}, 2)
            p == q   # True
        """
        if not isinstance(other, MPoly):
            return NotImplemented
        return self.nvars == other.nvars and self.coeffs == other.coeffs

    def __repr__(self) -> str:
        """Developer-friendly representation.

        Example::

            repr(MPoly({(2, 0): Fraction(1)}, 2))
            # "MPoly({(2, 0): Fraction(1, 1)}, nvars=2)"
        """
        return f"MPoly({self.coeffs!r}, nvars={self.nvars})"

    # ------------------------------------------------------------------
    # Utility
    # ------------------------------------------------------------------

    def monomials_descending(self, order: str = "grlex") -> list[Monomial]:
        """Return all non-zero monomials in descending order.

        Useful for pretty-printing and univariate root-finding.

        Example::

            p = MPoly(
                {(2, 0): Fraction(1), (1, 0): Fraction(-3), (0, 0): Fraction(2)}, 2
            )
            p.monomials_descending("lex")
            # [(2, 0), (1, 0), (0, 0)]
        """
        key = monomial_key(order)
        return sorted(self.coeffs, key=key, reverse=True)

    def is_univariate(self) -> int | None:
        """Return the index of the *only* active variable, or None.

        A polynomial in ``nvars`` variables is univariate if at most one
        variable index has a non-zero exponent across all monomials.

        Returns
        -------
        The variable index (0-based) if univariate in exactly one
        variable; ``None`` otherwise.

        Examples::

            # 3x^2 + 1 in Q[x, y] — only variable 0 (x) appears
            p = MPoly({(2, 0): Fraction(3), (0, 0): Fraction(1)}, 2)
            p.is_univariate()   # → 0

            # x*y — both variables
            q = MPoly({(1, 1): Fraction(1)}, 2)
            q.is_univariate()   # → None
        """
        active: set[int] = set()
        for m in self.coeffs:
            for i, e in enumerate(m):
                if e != 0:
                    active.add(i)
        if len(active) == 1:
            return next(iter(active))
        if not active:
            # constant polynomial — consider it "univariate" in var 0 by convention
            return 0
        return None

    def to_univariate_coeffs(self, var_idx: int) -> list[Fraction]:
        """Extract coefficients as a univariate list for variable ``var_idx``.

        Assumes ``self.is_univariate() == var_idx`` (caller's responsibility).

        Returns a list where index ``k`` holds the coefficient of
        ``x^k`` (ascending degree).  The list length is ``max_degree + 1``.

        Example::

            # x^2 - 1 in Q[x, y]
            p = MPoly({(2, 0): Fraction(1), (0, 0): Fraction(-1)}, 2)
            p.to_univariate_coeffs(0)   # → [Fraction(-1), Fraction(0), Fraction(1)]
        """
        max_deg = max((m[var_idx] for m in self.coeffs), default=0)
        result = [Fraction(0)] * (max_deg + 1)
        for m, c in self.coeffs.items():
            result[m[var_idx]] = c
        return result

    def leading_monomial_divides(self, m: Monomial, order: str = "grlex") -> bool:
        """Return True if the leading monomial divides ``m``.

        Convenience predicate used during polynomial reduction: can the
        leading term of ``self`` cancel the monomial ``m`` in the dividend?

        Example::

            g = MPoly({(1, 1): Fraction(1)}, 2)   # x*y
            g.leading_monomial_divides((2, 3))     # True — (1,1)|(2,3)
            g.leading_monomial_divides((0, 2))     # False — (1,1) does not divide (0,2)
        """
        lm = self.lm(order)
        return divides(lm, m)

    def _compare_monomials(self, a: Monomial, b: Monomial, order: str) -> int:
        """Delegate to :func:`~cas_multivariate.monomial.cmp_monomials`."""
        return cmp_monomials(a, b, order)

    def diff(self, var_idx: int) -> MPoly:
        """Return the partial derivative with respect to variable ``var_idx``.

        d/dx_i (c · x^a) = c · a_i · x^(a with a_i reduced by 1)

        Example::

            # d/dx (x^2 y + x) = 2xy + 1
            p = MPoly({(2, 1): Fraction(1), (1, 0): Fraction(1)}, 2)
            p.diff(0).coeffs
            # {(1, 1): Fraction(2), (0, 0): Fraction(1)}
        """
        result: dict[Monomial, Fraction] = {}
        for m, c in self.coeffs.items():
            exp = m[var_idx]
            if exp == 0:
                continue  # constant in this variable — derivative is 0
            new_m = tuple(e - (1 if i == var_idx else 0) for i, e in enumerate(m))
            result[new_m] = result.get(new_m, Fraction(0)) + c * exp  # type: ignore[arg-type]
        return MPoly(_clean(result), self.nvars)

    def eval_at(self, var_idx: int, value: Fraction) -> MPoly:
        """Substitute ``value`` for variable ``var_idx``, returning a new poly.

        This is used during back-substitution in :mod:`~cas_multivariate.solve`.

        Example::

            # p = x^2 + y, substitute x=2 → 4 + y
            p = MPoly({(2, 0): Fraction(1), (0, 1): Fraction(1)}, 2)
            result = p.eval_at(0, Fraction(2))
            result.coeffs
            # {(0, 1): Fraction(1), (0, 0): Fraction(4)}
        """
        result: dict[Monomial, Fraction] = {}
        for m, c in self.coeffs.items():
            exp = m[var_idx]
            scaled = c * (value ** exp)
            new_m = tuple(e if i != var_idx else 0 for i, e in enumerate(m))
            result[new_m] = result.get(new_m, Fraction(0)) + scaled  # type: ignore[arg-type]
        return MPoly(_clean(result), self.nvars)


# ---------------------------------------------------------------------------
# Factory: build MPoly from a monomial/coeff pair (shorthand)
# ---------------------------------------------------------------------------


def make_var(var_idx: int, nvars: int) -> MPoly:
    """Return the polynomial equal to the single variable x_{var_idx}.

    Convenience factory used in tests and IR conversion.

    Example::

        make_var(0, 2)   # MPoly with {(1,0): 1} — the variable x in Q[x,y]
        make_var(1, 2)   # MPoly with {(0,1): 1} — the variable y in Q[x,y]
    """
    exp: Monomial = tuple(1 if i == var_idx else 0 for i in range(nvars))
    return MPoly({exp: Fraction(1)}, nvars)


def div_reduction_step(
    f: MPoly, g: MPoly, order: str = "grlex"
) -> tuple[MPoly, MPoly] | None:
    """Perform one step of multivariate division of ``f`` by ``g``.

    If ``g``'s leading term divides ``f``'s leading term, subtract the
    appropriate multiple of ``g`` from ``f`` and return ``(quotient_term, remainder)``.
    Otherwise return ``None`` (this step doesn't apply).

    This is the inner loop of the reduction algorithm.

    Returns
    -------
    ``(term, new_f)`` where ``term`` is the single-term polynomial we
    subtracted (the piece of the quotient), and ``new_f = f - term * g``.

    Example::

        # x^2 / x = x  (one step)
        f = MPoly({(2, 0): Fraction(1)}, 2)
        g = MPoly({(1, 0): Fraction(1)}, 2)
        term, new_f = div_reduction_step(f, g, "lex")
        # term.coeffs == {(1, 0): 1}, new_f.is_zero() == True
    """
    if f.is_zero():
        return None
    lm_f = f.lm(order)
    lm_g = g.lm(order)
    if not divides(lm_g, lm_f):
        return None
    # Compute the multiplier: (lc_f / lc_g) * x^(lm_f - lm_g)
    exp_diff: Monomial = div_monomial(lm_f, lm_g)
    coeff = f.lc(order) / g.lc(order)
    term = MPoly.monomial_poly(exp_diff, coeff, f.nvars)
    return term, f - g.mul_monomial(exp_diff, coeff)
