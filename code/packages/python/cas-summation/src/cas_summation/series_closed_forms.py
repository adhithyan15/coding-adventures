"""Canonical infinite-series closed-form recogniser — Track I1.

Track I1 of ``code/specs/macsyma-truly-finish-plan.md``.

This module pattern-matches the canonical convergent series whose
closed forms are known in elementary form and returns the IR for the
closed form.  It is invoked from :func:`cas_summation.summation.evaluate_sum`
on the infinite-upper-bound branch (``hi = %inf``) *after* the
constant / geometric / telescope / classic-series / Gosper paths but
*before* the unevaluated fallback.

Recognised series
-----------------

==========================================  ==================================
Sum                                          Closed form
==========================================  ==================================
``∑_{k=1}^∞ 1/k^(2m)``  (m = 1..6)            ``(2π)^(2m) · |B_{2m}| / (2·(2m)!)``
``∑_{k=1}^∞ (-1)^(k-1)/k``                    ``log(2)``
``∑_{k=1}^∞ (-1)^(k-1)/k^(2m)``  (m = 1..3)   ``(1 − 2^(1-2m)) · zeta(2m)``
``∑_{k=0}^∞ 1/k!``                            ``%e``
``∑_{k=0}^∞ x^k/k!``                          ``exp(x)``
``∑_{k=0}^∞ (-1)^k · x^(2k)/(2k)!``           ``cos(x)``
``∑_{k=0}^∞ (-1)^k · x^(2k+1)/(2k+1)!``       ``sin(x)``
``∑_{k=0}^∞ x^(2k)/(2k)!``                    ``cosh(x)``
``∑_{k=0}^∞ x^(2k+1)/(2k+1)!``                ``sinh(x)``
==========================================  ==================================

For the symbolic-in-``x`` Taylor patterns (``exp_x``, ``cos_x``,
``sin_x``, ``cosh_x``, ``sinh_x``), the summand contains a free symbol
that is structurally distinct from the index ``k``; the helper extracts
it and returns ``Exp(x)`` / ``Cos(x)`` / etc.

Design constraints (per spec §I1)
---------------------------------

* **One generic Bernoulli helper** — ``B_{2m}`` is computed by the
  textbook recurrence ``B_0 = 1; Σ_{j=0}^{n} C(n+1, j) · B_j = 0``.
  No hardcoded lookup table.  Bounded recursion: the recurrence is a
  pure ``for j in range(n)`` loop, depth ``n``.
* **One generic zeta-2m branch** — coefficient derived from
  ``(2π)^(2m) · |B_{2m}| / (2·(2m)!)``.  Same for the eta-2m branch via
  ``η(2m) = (1 − 2^(1−2m)) · ζ(2m)``.  Six exponents handled by a
  single helper.
* **Exact arithmetic** — every numeric step is ``Fraction``; the IR
  literal is then emitted as ``IRRational`` or ``IRInteger`` depending
  on the denominator.
* **Infinite-bound only** — finite ``hi`` returns ``None`` immediately
  so the caller falls through to Faulhaber / geometric / Gosper.

Notes on factorial representation
---------------------------------

The IR represents ``k!`` as ``GammaFunc(k+1)`` (analytic continuation
of factorial; matches MACSYMA convention).  All factorial-in-denominator
patterns match against ``DIV(numer, GAMMA_FUNC(ADD(k+1, ...)))`` shapes.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EXP,
    GAMMA_FUNC,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SINH,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# π and e as IR symbol literals (MACSYMA convention).
_PI = IRSymbol("%pi")
_E = IRSymbol("%e")

# Maximum even-zeta exponent we recognise — matches the spec's table
# (k = 2..12 in steps of 2 → m = 1..6).  Anything beyond this falls
# through to the unevaluated SUM.  This bound also caps the Bernoulli
# recurrence depth, so the helper is provably bounded.
_MAX_ZETA_M = 6
# Maximum even-eta exponent (1 − 2^(1−2m)) · ζ(2m).  Spec lists eta(2),
# eta(4), eta(6); we use the same generic helper.
_MAX_ETA_M = 3


# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _frac(c: Fraction) -> IRNode:
    """Convert a ``Fraction`` to the smallest IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _is_int(node: IRNode, value: int) -> bool:
    """True iff ``node`` is an ``IRInteger`` equal to ``value``."""
    return isinstance(node, IRInteger) and node.value == value


def _is_neg_one_base(node: IRNode) -> bool:
    """True for ``-1`` whether stored as ``IRInteger(-1)`` or ``NEG(1)``."""
    if _is_int(node, -1):
        return True
    return (
        isinstance(node, IRApply)
        and node.head == NEG
        and len(node.args) == 1
        and _is_int(node.args[0], 1)
    )


def _is_constant_in(node: IRNode, k: IRSymbol) -> bool:
    """True iff ``node`` is structurally constant in ``k``."""
    if node == k:
        return False
    if isinstance(node, IRApply):
        return all(_is_constant_in(arg, k) for arg in node.args)
    return True


# ---------------------------------------------------------------------------
# Bernoulli numbers (one generic helper)
# ---------------------------------------------------------------------------


def _bernoulli(n: int) -> Fraction:
    """Return ``B_n`` (the n-th Bernoulli number) as an exact ``Fraction``.

    Computed via the textbook recurrence::

        B_0 = 1
        Σ_{j=0}^{n} C(n+1, j) · B_j = 0      for n ≥ 1

    Solving the last equation for ``B_n`` gives::

        B_n = − (1 / (n+1)) · Σ_{j=0}^{n-1} C(n+1, j) · B_j

    The recurrence is a pure ``for j in range(n)`` loop, so the depth is
    bounded by ``n``; the caller only ever asks for ``n ≤ 2·_MAX_ZETA_M``
    (= 12), so memory and time are both trivial.

    Convention used: ``B_1 = −1/2`` (Knuth / MACSYMA).  The sign of the
    odd-index Bernoullis doesn't matter for this module since we only
    consult the even-index ones, but we follow the standard convention
    so future callers see no surprise.

    Examples
    --------
    >>> _bernoulli(0)
    Fraction(1, 1)
    >>> _bernoulli(2)
    Fraction(1, 6)
    >>> _bernoulli(4)
    Fraction(-1, 30)
    >>> _bernoulli(12)
    Fraction(-691, 2730)
    """
    if n < 0:
        raise ValueError(f"Bernoulli index must be non-negative, got {n}")
    bs: list[Fraction] = [Fraction(0)] * (n + 1)
    bs[0] = Fraction(1)
    # Binomial coefficients built iteratively to keep arithmetic exact.
    for m in range(1, n + 1):
        # B_m = − (1 / (m+1)) · Σ_{j=0}^{m-1} C(m+1, j) · B_j
        total = Fraction(0)
        binom = 1  # C(m+1, 0)
        for j in range(m):
            total += binom * bs[j]
            # Update C(m+1, j) → C(m+1, j+1).
            binom = binom * (m + 1 - j) // (j + 1)
        bs[m] = -total / (m + 1)
    return bs[n]


def _zeta_even(m: int) -> Fraction:
    """Return ``ζ(2m)`` as an exact ``Fraction × π^(2m)`` coefficient.

    Specifically returns the rational coefficient ``c`` such that
    ``ζ(2m) = c · π^(2m)`` — *not* the value of ``ζ(2m)`` itself
    (which is transcendental).

    Closed form (Euler 1735)::

        ζ(2m) = (−1)^(m+1) · (2π)^(2m) · B_{2m} / (2 · (2m)!)
              = (2π)^(2m) · |B_{2m}| / (2 · (2m)!)

    The second form drops the alternating sign because ``B_{2m}`` itself
    alternates: ``B_2 = +1/6, B_4 = −1/30, B_6 = +1/42, …``.  Using the
    absolute value gives the always-positive coefficient directly.

    Extracting the ``π^(2m)`` factor leaves the rational coefficient::

        c = 2^(2m) · |B_{2m}| / (2 · (2m)!)

    Examples
    --------
    >>> _zeta_even(1)   # ζ(2) = π²/6
    Fraction(1, 6)
    >>> _zeta_even(2)   # ζ(4) = π⁴/90
    Fraction(1, 90)
    >>> _zeta_even(3)   # ζ(6) = π⁶/945
    Fraction(1, 945)
    >>> _zeta_even(6)   # ζ(12) = 691·π¹²/638512875
    Fraction(691, 638512875)
    """
    if m < 1:
        raise ValueError(f"zeta-even index must be ≥ 1, got {m}")
    b = _bernoulli(2 * m)
    factorial_2m = 1
    for i in range(1, 2 * m + 1):
        factorial_2m *= i
    # 2^(2m) · |B_{2m}| / (2 · (2m)!)
    return Fraction(2 ** (2 * m)) * abs(b) / (2 * factorial_2m)


def _eta_even(m: int) -> Fraction:
    """Return the rational coefficient ``c`` such that ``η(2m) = c · π^(2m)``.

    Closed form via the Dirichlet eta–Riemann zeta identity::

        η(s) = (1 − 2^(1 − s)) · ζ(s)

    For ``s = 2m``::

        c = (1 − 2^(1 − 2m)) · _zeta_even(m)

    Examples
    --------
    >>> _eta_even(1)   # η(2) = π²/12
    Fraction(1, 12)
    >>> _eta_even(2)   # η(4) = 7π⁴/720
    Fraction(7, 720)
    >>> _eta_even(3)   # η(6) = 31π⁶/30240
    Fraction(31, 30240)
    """
    if m < 1:
        raise ValueError(f"eta-even index must be ≥ 1, got {m}")
    return (Fraction(1) - Fraction(1, 2 ** (2 * m - 1))) * _zeta_even(m)


def _pi_power_with_coeff(coeff: Fraction, power: int) -> IRNode:
    """Build the IR for ``coeff · π^power``.

    Emits a normalised shape: ``MUL(coeff_ir, POW(π, power))`` when the
    coefficient is a non-trivial rational, or just ``POW(π, power) /
    denom`` when the coefficient is ``1/n`` for some integer ``n`` (to
    match the canonical form ``π²/6`` rather than ``(1/6)·π²``).  Both
    shapes are equivalent under VM eval; the latter is more readable.
    """
    if coeff.numerator == 1 and coeff.denominator > 1:
        # Canonical form: π^power / denom
        return _div(_pow(_PI, _int(power)), _int(coeff.denominator))
    # General: (numer/denom) · π^power
    return _mul(_frac(coeff), _pow(_PI, _int(power)))


# ---------------------------------------------------------------------------
# Pattern recognisers
# ---------------------------------------------------------------------------


def _extract_inv_k_pow(f: IRNode, k: IRSymbol) -> int | None:
    """Match ``1/k^m`` (or ``1/k`` ≡ m=1) and return ``m``; else ``None``.

    Accepts both ``DIV(1, POW(k, m))`` and ``DIV(1, k)``.
    """
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numer, denom = f.args
    if not _is_int(numer, 1):
        return None
    if denom == k:
        return 1
    if isinstance(denom, IRApply) and denom.head == POW and len(denom.args) == 2:
        base, exp = denom.args
        if base == k and isinstance(exp, IRInteger) and exp.value >= 1:
            return exp.value
    return None


def _extract_alt_inv_k_pow(f: IRNode, k: IRSymbol) -> int | None:
    """Match ``(-1)^(k-1) / k^m`` and return ``m``; else ``None``.

    Accepts both ``DIV((-1)^(k-1), POW(k, m))`` and ``DIV((-1)^(k-1), k)``.
    The sign factor is matched as ``POW(base, SUB(k, 1))`` where ``base``
    is either ``IRInteger(-1)`` or ``NEG(1)``.
    """
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numer, denom = f.args
    # Numerator: (-1)^(k-1).
    if not (
        isinstance(numer, IRApply)
        and numer.head == POW
        and len(numer.args) == 2
        and _is_neg_one_base(numer.args[0])
    ):
        return None
    exp_node = numer.args[1]
    if not (
        isinstance(exp_node, IRApply)
        and exp_node.head == SUB
        and len(exp_node.args) == 2
        and exp_node.args[0] == k
        and _is_int(exp_node.args[1], 1)
    ):
        return None
    # Denominator: k or k^m.
    if denom == k:
        return 1
    if isinstance(denom, IRApply) and denom.head == POW and len(denom.args) == 2:
        base, exp = denom.args
        if base == k and isinstance(exp, IRInteger) and exp.value >= 1:
            return exp.value
    return None


def _try_zeta_2m(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=1}^∞ 1/k^(2m) → ζ(2m) · π^(2m)`` for ``m = 1..6``."""
    if not _is_int(lo, 1):
        return None
    m_exp = _extract_inv_k_pow(f, k)
    if m_exp is None:
        return None
    if m_exp % 2 != 0:
        return None  # Odd zeta is not closed form.
    m = m_exp // 2
    if not (1 <= m <= _MAX_ZETA_M):
        return None
    return _pi_power_with_coeff(_zeta_even(m), 2 * m)


def _try_eta_2m(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=1}^∞ (−1)^(k−1)/k^(2m) → η(2m) · π^(2m)``, ``m = 1..3``."""
    if not _is_int(lo, 1):
        return None
    m_exp = _extract_alt_inv_k_pow(f, k)
    if m_exp is None:
        return None
    if m_exp % 2 != 0:
        return None
    m = m_exp // 2
    if not (1 <= m <= _MAX_ETA_M):
        return None
    return _pi_power_with_coeff(_eta_even(m), 2 * m)


def _try_eta_1(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=1}^∞ (−1)^(k−1)/k → log(2)``  (Mercator series)."""
    if not _is_int(lo, 1):
        return None
    m_exp = _extract_alt_inv_k_pow(f, k)
    if m_exp != 1:
        return None
    return IRApply(LOG, (_int(2),))


def _match_gamma_kp1(node: IRNode, k: IRSymbol) -> bool:
    """True iff ``node = GammaFunc(k + 1)`` (= ``k!``)."""
    if not (
        isinstance(node, IRApply)
        and node.head == GAMMA_FUNC
        and len(node.args) == 1
    ):
        return False
    arg = node.args[0]
    return (
        isinstance(arg, IRApply)
        and arg.head == ADD
        and len(arg.args) == 2
        and arg.args[0] == k
        and _is_int(arg.args[1], 1)
    )


def _match_gamma_of_linear_in_k_plus_1(
    node: IRNode, k: IRSymbol, slope: int, intercept: int
) -> bool:
    """True iff ``node = GammaFunc(slope·k + intercept + 1)``.

    Matches the IR shape ``GAMMA_FUNC(ADD(MUL(slope, k), intercept+1))``
    used for ``(slope·k + intercept)!`` (e.g. ``(2k)!`` is
    ``GammaFunc(2k + 1)``, ``(2k+1)!`` is ``GammaFunc(2k + 2)``).
    """
    if not (
        isinstance(node, IRApply)
        and node.head == GAMMA_FUNC
        and len(node.args) == 1
    ):
        return False
    arg = node.args[0]
    if not (isinstance(arg, IRApply) and arg.head == ADD and len(arg.args) == 2):
        return False
    left, right = arg.args
    # left = slope * k, right = intercept + 1
    if not (
        isinstance(left, IRApply)
        and left.head == MUL
        and len(left.args) == 2
        and _is_int(left.args[0], slope)
        and left.args[1] == k
    ):
        return False
    return _is_int(right, intercept + 1)


def _try_e_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ 1/k! → %e``."""
    if not _is_int(lo, 0):
        return None
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numer, denom = f.args
    if not _is_int(numer, 1):
        return None
    if not _match_gamma_kp1(denom, k):
        return None
    return _E


def _extract_pow_of_x_in_k(node: IRNode, k: IRSymbol) -> IRNode | None:
    """If ``node = POW(x, k)`` with ``x`` constant in ``k`` and ``x ≠ k``,
    return ``x``; else ``None``."""
    if not (isinstance(node, IRApply) and node.head == POW and len(node.args) == 2):
        return None
    base, exp = node.args
    if exp != k:
        return None
    if base == k:
        return None
    if not _is_constant_in(base, k):
        return None
    return base


def _try_exp_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ x^k/k! → exp(x)`` (symbolic ``x ≠ k``)."""
    if not _is_int(lo, 0):
        return None
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numer, denom = f.args
    x = _extract_pow_of_x_in_k(numer, k)
    if x is None:
        return None
    if not _match_gamma_kp1(denom, k):
        return None
    return IRApply(EXP, (x,))


def _extract_pow_of_x_in_linear_k(
    node: IRNode, k: IRSymbol, slope: int, intercept: int
) -> IRNode | None:
    """If ``node = POW(x, slope·k + intercept)`` (or ``POW(x, slope·k)``
    when ``intercept == 0``) with ``x`` constant in ``k``, return ``x``.

    Matches the IR ``POW(x, ADD(MUL(slope, k), intercept))`` when
    ``intercept != 0``, and ``POW(x, MUL(slope, k))`` when
    ``intercept == 0``.
    """
    if not (isinstance(node, IRApply) and node.head == POW and len(node.args) == 2):
        return None
    base, exp = node.args
    if base == k or not _is_constant_in(base, k):
        return None
    # Bare slope·k form (intercept = 0).
    if intercept == 0:
        if not (
            isinstance(exp, IRApply)
            and exp.head == MUL
            and len(exp.args) == 2
            and _is_int(exp.args[0], slope)
            and exp.args[1] == k
        ):
            return None
        return base
    # slope·k + intercept form.
    if not (isinstance(exp, IRApply) and exp.head == ADD and len(exp.args) == 2):
        return None
    left, right = exp.args
    if not (
        isinstance(left, IRApply)
        and left.head == MUL
        and len(left.args) == 2
        and _is_int(left.args[0], slope)
        and left.args[1] == k
    ):
        return None
    if not _is_int(right, intercept):
        return None
    return base


def _try_alt_taylor_series(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    slope: int,
    intercept: int,
    head: IRSymbol,
) -> IRNode | None:
    """Generic ``Σ_{k=0}^∞ (-1)^k · x^(slope·k + intercept) / (slope·k + intercept)!``.

    Returns ``head(x)`` (``sin(x)`` or ``cos(x)``) when the structural
    shape matches, else ``None``.  Used by :func:`_try_cos_series`
    (slope=2, intercept=0, head=COS) and :func:`_try_sin_series`
    (slope=2, intercept=1, head=SIN).

    Expected IR shape::

        MUL(POW(-1, k), DIV(POW(x, slope·k + intercept),
                            GammaFunc(slope·k + intercept + 1)))

    Symmetric variants (operand order swap, sign-1 as ``NEG(1)``) are
    accepted by the shared ``_is_neg_one_base`` helper.
    """
    if not _is_int(lo, 0):
        return None
    if not (isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2):
        return None
    a, b = f.args

    def _try_orientation(sign_term: IRNode, body: IRNode) -> IRNode | None:
        # sign_term must be (-1)^k.
        if not (
            isinstance(sign_term, IRApply)
            and sign_term.head == POW
            and len(sign_term.args) == 2
            and _is_neg_one_base(sign_term.args[0])
            and sign_term.args[1] == k
        ):
            return None
        # body must be DIV(POW(x, slope·k + intercept), GAMMA_FUNC(... + 1)).
        if not (
            isinstance(body, IRApply) and body.head == DIV and len(body.args) == 2
        ):
            return None
        numer, denom = body.args
        x = _extract_pow_of_x_in_linear_k(numer, k, slope, intercept)
        if x is None:
            return None
        if not _match_gamma_of_linear_in_k_plus_1(denom, k, slope, intercept):
            return None
        return IRApply(head, (x,))

    result = _try_orientation(a, b)
    if result is not None:
        return result
    return _try_orientation(b, a)


def _try_cos_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ (−1)^k · x^(2k) / (2k)! → cos(x)``."""
    return _try_alt_taylor_series(f, k, lo, slope=2, intercept=0, head=COS)


def _try_sin_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ (−1)^k · x^(2k+1) / (2k+1)! → sin(x)``."""
    return _try_alt_taylor_series(f, k, lo, slope=2, intercept=1, head=SIN)


def _try_hyperbolic_taylor_series(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    slope: int,
    intercept: int,
    head: IRSymbol,
) -> IRNode | None:
    """Generic ``Σ_{k=0}^∞ x^(slope·k + intercept) / (slope·k + intercept)!``.

    Returns ``head(x)`` (``cosh(x)`` or ``sinh(x)``) when the shape
    matches, else ``None``.  The sign factor is *absent* (hyperbolic
    series); the body is just ``DIV(POW(x, …), GammaFunc(… + 1))``.

    Used by :func:`_try_cosh_series` (slope=2, intercept=0, head=COSH)
    and :func:`_try_sinh_series` (slope=2, intercept=1, head=SINH).
    """
    if not _is_int(lo, 0):
        return None
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numer, denom = f.args
    x = _extract_pow_of_x_in_linear_k(numer, k, slope, intercept)
    if x is None:
        return None
    if not _match_gamma_of_linear_in_k_plus_1(denom, k, slope, intercept):
        return None
    return IRApply(head, (x,))


def _try_cosh_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ x^(2k) / (2k)! → cosh(x)``."""
    return _try_hyperbolic_taylor_series(f, k, lo, slope=2, intercept=0, head=COSH)


def _try_sinh_series(f: IRNode, k: IRSymbol, lo: IRNode) -> IRNode | None:
    """``Σ_{k=0}^∞ x^(2k+1) / (2k+1)! → sinh(x)``."""
    return _try_hyperbolic_taylor_series(f, k, lo, slope=2, intercept=1, head=SINH)


# ---------------------------------------------------------------------------
# Public dispatcher
# ---------------------------------------------------------------------------


def try_closed_form_series(
    summand: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
) -> IRNode | None:
    """Return the closed form for a recognised canonical infinite series.

    Returns ``None`` when:

    * The upper bound is not ``%inf`` (caller falls through to Gosper /
      Faulhaber / geometric / etc.).
    * The summand doesn't structurally match any of the recognised
      shapes (e.g. ``Σ 1/k³`` — odd zeta is not closed-form in
      elementary terms).
    * The lower bound doesn't match (e.g. ``Σ_{k=2}^∞ 1/k²``: spec
      requires ``lo=1`` for the zeta family).

    The recognisers are tried in declaration order; each is a structural
    pattern match with no side effects, so order matters only for
    overlapping shapes (none in this table).

    See module docstring for the full table of recognised series.
    """
    # Infinite-bound only — finite hi goes through Gosper / Faulhaber / etc.
    # Inline the inf-check rather than importing from summation.py to avoid
    # a circular import (summation.py is what *calls* us).
    if not (isinstance(hi, IRSymbol) and hi.name in {"inf", "%inf"}):
        return None

    for pattern in (
        _try_zeta_2m,
        _try_eta_2m,
        _try_eta_1,
        _try_e_series,
        _try_exp_series,
        _try_cos_series,
        _try_sin_series,
        _try_cosh_series,
        _try_sinh_series,
    ):
        result = pattern(summand, k, lo)
        if result is not None:
            return result
    return None
