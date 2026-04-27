"""Polynomial × sin/cos integration — Phase 4a.

Integrates ``p(x) · sin(ax+b)`` and ``p(x) · cos(ax+b)`` for ``p ∈ Q[x]``
and ``a ∈ Q \\ {0}`` via the tabular integration-by-parts formula.

Applying IBP ``deg(p) + 1`` times with ``u = p`` (differentiated each step)
and ``dv = sin(ax+b) dx`` (integrated each step) yields a finite telescoping
sum that groups into two coefficient polynomials:

    C(x) = Σ_{k=0,2,4,…} (−1)^(k/2)  · p^(2k)(x) / a^(2k+1)  (even derivs)
    S(x) = Σ_{k=0,2,4,…} (−1)^k       · p^(2k+1)(x) / a^(2k+2) (odd derivs)

Closed forms:

    ∫ p(x)·sin(ax+b) dx = sin(ax+b)·S(x) − cos(ax+b)·C(x)
    ∫ p(x)·cos(ax+b) dx = sin(ax+b)·C(x) + cos(ax+b)·S(x)

Both cases share the same ``_cs_coeffs`` computation — only the IR
assembly differs.

See ``code/specs/phase4-trig-integration.md`` for the full derivation
and worked examples.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import Polynomial, add, deriv, normalize
from symbolic_ir import (
    ADD,
    COS,
    MUL,
    NEG,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def trig_sin_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ poly(x)·sin(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    Result shape: ``sin(ax+b)·S(x) − cos(ax+b)·C(x)`` where C and S are
    the tabular coefficient polynomials.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)
    C, S = _cs_coeffs(p, a)
    arg_ir = linear_to_ir(a, b, x_sym)
    sin_ir = IRApply(SIN, (arg_ir,))
    cos_ir = IRApply(COS, (arg_ir,))
    # sin·S − cos·C
    return _assemble(sin_ir, S, cos_ir, C, x_sym, subtract=True)


def trig_cos_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ poly(x)·cos(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    Result shape: ``sin(ax+b)·C(x) + cos(ax+b)·S(x)`` where C and S are
    the tabular coefficient polynomials.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)
    C, S = _cs_coeffs(p, a)
    arg_ir = linear_to_ir(a, b, x_sym)
    sin_ir = IRApply(SIN, (arg_ir,))
    cos_ir = IRApply(COS, (arg_ir,))
    # sin·C + cos·S
    return _assemble(sin_ir, C, cos_ir, S, x_sym, subtract=False)


def _cs_coeffs(poly: Polynomial, a: Fraction) -> tuple[Polynomial, Polynomial]:
    """Compute coefficient polynomials C and S for the tabular IBP formula.

    For index ``k`` of the derivative sequence:
    - ``sign = (−1)^(k // 2)``  — gives the pattern +1, +1, −1, −1, +1, +1, …
    - ``scale = sign / a^(k+1)``
    - Even k contributes to C; odd k contributes to S.

    Both sums terminate when a derivative reaches zero.
    """
    # Build derivative sequence: [p, p', p'', …, 0].
    p_frac = tuple(Fraction(c) for c in normalize(poly))
    derivs: list[Polynomial] = [p_frac]
    while normalize(derivs[-1]):
        nxt = tuple(Fraction(c) for c in normalize(deriv(derivs[-1])))
        derivs.append(nxt)

    c_acc: Polynomial = ()
    s_acc: Polynomial = ()
    for k, dk in enumerate(derivs):
        if not normalize(dk):
            break
        sign = Fraction((-1) ** (k // 2))
        scale = sign / a ** (k + 1)
        scaled = tuple(c * scale for c in dk)
        if k % 2 == 0:
            c_acc = add(c_acc, scaled)
        else:
            s_acc = add(s_acc, scaled)
    return normalize(c_acc), normalize(s_acc)


def _assemble(
    trig1_ir: IRNode,
    poly1: Polynomial,
    trig2_ir: IRNode,
    poly2: Polynomial,
    x_sym: IRSymbol,
    *,
    subtract: bool,
) -> IRNode:
    """Build ``trig1·poly1 ± trig2·poly2`` as IR, dropping zero terms."""
    has1 = bool(normalize(poly1))
    has2 = bool(normalize(poly2))

    if not has1 and not has2:
        return IRInteger(0)

    t1 = _poly_trig(poly1, trig1_ir, x_sym) if has1 else None
    t2 = _poly_trig(poly2, trig2_ir, x_sym) if has2 else None

    if t1 is None:
        # Only t2 present; sign: subtract means result is −t2, else +t2.
        return IRApply(NEG, (t2,)) if subtract else t2  # type: ignore[arg-type]
    if t2 is None:
        return t1

    if subtract:
        return IRApply(SUB, (t1, t2))
    return IRApply(ADD, (t1, t2))


def _poly_trig(coef: Polynomial, trig_ir: IRNode, x_sym: IRSymbol) -> IRNode:
    """Build ``coef(x) · trig_ir``, collapsing ±1 coefficients."""
    coef_ir = from_polynomial(coef, x_sym)
    if isinstance(coef_ir, IRInteger):
        if coef_ir.value == 1:
            return trig_ir
        if coef_ir.value == -1:
            return IRApply(NEG, (trig_ir,))
    return IRApply(MUL, (coef_ir, trig_ir))


__all__ = ["trig_cos_integral", "trig_sin_integral"]
