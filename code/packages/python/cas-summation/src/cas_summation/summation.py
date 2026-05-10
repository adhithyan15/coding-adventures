"""Main dispatcher for symbolic summation and product evaluation.

This module provides two public functions:

- ``evaluate_sum(f, k, lo, hi, vm)``   — evaluates Σ_{k=lo}^{hi} f(k)
- ``evaluate_product(f, k, lo, hi, vm)`` — evaluates Π_{k=lo}^{hi} f(k)

Both return an IR node representing the closed form, or return the
unevaluated ``SUM``/``PRODUCT`` node when no pattern matches.

Dispatch order for ``evaluate_sum``
------------------------------------

1. **Constant** — f does not contain k:
       Σ c = c · (hi − lo + 1)

2. **Geometric** — f = coeff · base^k (base constant in k):
       Finite:   coeff · base^lo · (base^(hi−lo+1) − 1) / (base − 1)
       Infinite: coeff · base^lo / (1 − base)

3. **Power of index** — f = coeff · k^m (m = 0…5):
       Uses Faulhaber's formula  Σ_{k=lo}^{hi} k^m = F(hi,m) − F(lo−1,m)

4. **Classic infinite series** — when hi = %inf (or inf):
       Σ 1/k²        → π²/6
       Σ 1/k⁴        → π⁴/90
       Σ (-1)^k/(2k+1) → π/4
       Σ 1/k!        → %e
       Σ x^k/k!      → exp(x)

5. **Numeric small range** — lo and hi are concrete integers (range ≤ 1000):
       Compute directly via repeated substitution + VM eval.

6. **Fallback** — return unevaluated ``SUM(f, k, lo, hi)``.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    PRODUCT,
    SUB,
    SUM,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_summation.geometric_sum import geometric_sum_ir
from cas_summation.poly_sum import poly_sum_ir
from cas_summation.product_eval import evaluate_product_expr
from cas_summation.special_sums import try_special_infinite

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _frac(c: Fraction) -> IRNode:
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _is_inf(node: IRNode) -> bool:
    """True iff *node* represents +∞ (both MACSYMA ``%inf`` and raw ``inf``)."""
    return isinstance(node, IRSymbol) and node.name in {"inf", "%inf"}


def _is_constant_in(f: IRNode, k: IRSymbol) -> bool:
    """True iff *f* contains no occurrence of *k*."""
    if f == k:
        return False
    if isinstance(f, IRApply):
        return all(_is_constant_in(arg, k) for arg in f.args)
    return True


def _ir_rational_val(node: IRNode) -> Fraction | None:
    """Return the value of an integer/rational IR literal, or None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _ir_int_val(node: IRNode) -> int | None:
    """Return the Python int value of an IRInteger, or None."""
    return node.value if isinstance(node, IRInteger) else None


def _try_geometric(
    f: IRNode, k: IRSymbol
) -> tuple[IRNode, IRNode] | None:
    """If f = coeff · base^k, return (coeff, base); else None.

    Handles:
    - base^k           → (IRInteger(1), base)
    - coeff * base^k   → (coeff, base)   where coeff is constant in k
    - base^k * coeff   → (coeff, base)
    """
    # Direct: base^k  where base is constant in k (and base ≠ k)
    if (
        isinstance(f, IRApply)
        and f.head == POW
        and len(f.args) == 2
        and f.args[1] == k
        and _is_constant_in(f.args[0], k)
        and f.args[0] != k
    ):
        return (IRInteger(1), f.args[0])

    # MUL(c, base^k) or MUL(base^k, c)
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        a, b = f.args
        for coeff_cand, pow_cand in ((a, b), (b, a)):
            if (
                isinstance(pow_cand, IRApply)
                and pow_cand.head == POW
                and len(pow_cand.args) == 2
                and pow_cand.args[1] == k
                and _is_constant_in(pow_cand.args[0], k)
                and pow_cand.args[0] != k
                and _is_constant_in(coeff_cand, k)
            ):
                return (coeff_cand, pow_cand.args[0])

    return None


def _try_power_of_k(
    f: IRNode, k: IRSymbol
) -> tuple[Fraction, int] | None:
    """If f = coeff · k^m (m non-negative integer ≤ 5), return (coeff, m); else None.

    Handles:
    - k            → (Fraction(1), 1)
    - k^m          → (Fraction(1), m)
    - coeff * k^m  → (coeff, m)
    - coeff * k    → (coeff, 1)
    """
    # f == k  (i.e. k^1 with coeff=1)
    if f == k:
        return Fraction(1), 1

    # f = POW(k, m)
    if (
        isinstance(f, IRApply)
        and f.head == POW
        and len(f.args) == 2
        and f.args[0] == k
    ):
        m = _ir_int_val(f.args[1])
        if m is not None and 0 <= m <= 5:
            return Fraction(1), m

    # f = MUL(coeff, k) or MUL(k, coeff)
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        a, b = f.args
        for coeff_cand, other in ((a, b), (b, a)):
            c = _ir_rational_val(coeff_cand)
            if c is None or not _is_constant_in(coeff_cand, k):
                continue
            # other must be k or POW(k, m)
            if other == k:
                return c, 1
            if (
                isinstance(other, IRApply)
                and other.head == POW
                and len(other.args) == 2
                and other.args[0] == k
            ):
                m = _ir_int_val(other.args[1])
                if m is not None and 0 <= m <= 5:
                    return c, m

    return None


# ---------------------------------------------------------------------------
# Main public functions
# ---------------------------------------------------------------------------


def evaluate_sum(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm: object,
) -> IRNode:
    """Evaluate Σ_{k=lo}^{hi} f(k) symbolically, or return unevaluated SUM.

    Parameters
    ----------
    f:
        The summand expression (may contain *k*).
    k:
        The index variable (an ``IRSymbol``).
    lo:
        Lower bound (already evaluated by the VM).
    hi:
        Upper bound (already evaluated by the VM).
    vm:
        The symbolic VM instance (used for sub-expression evaluation).

    Returns
    -------
    IRNode
        Closed-form result, or ``IRApply(SUM, (f, k, lo, hi))`` if
        no pattern matches.
    """
    inf_upper = _is_inf(hi)

    # ── 1. Constant summand ─────────────────────────────────────────────────
    if _is_constant_in(f, k):
        # Σ_{k=lo}^{hi} c = c * (hi - lo + 1)
        count = IRApply(ADD, (IRApply(SUB, (hi, lo)), _int(1)))
        return vm.eval(IRApply(MUL, (f, count)))

    # ── 2. Geometric series ─────────────────────────────────────────────────
    geo = _try_geometric(f, k)
    if geo is not None:
        coeff, base = geo
        raw = geometric_sum_ir(
            coeff=coeff,
            base=base,
            lo=lo,
            hi=hi,
            is_infinite=inf_upper,
        )
        return vm.eval(raw)

    # ── 3. Power of index (Faulhaber) ───────────────────────────────────────
    power = _try_power_of_k(f, k)
    if power is not None:
        coeff, m = power
        lo_int = _ir_int_val(lo)
        if lo_int is not None and lo_int >= 0 and not inf_upper:
            raw = poly_sum_ir(m=m, coeff=coeff, lo_val=lo_int, hi=hi)
            if raw is not None:
                return vm.eval(raw)

    # ── 4. Classic infinite series ──────────────────────────────────────────
    if inf_upper:
        result = try_special_infinite(f, k, lo)
        if result is not None:
            return vm.eval(result)

    # ── 5. Numeric small range ──────────────────────────────────────────────
    lo_int = _ir_int_val(lo)
    hi_int = _ir_int_val(hi)
    if lo_int is not None and hi_int is not None and 0 <= hi_int - lo_int <= 999:
        try:
            from cas_substitution import subst

            total = Fraction(0)
            for kv in range(lo_int, hi_int + 1):
                term = subst(IRInteger(kv), k, f)
                evaluated = vm.eval(term)
                r = _ir_rational_val(evaluated)
                if r is None:
                    total = None  # type: ignore[assignment]
                    break
                total += r
            if total is not None:
                return _frac(total)
        except Exception:
            pass

    # ── 6. Unevaluated ──────────────────────────────────────────────────────
    return IRApply(SUM, (f, k, lo, hi))


def evaluate_product(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm: object,
) -> IRNode:
    """Evaluate Π_{k=lo}^{hi} f(k) symbolically, or return unevaluated PRODUCT.

    Parameters
    ----------
    f:
        The factor expression (may contain *k*).
    k:
        The product index variable (an ``IRSymbol``).
    lo:
        Lower bound (already evaluated by the VM).
    hi:
        Upper bound (already evaluated by the VM).
    vm:
        The symbolic VM instance.

    Returns
    -------
    IRNode
        Closed-form result, or ``IRApply(PRODUCT, (f, k, lo, hi))`` if
        no pattern matches.
    """
    result = evaluate_product_expr(f, k, lo, hi, vm)
    if result is not None:
        return vm.eval(result)
    return IRApply(PRODUCT, (f, k, lo, hi))
