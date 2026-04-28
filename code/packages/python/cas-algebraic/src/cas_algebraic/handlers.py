"""VM handler for AlgFactor — polynomial factoring over Q[√d].

This module provides a single handler that the symbolic VM calls when it
encounters an ``AlgFactor(polynomial, sqrt_expr)`` expression.

Surface syntax (MACSYMA)::

    algfactor(x^4 + 1, sqrt(2))
    → (x^2 + sqrt(2)*x + 1) * (x^2 - sqrt(2)*x + 1)

    algfactor(x^2 - 2, sqrt(2))
    → (x - sqrt(2)) * (x + sqrt(2))

    algfactor(x^4 - 5, sqrt(5))
    → (x^2 - sqrt(5)) * (x^2 + sqrt(5))

The handler is registered under the head name ``"AlgFactor"`` in
:func:`build_alg_factor_handler_table`.

IR representation
-----------------
``AlgFactor(poly_expr, Sqrt(d_expr))``

- First argument: the polynomial expression (any IR node).
- Second argument: ``Sqrt(IRInteger(d))`` — the algebraic element to adjoin.

Output IR for each factor
--------------------------
A factor with algebraic coefficients  a_k + b_k·√d  at degree k is built as
an ``Add``/``Mul`` IR tree.  For example the factor  x² + √2·x + 1  becomes::

    Add(Add(Pow(x, 2), Mul(Sqrt(2), x)), 1)

The full result for a product of two factors f1 * f2 is::

    Mul(f1_ir, f2_ir)

Graceful fall-through
---------------------
The handler returns the expression unevaluated (returns ``expr`` unchanged)
when:

- The wrong number of arguments is given.
- The second argument is not a ``Sqrt(n)`` expression.
- The first argument is not a polynomial in one variable over Q.
- No splitting over Q[√d] was found (polynomial is already irreducible
  or the splitting pattern does not apply).
"""

from __future__ import annotations

import math
from fractions import Fraction
from typing import TYPE_CHECKING

from symbolic_ir import (
    ADD,
    MUL,
    POW,
    SQRT,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_algebraic.algebraic import AlgPoly, factor_over_extension

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


# ---------------------------------------------------------------------------
# Helper: extract integer d from Sqrt(d) IR
# ---------------------------------------------------------------------------


def _extract_d(node: IRNode) -> int | None:
    """Return d if node is IRApply(SQRT, (IRInteger(d),)) else None.

    The second argument to AlgFactor must be exactly ``Sqrt(d)`` where d
    is a positive integer.  Any other shape returns None, causing the
    handler to fall through to unevaluated.

    Examples::

        Sqrt(2)  → 2
        Sqrt(5)  → 5
        Sqrt(x)  → None  (symbolic, not integer)
        2        → None  (not a Sqrt)
    """
    if not isinstance(node, IRApply):
        return None
    if node.head != SQRT:
        return None
    if len(node.args) != 1:
        return None
    inner = node.args[0]
    if not isinstance(inner, IRInteger):
        return None
    d = inner.value
    if d <= 0:
        return None
    # Verify d is square-free (the algebraic extension is well-defined only
    # when d is not a perfect square; if d is a perfect square then √d ∈ Q
    # already and the field Q[√d] = Q).
    sq = math.isqrt(d)
    if sq * sq == d:
        return None  # √d is rational — no extension needed.
    return d


# ---------------------------------------------------------------------------
# Helper: convert AlgPoly factor to IR
# ---------------------------------------------------------------------------


def _alg_coeff_to_ir(
    rational: Fraction, radical: Fraction, sqrt_d_ir: IRNode
) -> IRNode:
    """Build the IR for a single algebraic coefficient  rational + radical·√d.

    Three cases:

    1. radical == 0: pure rational → ``IRInteger`` or ``IRRational``.
    2. rational == 0: pure radical → ``Mul(radical_ir, sqrt_d_ir)``.
    3. Both non-zero: ``Add(rational_ir, Mul(radical_ir, sqrt_d_ir))``.

    ``sqrt_d_ir`` is the pre-built ``Sqrt(d)`` IR node, reused for all
    coefficients in this factor to keep the tree compact.
    """

    def _frac_ir(f: Fraction) -> IRNode:
        """Convert Fraction to IRInteger or IRRational."""
        if f.denominator == 1:
            return IRInteger(f.numerator)
        return IRRational(f.numerator, f.denominator)

    if radical == 0:
        return _frac_ir(rational)

    rad_part: IRNode
    if radical == 1:
        rad_part = sqrt_d_ir
    elif radical == -1:
        rad_part = IRApply(IRSymbol("Neg"), (sqrt_d_ir,))
    else:
        rad_part = IRApply(MUL, (_frac_ir(radical), sqrt_d_ir))

    if rational == 0:
        return rad_part

    return IRApply(ADD, (_frac_ir(rational), rad_part))


def _alg_poly_to_ir(
    factor: AlgPoly, x: IRSymbol, sqrt_d_ir: IRNode
) -> IRNode:
    """Build the IR tree for a polynomial with algebraic coefficients.

    The polynomial is given as a list of ``(rational_part, radical_part)``
    pairs in ascending degree.  For example  x² + √2·x + 1  is::

        [(Fraction(1), Fraction(0)),   # constant 1
         (Fraction(0), Fraction(1)),   # coefficient √2 for x^1
         (Fraction(1), Fraction(0))]   # coefficient 1 for x^2

    We build the tree bottom-up, summing the non-zero terms.

    Terms with zero coefficient are skipped to keep the output clean.
    """
    terms: list[IRNode] = []

    for k, (rat, rad) in enumerate(factor):
        if rat == 0 and rad == 0:
            continue  # Zero coefficient — skip

        coeff_ir = _alg_coeff_to_ir(rat, rad, sqrt_d_ir)

        if k == 0:
            terms.append(coeff_ir)
        elif k == 1:
            # coeff * x
            if rat == 1 and rad == 0:
                terms.append(x)
            else:
                terms.append(IRApply(MUL, (coeff_ir, x)))
        else:
            # coeff * x^k
            x_pow: IRNode = IRApply(POW, (x, IRInteger(k)))
            if rat == 1 and rad == 0:
                terms.append(x_pow)
            else:
                terms.append(IRApply(MUL, (coeff_ir, x_pow)))

    if not terms:
        return IRInteger(0)
    if len(terms) == 1:
        return terms[0]

    # Build a left-associative Add chain (matches the rest of the IR).
    acc: IRNode = terms[0]
    for t in terms[1:]:
        acc = IRApply(ADD, (acc, t))
    return acc


# ---------------------------------------------------------------------------
# Main handler
# ---------------------------------------------------------------------------


def alg_factor_handler(vm: VM, expr: IRApply) -> IRNode:
    """``AlgFactor(poly, Sqrt(d))`` — factor poly over Q[√d].

    Evaluates to a product of algebraic factors when a splitting exists,
    otherwise returns the expression unevaluated.

    Parameters
    ----------
    vm:
        The live VM instance.  We call ``vm.eval`` before delegating to the
        polynomial bridge so that e.g. ``AlgFactor(x^2*x^2 + 1, Sqrt(2))``
        is normalised first.
    expr:
        Must have exactly 2 arguments: the polynomial and ``Sqrt(d)``.

    Returns
    -------
    On success, a ``Mul(f1, f2, ...)`` IR tree where each ``f_k`` is a
    polynomial in x with coefficients from Q[√d].

    On failure (wrong arity, non-polynomial input, irreducible, unsupported
    extension), returns ``expr`` unchanged.

    Examples::

        # x^4 + 1 over Q[√2] → (x^2+√2x+1)(x^2-√2x+1)
        AlgFactor(Add(Pow(x,4),1), Sqrt(2))
        → Mul(Add(Add(Pow(x,2), Mul(Sqrt(2),x)), 1),
              Add(Add(Pow(x,2), Neg(Mul(Sqrt(2),x))), 1))
    """
    if len(expr.args) != 2:
        return expr

    poly_ir, sqrt_ir = expr.args

    # Extract d from Sqrt(d).
    d = _extract_d(sqrt_ir)
    if d is None:
        return expr

    # Find the polynomial variable by searching for free symbols.
    x = _find_variable(poly_ir)
    if x is None:
        return expr

    # Convert IR to rational polynomial.
    from symbolic_vm.polynomial_bridge import to_rational

    rational = to_rational(poly_ir, x)
    if rational is None:
        return expr

    num_frac, den_frac = rational
    _ONE_FRAC: tuple[Fraction, ...] = (Fraction(1),)
    if den_frac != _ONE_FRAC:
        return expr  # Rational function — not a polynomial.

    # Convert Fraction coefficients → integers (clear denominators).
    denoms = [c.denominator for c in num_frac]
    import math as _math

    lcm = denoms[0]
    for dn in denoms[1:]:
        lcm = lcm * dn // _math.gcd(lcm, dn)
    int_coeffs = [int(c * lcm) for c in num_frac]

    # Attempt factoring over Q[√d].
    factors = factor_over_extension(int_coeffs, d)
    if factors is None:
        return expr  # Irreducible over Q[√d].

    # Pre-build Sqrt(d) IR node for use in all factor IRs.
    sqrt_d_ir: IRNode = IRApply(SQRT, (IRInteger(d),))

    # Convert each AlgPoly factor to an IR tree.
    factor_irs = [_alg_poly_to_ir(f, x, sqrt_d_ir) for f in factors]

    if not factor_irs:
        return expr

    if len(factor_irs) == 1:
        return factor_irs[0]

    # Multiply all factors together (left-associative Mul chain).
    acc_ir: IRNode = factor_irs[0]
    for f_ir in factor_irs[1:]:
        acc_ir = IRApply(MUL, (acc_ir, f_ir))
    return acc_ir


# ---------------------------------------------------------------------------
# _find_variable helper (duplicated from cas_handlers to avoid circular deps)
# ---------------------------------------------------------------------------

_CONSTANT_NAMES = frozenset({"True", "False", "%pi", "%e", "%i"})


def _find_variable(node: IRNode) -> IRSymbol | None:
    """Return the first free ``IRSymbol`` in ``node``, depth-first.

    Skips pre-bound constant names.  Returns ``None`` for numeric-only
    expressions.
    """
    if isinstance(node, IRSymbol):
        if node.name not in _CONSTANT_NAMES:
            return node
        return None
    if isinstance(node, IRApply):
        for arg in node.args:
            found = _find_variable(arg)
            if found is not None:
                return found
    return None


# ---------------------------------------------------------------------------
# Public builder
# ---------------------------------------------------------------------------


def build_alg_factor_handler_table() -> dict[str, object]:
    """Return the handler table entry for AlgFactor.

    This is merged into ``build_cas_handler_table()`` in ``symbolic-vm``
    so that ``AlgFactor(poly, Sqrt(d))`` is dispatched to
    :func:`alg_factor_handler` automatically.

    Returns
    -------
    ``{"AlgFactor": alg_factor_handler}``
    """
    return {"AlgFactor": alg_factor_handler}
