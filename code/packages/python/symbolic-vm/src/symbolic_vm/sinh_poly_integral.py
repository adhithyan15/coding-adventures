"""Polynomial × sinh/cosh integration — Phase 13.

Integrates ``P(x) · sinh(ax+b)`` and ``P(x) · cosh(ax+b)`` for ``P ∈ Q[x]``
and ``a ∈ Q \\ {0}`` via the tabular integration-by-parts formula.

The antiderivative cycle for ``sinh(ax+b)`` alternates between sinh and cosh:

    ∫¹ sinh(ax+b) = (1/a)·cosh(ax+b)
    ∫² sinh(ax+b) = (1/a²)·sinh(ax+b)
    ...

Applying tabular IBP with ``u = P(x)`` (differentiated) and sign alternation
``(−1)^k`` (period 2 — unlike trig's period 4) yields:

    C(x) = Σ_{k even} (1/a^(k+1)) · P^(k)(x)    — coefficient of cosh
    S(x) = Σ_{k odd}  (−1/a^(k+1)) · P^(k)(x)   — coefficient of sinh

Closed forms:

    ∫ P(x)·sinh(ax+b) dx = C(x)·cosh(ax+b) + S(x)·sinh(ax+b)
    ∫ P(x)·cosh(ax+b) dx = C(x)·sinh(ax+b) + S(x)·cosh(ax+b)

Both cases share the same ``_cs_coeffs`` computation.  The only difference is
the assembly: for sinh, C multiplies cosh and S multiplies sinh; for cosh it's
the reverse.

The sign alternation ``(−1)^k`` distinguishes this from
``trig_poly_integral.py``, where the trig identity ``d²/dx² sin = −sin``
introduces an extra sign flip every two steps (pattern ``(−1)^(k//2)``).
For hyperbolic functions ``d²/dx² sinh = +sinh``, so no extra flip occurs.

See ``code/specs/phase13-hyperbolic.md`` for the full derivation and worked
examples.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import Polynomial, add, deriv, normalize
from symbolic_ir import (
    ADD,
    COSH,
    MUL,
    NEG,
    SINH,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def sinh_poly_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ poly(x)·sinh(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    Result shape: ``C(x)·cosh(ax+b) + S(x)·sinh(ax+b)`` where C and S are
    the tabular coefficient polynomials.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)
    C, S = _cs_coeffs(p, a)
    arg_ir = linear_to_ir(a, b, x_sym)
    sinh_ir = IRApply(SINH, (arg_ir,))
    cosh_ir = IRApply(COSH, (arg_ir,))
    # C·cosh + S·sinh
    return _assemble(cosh_ir, C, sinh_ir, S, x_sym)


def cosh_poly_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ poly(x)·cosh(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    Result shape: ``C(x)·sinh(ax+b) + S(x)·cosh(ax+b)`` — same C and S
    polynomials as sinh, but the trig functions are swapped.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)
    C, S = _cs_coeffs(p, a)
    arg_ir = linear_to_ir(a, b, x_sym)
    sinh_ir = IRApply(SINH, (arg_ir,))
    cosh_ir = IRApply(COSH, (arg_ir,))
    # C·sinh + S·cosh
    return _assemble(sinh_ir, C, cosh_ir, S, x_sym)


def _cs_coeffs(poly: Polynomial, a: Fraction) -> tuple[Polynomial, Polynomial]:
    """Compute coefficient polynomials C and S for the tabular IBP formula.

    For index ``k`` of the derivative sequence:
    - ``sign = (−1)^k``  — gives the pattern +1, −1, +1, −1, …
    - ``scale = sign / a^(k+1)``
    - Even k contributes to C (coefficient of cosh for sinh-integral,
      coefficient of sinh for cosh-integral).
    - Odd k contributes to S.

    The hyperbolic sign alternation differs from ``trig_poly_integral._cs_coeffs``
    which uses ``(−1)^(k // 2)`` (period 4) because ``d²/dx² sinh = +sinh``
    (no sign flip), whereas ``d²/dx² sin = −sin`` (sign flip every 2 steps).

    Both sums terminate when a derivative reaches zero.
    """
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
        sign = Fraction((-1) ** k)
        scale = sign / a ** (k + 1)
        scaled = tuple(c * scale for c in dk)
        if k % 2 == 0:
            c_acc = add(c_acc, scaled)
        else:
            s_acc = add(s_acc, scaled)
    return normalize(c_acc), normalize(s_acc)


def _assemble(
    hyp1_ir: IRNode,
    poly1: Polynomial,
    hyp2_ir: IRNode,
    poly2: Polynomial,
    x_sym: IRSymbol,
) -> IRNode:
    """Build ``hyp1·poly1 + hyp2·poly2`` as IR, dropping zero terms."""
    has1 = bool(normalize(poly1))
    has2 = bool(normalize(poly2))

    if not has1 and not has2:
        return IRInteger(0)

    t1 = _poly_hyp(poly1, hyp1_ir, x_sym) if has1 else None
    t2 = _poly_hyp(poly2, hyp2_ir, x_sym) if has2 else None

    if t1 is None:
        return t2  # type: ignore[return-value]
    if t2 is None:
        return t1

    return IRApply(ADD, (t1, t2))


def _poly_hyp(coef: Polynomial, hyp_ir: IRNode, x_sym: IRSymbol) -> IRNode:
    """Build ``coef(x) · hyp_ir``, collapsing ±1 coefficients."""
    coef_ir = from_polynomial(coef, x_sym)
    if isinstance(coef_ir, IRInteger):
        if coef_ir.value == 1:
            return hyp_ir
        if coef_ir.value == -1:
            return IRApply(NEG, (hyp_ir,))
    return IRApply(MUL, (coef_ir, hyp_ir))


__all__ = ["cosh_poly_integral", "sinh_poly_integral"]
