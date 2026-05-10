"""Symbolic product evaluation: Π_{k=a}^{b} f(k).

The ``product(f, k, a, b)`` function evaluates finite products to closed
forms for the following families:

1. **Constant factor** — f does not depend on k:

       Π_{k=a}^{b} c  =  c^(b − a + 1)

2. **Identity product** — f = k, lower bound = 1:

       Π_{k=1}^{n} k  =  n!  =  GammaFunc(n + 1)

   We use GammaFunc (Phase 23) rather than introducing a separate FACTORIAL
   head.  GammaFunc(n+1) = n! for positive integers.

3. **Scaled identity** — f = c·k, lower bound = 1:

       Π_{k=1}^{n} c·k  =  c^n · n!  =  c^n · GammaFunc(n + 1)

4. **Concrete numeric bounds** — when both lo and hi are small integers,
   compute the product directly as a numeric integer node.

Usage::

    from symbolic_ir import IRSymbol, IRInteger
    from cas_summation.product_eval import evaluate_product_expr

    n = IRSymbol("n")
    k = IRSymbol("k")
    # product(k, k, 1, n)  →  GammaFunc(n+1)
    expr = evaluate_product_expr(k, k, IRInteger(1), n, vm=None)
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    GAMMA_FUNC,
    MUL,
    POW,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _frac(c: Fraction) -> IRNode:
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _gamma(n: IRNode) -> IRNode:
    """GammaFunc(n+1) = n!"""
    return IRApply(GAMMA_FUNC, (IRApply(ADD, (n, _int(1))),))


def _is_int(node: IRNode, value: int) -> bool:
    return isinstance(node, IRInteger) and node.value == value


def _ir_int_val(node: IRNode) -> int | None:
    """Return the integer value of an IRInteger, or None."""
    return node.value if isinstance(node, IRInteger) else None


def _ir_rational_val(node: IRNode) -> Fraction | None:
    """Return the rational value of an integer or rational IR node, or None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _is_constant_in(f: IRNode, k: IRSymbol) -> bool:
    """True iff *f* does not contain *k* anywhere."""
    if f == k:
        return False
    if isinstance(f, IRApply):
        return all(_is_constant_in(arg, k) for arg in f.args)
    return True


def _split_linear_coeff(f: IRNode, k: IRSymbol) -> tuple[Fraction, bool] | None:
    """If f = c*k (c rational, k the index), return (c, True); else None.

    Handles:
    - f == k → (Fraction(1), True)
    - f = MUL(c, k) where c is rational → (c, True)
    - f = MUL(k, c) where c is rational → (c, True)
    """
    if f == k:
        return Fraction(1), True
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        a, b = f.args
        c_a = _ir_rational_val(a)
        c_b = _ir_rational_val(b)
        if c_a is not None and b == k:
            return c_a, True
        if c_b is not None and a == k:
            return c_b, True
    return None


# ---------------------------------------------------------------------------
# Public evaluator
# ---------------------------------------------------------------------------


def evaluate_product_expr(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm: object,
) -> IRNode | None:
    """Attempt to evaluate Π_{k=lo}^{hi} f(k) in closed form, or return None.

    Parameters
    ----------
    f:
        The factor expression (may depend on *k*).
    k:
        The product index variable.
    lo:
        The lower bound IR node (evaluated by the VM before calling here).
    hi:
        The upper bound IR node (evaluated by the VM before calling here).
    vm:
        The symbolic VM (used only for numeric evaluation of concrete products).

    Returns
    -------
    IRNode | None
        Closed form, or ``None`` if the product is not recognised.
    """
    # ── Case 1: constant factor ─────────────────────────────────────────────
    # Π_{k=a}^{b} c = c^(b − a + 1)
    if _is_constant_in(f, k):
        from symbolic_ir import SUB

        span = IRApply(ADD, (IRApply(SUB, (hi, lo)), _int(1)))
        return _pow(f, span)

    # ── Case 2: product(k, k, 1, n) = n! = GammaFunc(n+1) ─────────────────
    if _is_int(lo, 1) and f == k:
        return _gamma(hi)

    # ── Case 3: product(c*k, k, 1, n) = c^n * GammaFunc(n+1) ──────────────
    if _is_int(lo, 1):
        result = _split_linear_coeff(f, k)
        if result is not None and result[1]:
            c, _ = result
            # c^n * n!
            if c == Fraction(1):
                return _gamma(hi)
            coeff_ir = _frac(c)
            return _mul(_pow(coeff_ir, hi), _gamma(hi))

    # ── Case 4: numeric small product ──────────────────────────────────────
    lo_int = _ir_int_val(lo)
    hi_int = _ir_int_val(hi)
    if (
        lo_int is not None
        and hi_int is not None
        and hi_int - lo_int <= 20
        and vm is not None
    ):
        # Compute numerically by substituting k = lo, lo+1, …, hi.
        # We can only do this if f is a polynomial/rational expression in k
        # that we can evaluate numerically.  Use the vm if available.
        try:
            from cas_substitution import subst

            product_val = Fraction(1)
            for kv in range(lo_int, hi_int + 1):
                k_node = IRInteger(kv)
                term = subst(k_node, k, f)
                evaluated = vm.eval(term)
                r = _ir_rational_val(evaluated)
                if r is None:
                    return None
                product_val *= r
            return _frac(product_val)
        except Exception:
            pass

    return None
