"""Monomial ordering for multivariate polynomial computation.

A *monomial* in ``n`` variables  x₁, x₂, …, xₙ  is a product
``x₁^a₁ · x₂^a₂ · … · xₙ^aₙ`` where each exponent aᵢ ≥ 0.  We
represent it as a tuple of non-negative integers ``(a₁, a₂, …, aₙ)`` —
compact, hashable, and directly usable as a dictionary key in
:class:`~cas_multivariate.polynomial.MPoly`.

Why do orderings matter?
------------------------
Buchberger's algorithm and polynomial reduction both depend on a
*monomial ordering*: a total order on monomials that is compatible with
multiplication (if  α > β  then  α·γ > β·γ  for any γ).  The choice of
ordering affects which Gröbner basis you compute, and the *lex* ordering
produces a basis whose shape makes back-substitution straightforward.

Three orderings are implemented here:

* **lex** (lexicographic): compare exponent vectors left-to-right,
  like dictionary ordering.  Useful for back-substitution because the
  lex Gröbner basis has univariate polynomials at the "bottom".

  ``(3, 0) > (2, 5)``  because the first exponent 3 > 2.

* **grlex** (graded lexicographic): compare total degree first; break
  ties lexicographically.  The *standard* ordering for Buchberger
  computations because it terminates faster in practice.

  ``(2, 1) > (1, 2)``  because both have total degree 3 but
  ``(2, 1)`` wins lexicographically.

* **grevlex** (graded reverse lexicographic): compare total degree
  first; break ties by *rightmost* exponent in *reverse*.  Often
  produces the smallest intermediate bases, but less used here.

  ``(1, 1, 1) > (3, 0, 0)``?  Both degree 3; rightmost non-zero
  exponent of ``(3, 0, 0)`` is at position 0 (value 3), while for
  ``(1, 1, 1)`` the rightmost position 2 has value 1 — smaller, so
  ``(1, 1, 1)`` wins.

Usage::

    from cas_multivariate.monomial import monomial_key, lcm_monomial, divides

    alpha = (2, 1)
    beta  = (1, 2)

    # Sort in decreasing grlex order (highest first):
    sorted([beta, alpha], key=monomial_key("grlex"), reverse=True)
    # → [(2, 1), (1, 2)]

    # LCM of two monomials (component-wise max):
    lcm_monomial(alpha, beta)  # → (2, 2)

    # Does alpha divide beta?  α|β iff αᵢ ≤ βᵢ for all i.
    divides((1, 1), (2, 3))   # True
    divides((2, 1), (1, 2))   # False
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Type alias
# ---------------------------------------------------------------------------

#: An exponent tuple representing a monomial.
Monomial = tuple[int, ...]


# ---------------------------------------------------------------------------
# Monomial ordering keys
# ---------------------------------------------------------------------------


def _lex_key(m: Monomial) -> tuple[int, ...]:
    """Lexicographic key: compare component by component, left to right.

    Larger exponent in the first position wins; ties broken by the next
    position, and so on.

    Truth table::

        lex_key((3, 0)) > lex_key((2, 5))   # True  — first coord 3 > 2
        lex_key((2, 5)) > lex_key((2, 3))   # True  — first tied, second 5>3
        lex_key((1, 0)) > lex_key((0, 9))   # True  — first coord 1 > 0
    """
    return m  # tuple comparison IS lexicographic


def _grlex_key(m: Monomial) -> tuple[int, ...]:
    """Graded lexicographic key.

    Sort first by total degree (sum of exponents), then by ``lex`` to
    break ties.  This is the *default ordering* for Buchberger because it
    tends to reduce the number of s-polynomial reductions needed.

    Truth table::

        grlex_key((2, 1)) > grlex_key((1, 2))  # True  — same deg, 2>1 lex
        grlex_key((0, 3)) > grlex_key((2, 0))  # True  — same deg, lex (0,3)>(2,0)? No!
        # (0,3) vs (2,0): both deg 3. lex: 0 < 2 so (2,0) > (0,3).
        grlex_key((2, 0)) > grlex_key((0, 3))  # True
    """
    return (sum(m),) + m


def _grevlex_key(m: Monomial) -> tuple[int | tuple[int, ...], ...]:
    """Graded reverse lexicographic key.

    Sort by total degree first, then break ties by comparing the *last*
    exponent in *reverse* (smaller last exponent wins in grevlex).

    We implement the key as ``(total_degree, negated_reversed_tuple)`` so
    that Python's default tuple comparison gives the correct result.

    Truth table::

        grevlex_key((1,2,0)) vs grevlex_key((0,1,2)):
        total degree: both 3.
        Reversed tuples: (0,2,1) vs (2,1,0).
        Negate:         (0,-2,-1) vs (-2,-1,0).
        Compare:         (0,-2,-1) > (-2,-1,0)  → True.
        So (1,2,0) > (0,1,2) in grevlex.
    """
    return (sum(m),) + tuple(-e for e in reversed(m))


def monomial_key(order: str = "grlex"):
    """Return a sort-key function for the given monomial ordering.

    Parameters
    ----------
    order:
        One of ``"lex"``, ``"grlex"``, ``"grevlex"``.  Defaults to
        ``"grlex"`` — the standard ordering for Buchberger's algorithm.

    Returns
    -------
    A callable ``key(m: Monomial) -> tuple`` suitable for use with
    ``sorted(..., key=monomial_key(order))``.

    Example::

        monomials = [(1, 2), (2, 1), (0, 3)]
        sorted(monomials, key=monomial_key("grlex"), reverse=True)
        # → [(2, 1), (1, 2), (0, 3)]  — all degree 3, lex tiebreak
    """
    if order == "lex":
        return _lex_key
    if order == "grlex":
        return _grlex_key
    if order == "grevlex":
        return _grevlex_key
    raise ValueError(
        f"Unknown monomial order: {order!r}. Use 'lex', 'grlex', or 'grevlex'."
    )


def cmp_monomials(a: Monomial, b: Monomial, order: str = "grlex") -> int:
    """Compare two monomials under the given ordering.

    Returns
    -------
    +1 if a > b, -1 if a < b, 0 if a == b.

    Example::

        cmp_monomials((2, 1), (1, 2), "grlex")  # → +1
        cmp_monomials((1, 2), (2, 1), "grlex")  # → -1
        cmp_monomials((1, 1), (1, 1), "grlex")  # → 0
    """
    key = monomial_key(order)
    ka, kb = key(a), key(b)
    if ka > kb:
        return 1
    if ka < kb:
        return -1
    return 0


# ---------------------------------------------------------------------------
# Monomial arithmetic helpers
# ---------------------------------------------------------------------------


def lcm_monomial(a: Monomial, b: Monomial) -> Monomial:
    """Return the LCM (least common multiple) of two monomials.

    The LCM of  x^a · y^b  and  x^c · y^d  is  x^max(a,c) · y^max(b,d).
    This is used in Buchberger's algorithm to compute the S-polynomial.

    Example::

        lcm_monomial((2, 1, 0), (1, 2, 3))  # → (2, 2, 3)
        lcm_monomial((0, 0), (0, 0))         # → (0, 0)
    """
    return tuple(max(ai, bi) for ai, bi in zip(a, b, strict=True))


def divides(a: Monomial, b: Monomial) -> bool:
    """Return True if monomial ``a`` divides monomial ``b``.

    ``a | b``  iff  ``aᵢ ≤ bᵢ``  for all positions i.

    Example::

        divides((1, 1), (2, 3))   # True  — x·y divides x²·y³
        divides((2, 1), (1, 2))   # False — x² does not divide x¹
        divides((0, 0), (5, 7))   # True  — 1 divides everything
    """
    return all(ai <= bi for ai, bi in zip(a, b, strict=True))


def div_monomial(b: Monomial, a: Monomial) -> Monomial:
    """Return b / a as a monomial, assuming a divides b.

    b / a = x^(b₁−a₁) · y^(b₂−a₂) · …

    The caller is responsible for checking that ``divides(a, b)`` is True
    before calling this; negative exponents are not meaningful in a
    polynomial ring.

    Example::

        div_monomial((3, 2), (1, 1))  # → (2, 1)
        div_monomial((2, 0), (2, 0))  # → (0, 0)   — monomial / itself = 1
    """
    return tuple(bi - ai for bi, ai in zip(b, a, strict=True))


def total_degree(m: Monomial) -> int:
    """Return the total degree of a monomial (sum of all exponents).

    Example::

        total_degree((3, 2, 1))  # → 6
        total_degree((0, 0))     # → 0
    """
    return sum(m)
