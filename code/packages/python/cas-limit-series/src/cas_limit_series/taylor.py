"""``taylor_polynomial`` — Taylor expansion of a polynomial expression.

For a polynomial ``p(var)``, the Taylor series around ``point`` to
order ``order`` is::

    p(var) = sum_{k=0..order} p^{(k)}(point) / k! * (var - point)^k

We build this exactly using a pure-Python polynomial differentiator —
no dependency on the symbolic-vm derivative handler, which keeps the
package leaf-light and easy to reason about.

Inputs accepted: ``Add``, ``Mul``, ``Pow``, ``Neg``, integer / rational
literals, and a single variable symbol. Anything else (transcendental
functions, other variables) raises :class:`PolynomialError`. The full
transcendental-Taylor case lives in a follow-up PR that pulls in the
general differentiation hook.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)


class PolynomialError(ValueError):
    """Raised when an expression contains non-polynomial parts."""


# ---------------------------------------------------------------------------
# IR ↔ polynomial coefficient list
# ---------------------------------------------------------------------------


def _to_coefficients(expr: IRNode, var: IRSymbol) -> list[Fraction]:
    """Convert an IR polynomial in ``var`` to a coefficient list.

    Returns ``[a_0, a_1, ..., a_n]`` such that
    ``p(var) = sum a_i * var^i``. Raises :class:`PolynomialError` for
    any non-polynomial term.
    """
    if isinstance(expr, IRInteger):
        return [Fraction(expr.value)]
    if isinstance(expr, IRRational):
        return [Fraction(expr.numer, expr.denom)]
    if isinstance(expr, IRFloat):
        # We accept floats but convert to Fraction (lossy but exact-after).
        return [Fraction(expr.value).limit_denominator()]
    if isinstance(expr, IRSymbol):
        if expr == var:
            return [Fraction(0), Fraction(1)]
        # Constant non-target symbols are treated as opaque coefficients
        # only at the constant slot.
        raise PolynomialError(
            f"taylor: expression contains symbol {expr.name!r} other than {var.name!r}"
        )
    if isinstance(expr, IRApply) and isinstance(expr.head, IRSymbol):
        head = expr.head.name
        if head == "Add":
            result: list[Fraction] = [Fraction(0)]
            for arg in expr.args:
                term = _to_coefficients(arg, var)
                result = _coeffs_add(result, term)
            return result
        if head == "Sub":
            if len(expr.args) != 2:
                raise PolynomialError("Sub must have exactly 2 args")
            a = _to_coefficients(expr.args[0], var)
            b = _to_coefficients(expr.args[1], var)
            return _coeffs_sub(a, b)
        if head == "Neg":
            if len(expr.args) != 1:
                raise PolynomialError("Neg must have exactly 1 arg")
            return [-c for c in _to_coefficients(expr.args[0], var)]
        if head == "Mul":
            result = [Fraction(1)]
            for arg in expr.args:
                term = _to_coefficients(arg, var)
                result = _coeffs_mul(result, term)
            return result
        if head == "Pow":
            if len(expr.args) != 2:
                raise PolynomialError("Pow must have exactly 2 args")
            base, exp = expr.args
            if not isinstance(exp, IRInteger) or exp.value < 0:
                raise PolynomialError(
                    "Pow exponent must be a non-negative integer literal"
                )
            base_coeffs = _to_coefficients(base, var)
            result = [Fraction(1)]
            for _ in range(exp.value):
                result = _coeffs_mul(result, base_coeffs)
            return result
        if head == "Div":
            if len(expr.args) != 2:
                raise PolynomialError("Div must have exactly 2 args")
            num, den = expr.args
            if not isinstance(den, IRInteger | IRRational):
                raise PolynomialError(
                    "Div: denominator must be a numeric literal for polynomial Taylor"
                )
            num_coeffs = _to_coefficients(num, var)
            den_value = (
                Fraction(den.value)
                if isinstance(den, IRInteger)
                else Fraction(den.numer, den.denom)
            )
            return [c / den_value for c in num_coeffs]
    raise PolynomialError(
        f"taylor: unsupported expression for polynomial input: {expr!r}"
    )


def _coeffs_add(a: list[Fraction], b: list[Fraction]) -> list[Fraction]:
    n = max(len(a), len(b))
    return [
        (a[i] if i < len(a) else Fraction(0))
        + (b[i] if i < len(b) else Fraction(0))
        for i in range(n)
    ]


def _coeffs_sub(a: list[Fraction], b: list[Fraction]) -> list[Fraction]:
    n = max(len(a), len(b))
    return [
        (a[i] if i < len(a) else Fraction(0))
        - (b[i] if i < len(b) else Fraction(0))
        for i in range(n)
    ]


def _coeffs_mul(a: list[Fraction], b: list[Fraction]) -> list[Fraction]:
    out = [Fraction(0)] * (len(a) + len(b) - 1)
    for i, ai in enumerate(a):
        for j, bj in enumerate(b):
            out[i + j] += ai * bj
    return out


# ---------------------------------------------------------------------------
# Coefficient list → IR polynomial
# ---------------------------------------------------------------------------


def _from_coefficients(
    coeffs: list[Fraction], var: IRSymbol, *, point: IRNode
) -> IRNode:
    """Rebuild ``sum c_k * (var - point)^k`` as an IR Add."""
    terms: list[IRNode] = []
    for k, c in enumerate(coeffs):
        if c == 0:
            continue
        coef_node = _coeff_to_ir(c)
        if k == 0:
            terms.append(coef_node)
            continue
        # (var - point)^k
        delta = (
            var
            if (isinstance(point, IRInteger) and point.value == 0)
            else IRApply(SUB, (var, point))
        )
        base = delta if k == 1 else IRApply(POW, (delta, IRInteger(k)))
        if c == 1:
            terms.append(base)
        else:
            terms.append(IRApply(MUL, (coef_node, base)))
    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]
    return IRApply(ADD, tuple(terms))


def _coeff_to_ir(c: Fraction) -> IRNode:
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


# ---------------------------------------------------------------------------
# Public Taylor entry point
# ---------------------------------------------------------------------------


def taylor_polynomial(
    expr: IRNode,
    var: IRSymbol,
    point: IRNode,
    order: int,
) -> IRNode:
    """Truncated Taylor expansion of a polynomial ``expr`` around ``point``.

    The expansion is::

        sum_{k=0..order} (1/k!) * p^{(k)}(point) * (var - point)^k

    The result is an un-simplified ``IRApply(Add, ...)``; pass through
    ``cas_simplify.simplify`` to clean it up.

    Supports polynomial inputs only — see module docstring for the
    accepted shape. Raises :class:`PolynomialError` on transcendentals.
    """
    if order < 0:
        raise ValueError("taylor: order must be non-negative")

    coeffs_in_x = _to_coefficients(expr, var)
    # Shift the polynomial: substitute (var - point)+point for var,
    # equivalent to expanding around ``point``. We do it via
    # coefficient arithmetic to stay efficient.
    coeffs_in_delta = _shift_polynomial(coeffs_in_x, _to_fraction(point))

    # Truncate to order.
    truncated = coeffs_in_delta[: order + 1]
    return _from_coefficients(truncated, var, point=point)


def _to_fraction(node: IRNode) -> Fraction:
    """Convert a literal IR node to a Fraction. Raises on non-literals."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    if isinstance(node, IRFloat):
        return Fraction(node.value).limit_denominator()
    raise PolynomialError(
        f"taylor: expansion point must be a literal number, got {node!r}"
    )


def _shift_polynomial(coeffs: list[Fraction], shift: Fraction) -> list[Fraction]:
    """Return coefficients of ``p(var)`` rewritten as ``q(var - shift)``.

    Algorithm: q_k = (1/k!) * p^(k)(shift) where the k-th derivative of
    ``sum a_i x^i`` is ``sum i*(i-1)*...*(i-k+1) a_i x^(i-k)``.
    Evaluating at ``shift`` and dividing by ``k!``.
    """
    n = len(coeffs)
    out: list[Fraction] = []
    for k in range(n):
        # Compute p^(k)(shift) / k!
        sub_total = Fraction(0)
        for i in range(k, n):
            # falling factorial i*(i-1)*...*(i-k+1)
            falling = math.factorial(i) // math.factorial(i - k)
            sub_total += Fraction(falling) * coeffs[i] * (shift ** (i - k))
        out.append(sub_total / math.factorial(k))
    return out


