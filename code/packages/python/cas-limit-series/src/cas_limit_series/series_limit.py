"""``try_series_limit`` — Taylor-series-expansion fallback for limits.

This module implements the **Track J1** addition to the limit dispatcher:
a series-expansion fallback that fires AFTER direct substitution and
L'Hôpital have already failed to close. It handles indeterminate forms
of the shape ``f(x) / g(x)`` where both numerator and denominator
vanish (``0/0``) or both diverge (``∞/∞``), and the leading-order
behaviour can be read off from a Taylor expansion around the limit
point.

Why a separate fallback?
------------------------
L'Hôpital can fail or diverge for a few reasons:

1. **Differentiation explodes the expression size**. Two passes of
   the quotient rule on ``(sin(x)-x)/x^3`` make a big mess that the
   stub eval_fn doesn't simplify enough to recognise as ``0/0``.

2. **The recursion is bounded** (``_MAX_DEPTH = 8`` in
   ``limit_advanced``). Deep cancellations such as
   ``(tan(x) - x) / x^3 = 1/3`` need three L'Hôpital steps; if any
   intermediate step doesn't simplify cleanly the recursion may bail
   out with an unevaluated ``Limit(...)`` node.

3. **Bumping the differentiation order doesn't help** if the simplifier
   isn't strong enough — but bumping the *series* order always does,
   because polynomial arithmetic stays small and exact.

Algorithm
---------
For ``limit(f(x) / g(x), x, a)`` where direct sub gives ``0/0``::

    1. Translate the limit point to the origin.
        - a = 0          ⇒ u = x
        - a finite ≠ 0   ⇒ u = x - a
        - a = +∞ / -∞    ⇒ u = 1/x (and rebrand a as 0+ in u-space).
          (Not implemented in this first version; falls through.)

    2. Taylor-expand both numerator N(u) and denominator D(u) to
       order N (starting N = 4) using a transcendental-aware engine.

    3. Read off leading coefficients::

           N(u) = c_p * u^p + O(u^(p+1))
           D(u) = d_q * u^q + O(u^(q+1))

    4. Three cases:
          p > q   →  limit = 0
          p < q   →  limit = sign(c_p / d_q) · ∞
          p == q  →  limit = c_p / d_q   (exact Fraction)

    5. If both leading coefficients are still zero after extraction
       (i.e. the order-N expansion wasn't deep enough), bump N += 2
       and retry, up to N = 12.

Bounds
------
* ``max_order`` defaults to 12 and is hard-capped.
* The series ring uses exact :class:`fractions.Fraction` arithmetic.
* No recursion: the loop iterates a fixed number of times.
* Inputs are IR nodes, not strings — no ``eval`` of user-controlled
  data anywhere.

Supported transcendentals
-------------------------
``sin``, ``cos``, ``tan``, ``exp``, ``log`` (where ``log(1+u)`` is
handled by composing ``log_one_plus`` with ``arg - 1``). Pow with
integer exponent. Polynomial expressions of any degree.

Anything else raises :class:`_SeriesError` internally; ``try_series_limit``
catches and returns ``None`` so the caller can fall through to the
unevaluated ``Limit(...)`` node.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    TAN,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_limit_series.heads import LIMIT

# ---------------------------------------------------------------------------
# Series ring
# ---------------------------------------------------------------------------
#
# A truncated power series ``a_0 + a_1·u + a_2·u² + ... + a_N·u^N`` is
# represented as ``Series(coeffs, order)`` where ``coeffs`` is a list of
# Fractions of length ``order + 1``. ``order`` is the maximum tracked
# exponent; higher terms are silently discarded (this is what truncation
# means).
#
# All operations preserve ``order``. They never mutate inputs.

#: Hard ceiling — keeps polynomial multiplication bounded by O(N²).
_MAX_ORDER_LIMIT = 12


class _SeriesError(ValueError):
    """Raised when an expression cannot be Taylor-expanded.

    Internal — caught at the public boundary and converted to
    ``None`` so the dispatcher falls through cleanly.
    """


class Series:
    """A truncated power series with rational coefficients.

    Attributes
    ----------
    coeffs:
        ``[a_0, a_1, ..., a_N]``  — exactly ``order + 1`` entries.
    order:
        The truncation order ``N``. Coefficients of ``u^k`` for
        ``k > N`` are implicitly zero.
    """

    __slots__ = ("coeffs", "order")

    def __init__(self, coeffs: list[Fraction], order: int) -> None:
        if order < 0:
            raise _SeriesError("series order must be non-negative")
        # Normalise length to order + 1 (pad with zeros or truncate).
        if len(coeffs) < order + 1:
            coeffs = coeffs + [Fraction(0)] * (order + 1 - len(coeffs))
        elif len(coeffs) > order + 1:
            coeffs = coeffs[: order + 1]
        self.coeffs = coeffs
        self.order = order

    # ---- constructors -----------------------------------------------------

    @classmethod
    def constant(cls, c: Fraction, order: int) -> Series:
        """The series ``c + 0·u + 0·u² + ...``."""
        return cls([c], order)

    @classmethod
    def variable(cls, order: int) -> Series:
        """The series ``u`` (i.e. ``0 + 1·u``)."""
        if order < 1:
            return cls([Fraction(0)], order)
        return cls([Fraction(0), Fraction(1)], order)

    # ---- arithmetic -------------------------------------------------------

    def __add__(self, other: Series) -> Series:
        n = self.order  # both have the same order in our use
        return Series(
            [self.coeffs[i] + other.coeffs[i] for i in range(n + 1)], n
        )

    def __sub__(self, other: Series) -> Series:
        n = self.order
        return Series(
            [self.coeffs[i] - other.coeffs[i] for i in range(n + 1)], n
        )

    def __neg__(self) -> Series:
        return Series([-c for c in self.coeffs], self.order)

    def __mul__(self, other: Series) -> Series:
        n = self.order
        out = [Fraction(0)] * (n + 1)
        for i in range(n + 1):
            ai = self.coeffs[i]
            if ai == 0:
                continue
            for j in range(n + 1 - i):
                out[i + j] += ai * other.coeffs[j]
        return Series(out, n)

    def scaled(self, c: Fraction) -> Series:
        """Return ``c · self`` — scalar multiplication."""
        return Series([c * a for a in self.coeffs], self.order)

    def leading_index(self) -> int:
        """Smallest ``k`` such that ``coeffs[k] != 0``, or -1 if all zero.

        ``-1`` means *we cannot resolve the leading term within the
        tracked order* — the caller should bump the order and retry.
        """
        for k, c in enumerate(self.coeffs):
            if c != 0:
                return k
        return -1

    # ---- division ---------------------------------------------------------

    def reciprocal(self) -> Series:
        """Return the series of ``1 / self`` provided ``self(0) ≠ 0``.

        Uses the classic Newton-style recursion::

            (a_0 + a_1 u + ...)·(b_0 + b_1 u + ...) = 1
            ⇒ b_0 = 1/a_0
            ⇒ b_k = -1/a_0 · sum_{j=1..k} a_j · b_{k-j}

        If ``a_0 == 0`` (the series vanishes at 0), we cannot directly
        invert; the caller is expected to factor out the leading
        ``u^p`` first.
        """
        a = self.coeffs
        n = self.order
        if a[0] == 0:
            raise _SeriesError("reciprocal of a series with zero constant term")
        b = [Fraction(0)] * (n + 1)
        b[0] = Fraction(1) / a[0]
        for k in range(1, n + 1):
            s = Fraction(0)
            for j in range(1, k + 1):
                s += a[j] * b[k - j]
            b[k] = -s / a[0]
        return Series(b, n)

    def integer_power(self, k: int) -> Series:
        """Return ``self**k`` for non-negative integer ``k``.

        Uses repeated squaring for efficiency.
        """
        if k < 0:
            raise _SeriesError("series integer_power requires k >= 0")
        if k == 0:
            return Series.constant(Fraction(1), self.order)
        result = Series.constant(Fraction(1), self.order)
        base = self
        while k > 0:
            if k & 1:
                result = result * base
            k >>= 1
            if k > 0:
                base = base * base
        return result

    # ---- composition ------------------------------------------------------

    def compose_with_zero_constant(self, inner: Series) -> Series:
        """Return ``self(inner(u))`` provided ``inner(0) == 0``.

        Composition of series only makes sense when the inner series
        vanishes at 0 — otherwise the constant term of the inner shifts
        the expansion point and the result is no longer truncated at
        the same order.

        Computed as ``sum_k a_k · inner^k`` (truncated at ``order``).
        """
        if inner.coeffs[0] != 0:
            raise _SeriesError(
                "compose_with_zero_constant: inner series has nonzero constant"
            )
        n = self.order
        result = Series.constant(Fraction(0), n)
        # Horner-style: result = a_n + u·(a_{n-1} + u·(...)) but with
        # ``u`` replaced by ``inner``. Computing inner^k explicitly is
        # simpler and stays within budget for small N.
        inner_pow = Series.constant(Fraction(1), n)  # inner^0 = 1
        for k in range(n + 1):
            if self.coeffs[k] != 0:
                result = result + inner_pow.scaled(self.coeffs[k])
            if k < n:
                inner_pow = inner_pow * inner
        return result


# ---------------------------------------------------------------------------
# Known transcendental Taylor series (around u = 0)
# ---------------------------------------------------------------------------
#
# Each function returns the truncated Taylor series of f(u) at u = 0,
# to the requested order.


def _factorial(n: int) -> int:
    out = 1
    for k in range(2, n + 1):
        out *= k
    return out


def _series_exp(order: int) -> Series:
    """``exp(u) = sum u^k / k!``."""
    return Series([Fraction(1, _factorial(k)) for k in range(order + 1)], order)


def _series_sin(order: int) -> Series:
    """``sin(u) = u - u^3/3! + u^5/5! - ...`` — odd terms only."""
    coeffs = [Fraction(0)] * (order + 1)
    sign = 1
    k = 1
    while k <= order:
        coeffs[k] = Fraction(sign, _factorial(k))
        sign = -sign
        k += 2
    return Series(coeffs, order)


def _series_cos(order: int) -> Series:
    """``cos(u) = 1 - u^2/2! + u^4/4! - ...`` — even terms only."""
    coeffs = [Fraction(0)] * (order + 1)
    sign = 1
    k = 0
    while k <= order:
        coeffs[k] = Fraction(sign, _factorial(k))
        sign = -sign
        k += 2
    return Series(coeffs, order)


def _series_log_one_plus(order: int) -> Series:
    """``log(1 + u) = u - u^2/2 + u^3/3 - ...``."""
    coeffs = [Fraction(0)] * (order + 1)
    sign = 1
    for k in range(1, order + 1):
        coeffs[k] = Fraction(sign, k)
        sign = -sign
    return Series(coeffs, order)


def _series_tan(order: int) -> Series:
    """``tan(u) = sin(u) / cos(u)``.

    We expand to *at least* ``order`` and divide. Since the series
    ring is closed under reciprocal-of-nonzero-constant (cos has
    constant term 1), this is exact.
    """
    return _series_sin(order) * _series_cos(order).reciprocal()


# ---------------------------------------------------------------------------
# IR → Series translation
# ---------------------------------------------------------------------------


def _to_fraction(node: IRNode) -> Fraction:
    """Convert a literal IR node to a Fraction. Raises on non-literals."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    if isinstance(node, IRFloat):
        # Lossy but bounded; used only as a coefficient.
        return Fraction(node.value).limit_denominator()
    raise _SeriesError(f"expected literal, got {node!r}")


def _expand(expr: IRNode, var: IRSymbol, order: int) -> Series:
    """Expand *expr* in *var* to truncated order around var = 0.

    Pure recursion: each IR head dispatches to the corresponding
    series operation. Unsupported heads raise ``_SeriesError``.

    The caller MUST have already translated the limit point to the
    origin (so ``var → 0`` is the expansion point).
    """
    # --- literal numbers ---
    if isinstance(expr, (IRInteger, IRRational, IRFloat)):
        return Series.constant(_to_fraction(expr), order)

    # --- the expansion variable ---
    if isinstance(expr, IRSymbol):
        if expr == var:
            return Series.variable(order)
        # Other symbols are opaque constants. We cannot meaningfully
        # Taylor-expand around them, so reject — the limit fallback
        # only handles single-variable inputs.
        raise _SeriesError(f"unsupported symbol {expr.name!r}")

    if not isinstance(expr, IRApply) or not isinstance(expr.head, IRSymbol):
        raise _SeriesError(f"unsupported expression: {expr!r}")

    h = expr.head
    args = expr.args

    # --- arithmetic ---
    if h == ADD:
        result = Series.constant(Fraction(0), order)
        for a in args:
            result = result + _expand(a, var, order)
        return result
    if h == SUB:
        if len(args) != 2:
            raise _SeriesError("Sub expects 2 args")
        return _expand(args[0], var, order) - _expand(args[1], var, order)
    if h == NEG:
        if len(args) != 1:
            raise _SeriesError("Neg expects 1 arg")
        return -_expand(args[0], var, order)
    if h == MUL:
        result = Series.constant(Fraction(1), order)
        for a in args:
            result = result * _expand(a, var, order)
        return result
    if h == DIV:
        if len(args) != 2:
            raise _SeriesError("Div expects 2 args")
        n_ser = _expand(args[0], var, order)
        d_ser = _expand(args[1], var, order)
        # If denominator has nonzero constant term, direct reciprocal.
        if d_ser.coeffs[0] != 0:
            return n_ser * d_ser.reciprocal()
        # Otherwise we cannot keep dividing inside the ring — the caller
        # (``try_series_limit``) handles the top-level f/g case, and any
        # *inner* DIV by a vanishing series would change the leading
        # order of the surrounding expansion. Bail out.
        raise _SeriesError(
            "inner Div by a series vanishing at 0; cannot expand at fixed order"
        )
    if h == POW:
        if len(args) != 2:
            raise _SeriesError("Pow expects 2 args")
        base, exp_node = args
        # Only non-negative integer exponents are supported. Negative
        # integers would require reciprocal (constant term must be
        # nonzero); fractional exponents are out of scope.
        if isinstance(exp_node, IRInteger):
            k = exp_node.value
            if k >= 0:
                return _expand(base, var, order).integer_power(k)
            # Negative integer: reciprocal-then-power.
            base_ser = _expand(base, var, order)
            if base_ser.coeffs[0] == 0:
                raise _SeriesError(
                    "Pow with negative integer exponent over vanishing base"
                )
            return base_ser.reciprocal().integer_power(-k)
        raise _SeriesError("Pow exponent must be a non-negative integer literal")

    # --- transcendentals ---
    if h == EXP:
        if len(args) != 1:
            raise _SeriesError("Exp expects 1 arg")
        inner = _expand(args[0], var, order)
        # exp(a + b) = exp(a) · exp(b); split off the constant term.
        c0 = inner.coeffs[0]
        # If c0 != 0, the constant factor exp(c0) is opaque (no closed
        # rational), and we cannot keep the result in the rational ring.
        if c0 != 0:
            raise _SeriesError("Exp with nonzero constant inner term")
        return _series_exp(order).compose_with_zero_constant(inner)
    if h == SIN:
        if len(args) != 1:
            raise _SeriesError("Sin expects 1 arg")
        inner = _expand(args[0], var, order)
        if inner.coeffs[0] != 0:
            raise _SeriesError("Sin with nonzero constant inner term")
        return _series_sin(order).compose_with_zero_constant(inner)
    if h == COS:
        if len(args) != 1:
            raise _SeriesError("Cos expects 1 arg")
        inner = _expand(args[0], var, order)
        if inner.coeffs[0] != 0:
            raise _SeriesError("Cos with nonzero constant inner term")
        return _series_cos(order).compose_with_zero_constant(inner)
    if h == TAN:
        if len(args) != 1:
            raise _SeriesError("Tan expects 1 arg")
        inner = _expand(args[0], var, order)
        if inner.coeffs[0] != 0:
            raise _SeriesError("Tan with nonzero constant inner term")
        return _series_tan(order).compose_with_zero_constant(inner)
    if h == LOG:
        if len(args) != 1:
            raise _SeriesError("Log expects 1 arg")
        inner = _expand(args[0], var, order)
        # log requires inner(0) == 1: log(1 + (inner - 1)) where
        # (inner - 1)(0) == 0.
        c0 = inner.coeffs[0]
        if c0 != 1:
            raise _SeriesError(
                f"Log with constant inner term != 1 (got {c0}); not in rational ring"
            )
        shifted = Series(
            [Fraction(0)] + inner.coeffs[1:], order
        )  # inner - 1
        return _series_log_one_plus(order).compose_with_zero_constant(shifted)

    raise _SeriesError(f"unsupported head: {h}")


# ---------------------------------------------------------------------------
# Top-level entry point
# ---------------------------------------------------------------------------


def _split_quotient(expr: IRNode) -> tuple[IRNode, IRNode] | None:
    """Return ``(numer, denom)`` if *expr* looks like a quotient.

    Recognises both ``Div(N, D)`` and ``Mul(N, Pow(D, -1))`` shapes.
    Returns ``None`` for any other top-level shape — the Taylor
    fallback only fires on rational structures.
    """
    if not isinstance(expr, IRApply):
        return None
    if expr.head == DIV and len(expr.args) == 2:
        return expr.args[0], expr.args[1]
    if expr.head == MUL and len(expr.args) == 2:
        a, b = expr.args
        if (
            isinstance(b, IRApply)
            and b.head == POW
            and len(b.args) == 2
            and isinstance(b.args[1], IRInteger)
            and b.args[1].value == -1
        ):
            return a, b.args[0]
        if (
            isinstance(a, IRApply)
            and a.head == POW
            and len(a.args) == 2
            and isinstance(a.args[1], IRInteger)
            and a.args[1].value == -1
        ):
            return b, a.args[0]
    return None


def _shift_to_origin(expr: IRNode, var: IRSymbol, point: IRNode) -> IRNode:
    """Substitute ``var := var + point`` so ``var = 0`` corresponds to
    the original ``var = point``.

    We do NOT use the package's ``cas_substitution`` here because the
    Series expander walks the IR directly and we want the
    substitution to be totally explicit (no reliance on a substituter
    we don't control).
    """
    if isinstance(point, IRInteger) and point.value == 0:
        return expr  # no shift needed

    def _go(node: IRNode) -> IRNode:
        if isinstance(node, IRSymbol):
            if node == var:
                return IRApply(ADD, (var, point))
            return node
        if isinstance(node, IRApply):
            return IRApply(node.head, tuple(_go(a) for a in node.args))
        return node

    return _go(expr)


def _coeff_to_ir(c: Fraction) -> IRNode:
    """Convert a Fraction back to an IR literal node."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def try_series_limit(
    expr: IRNode,
    var: IRSymbol,
    point: IRNode,
    *,
    max_order: int = _MAX_ORDER_LIMIT,
) -> IRNode | None:
    """Taylor-series fallback for ``limit(expr, var, point)``.

    Fires when direct substitution and L'Hôpital have both failed to
    close. Returns:

    * an :class:`IRInteger` / :class:`IRRational` literal on success,
    * ``LIMIT.INF`` / ``LIMIT.MINF`` symbol on a divergent ratio,
    * ``None`` if the fallback cannot determine the value (caller
      should fall through to an unevaluated ``Limit(...)``).

    Parameters
    ----------
    expr:
        The expression whose limit is sought. Must have a rational
        ``f / g`` top-level structure — anything else returns ``None``.
    var:
        The limit variable (an ``IRSymbol``).
    point:
        The limit point. Must be a literal number (``IRInteger``,
        ``IRRational``, ``IRFloat``). Limits at ±∞ are not yet
        handled by this fallback and return ``None``.
    max_order:
        Hard ceiling on the Taylor-expansion order (clamped at 12 to
        keep polynomial multiplication bounded by O(N²)).
    """
    # --- safety bounds ---
    if max_order < 4:
        max_order = 4
    if max_order > _MAX_ORDER_LIMIT:
        max_order = _MAX_ORDER_LIMIT

    # --- top-level shape: must be a quotient ---
    nd = _split_quotient(expr)
    if nd is None:
        return None
    numer, denom = nd

    # --- limit at ±∞ → not yet handled (would need u = 1/x rewrite) ---
    if isinstance(point, IRSymbol) and point.name in ("inf", "minf"):
        return None
    if not isinstance(point, (IRInteger, IRRational, IRFloat)):
        return None

    # --- shift the expansion point to the origin ---
    shifted_n = _shift_to_origin(numer, var, point)
    shifted_d = _shift_to_origin(denom, var, point)

    # --- iterate orders ---
    order = 4
    while order <= max_order:
        try:
            n_ser = _expand(shifted_n, var, order)
            d_ser = _expand(shifted_d, var, order)
        except _SeriesError:
            return None

        p = n_ser.leading_index()
        q = d_ser.leading_index()

        # Both fully zero → degenerate case; the limit is genuinely
        # 0/0 with no resolvable leading term. Bump order and try again.
        if p == -1 and q == -1:
            order += 2
            continue

        # Numerator zero out to tracked order but denominator nonzero →
        # the limit is 0 (numerator vanishes faster than any power we
        # tracked, denominator does not).
        if p == -1 and q != -1:
            return IRInteger(0)

        # Denominator zero but numerator nonzero → divergence. Sign
        # follows the leading coefficient.
        if q == -1 and p != -1:
            sign = 1 if n_ser.coeffs[p] > 0 else -1
            return IRSymbol("inf") if sign > 0 else IRSymbol("minf")

        # Both leading orders found. Apply the three-way rule.
        c_p = n_ser.coeffs[p]
        d_q = d_ser.coeffs[q]
        if p > q:
            return IRInteger(0)
        if p < q:
            # c_p / d_q · u^(p-q) with p < q → 1/u^(q-p) · finite → ±∞
            sign_val = c_p / d_q
            return IRSymbol("inf") if sign_val > 0 else IRSymbol("minf")
        # p == q
        ratio = c_p / d_q
        return _coeff_to_ir(ratio)

    # Out of budget — caller falls through to unevaluated Limit(...).
    return None


__all__ = ["try_series_limit"]


# ---------------------------------------------------------------------------
# Re-export of LIMIT head for convenience — the dispatcher in
# ``limit_advanced.py`` imports this symbol when wiring the fallback.
# ---------------------------------------------------------------------------
_LIMIT = LIMIT  # noqa: N816 — match the spec's naming
