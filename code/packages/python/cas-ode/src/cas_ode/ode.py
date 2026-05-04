"""Symbolic ODE solver — the heart of cas-ode.

This module provides pure-Python functions that recognise and solve seven
classes of ordinary differential equations whose solutions can always be
written in closed form:

1. **First-order linear**: ``dy/dx + P(x)·y = Q(x)``
2. **Separable**: ``dy/dx = f(x)·g(y)`` (including the degenerate cases
   ``g(y) = 1`` and ``f(x) = 1``)
3. **Bernoulli**: ``dy/dx + P(x)·y = Q(x)·y^n``  (n ≠ 0, 1;
   reduces to linear via substitution ``v = y^(1-n)``)
4. **Exact**: ``M(x,y)·dx + N(x,y)·dy = 0`` when ``∂M/∂y = ∂N/∂x``
   (produces an implicit solution ``F(x,y) = C``)
5. **Second-order constant-coefficient homogeneous**:
   ``a·y'' + b·y' + c·y = 0``
6. **Second-order constant-coefficient non-homogeneous**:
   ``a·y'' + b·y' + c·y = f(x)`` where f is a constant, polynomial
   (degree ≤ 2), exponential, sin/cos, or e^(αx)·sin/cos(βx)

Every public function takes IR nodes as input and returns IR nodes as
output.  No floats are used for exact computation; rational arithmetic
uses Python's :class:`fractions.Fraction`.

Integration constants
---------------------
- First-order ODEs use ``%c``  — ``IRSymbol("%c")``.
- Second-order ODEs use ``%c1`` and ``%c2`` — ``IRSymbol("%c1")``
  and ``IRSymbol("%c2")``.

These match MACSYMA's naming convention.  The constants are treated as
free symbols; the VM leaves them unevaluated.

Architecture note
-----------------
Functions in this module are *pure*: they take an IR tree and return an
IR tree.  They do not call the VM; instead, the handler in
``cas_ode.handlers`` passes the VM handle in for the rare cases (linear
ODEs) where we need ``Integrate``.

Literate reading guide
----------------------
Read the functions in this order:

1.  :func:`_is_const_wrt`  — "does this IR subtree depend on ``x``?"
2.  :func:`_collect_second_order_coeffs` — pattern-match ``a·y''+b·y'+c·y``
3.  :func:`solve_second_order_const_coeff` — characteristic equation → roots
4.  :func:`_collect_linear_first_order` — pattern-match ``y' + P(x)·y``
5.  :func:`solve_linear_first_order` — integrating-factor method
6.  :func:`_try_separable` — separation of variables
7.  :func:`_is_pow_y` — helper: detect ``Pow(y, n)`` atoms
8.  :func:`_try_bernoulli` — Bernoulli substitution ``v = y^(1-n)``
9.  :func:`_try_exact` — exact ODE potential-function method
10. :func:`_collect_second_order_nonhom` — collect (a,b,c,f) from non-hom ODE
11. :func:`_classify_forcing` — identify the forcing-function family
12. :func:`_compute_particular` — undetermined-coefficient ansatz
13. :func:`_try_second_order_nonhom` — non-homogeneous 2nd-order dispatcher
14. :func:`solve_ode` — the top-level dispatcher
"""

from __future__ import annotations

import math
from fractions import Fraction
from typing import TYPE_CHECKING

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EQUAL,
    EXP,
    INTEGRATE,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_ir.nodes import C1, C2, C_CONST, ODE2, D

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Convenience builders — keep the body code readable.
# ---------------------------------------------------------------------------

_ZERO = IRInteger(0)
_ONE = IRInteger(1)
_NEG_ONE = IRInteger(-1)
_TWO = IRInteger(2)


def _add(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Add(a, b)`` — avoid wrapping when one side is ``0``."""
    if isinstance(a, IRInteger) and a.value == 0:
        return b
    if isinstance(b, IRInteger) and b.value == 0:
        return a
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Sub(a, b)``."""
    return IRApply(SUB, (a, b))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Mul(a, b)`` — drop trivial factors of ±1."""
    if isinstance(a, IRInteger) and a.value == 1:
        return b
    if isinstance(b, IRInteger) and b.value == 1:
        return a
    if isinstance(a, IRInteger) and a.value == -1:
        return IRApply(NEG, (b,))
    if isinstance(b, IRInteger) and b.value == -1:
        return IRApply(NEG, (a,))
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Div(a, b)``."""
    return IRApply(DIV, (a, b))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    """Build ``Pow(base, exp)``."""
    return IRApply(POW, (base, exp))


def _exp(arg: IRNode) -> IRNode:
    """Build ``Exp(arg)``."""
    return IRApply(EXP, (arg,))


def _sin(arg: IRNode) -> IRNode:
    """Build ``Sin(arg)``."""
    return IRApply(SIN, (arg,))


def _cos(arg: IRNode) -> IRNode:
    """Build ``Cos(arg)``."""
    return IRApply(COS, (arg,))


def _neg(arg: IRNode) -> IRNode:
    """Build ``Neg(arg)``."""
    return IRApply(NEG, (arg,))


def _frac_to_ir(f: Fraction) -> IRNode:
    """Lift a ``Fraction`` to the canonical IR literal.

    ``Fraction(2, 1)`` → ``IRInteger(2)``.
    ``Fraction(1, 2)`` → ``IRRational(1, 2)``.
    """
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


# ---------------------------------------------------------------------------
# Section 1 — Helper: does a subtree contain the variable ``x``?
# ---------------------------------------------------------------------------


def _is_const_wrt(node: IRNode, var: IRSymbol) -> bool:
    """Return ``True`` if ``node`` does not contain ``var``.

    This is the "is constant with respect to ``x``?" test used to split
    an expression into a ``var``-dependent part and a constant part.

    Examples::

        _is_const_wrt(IRInteger(3), x)    → True
        _is_const_wrt(IRSymbol("x"), x)   → False
        _is_const_wrt(IRSymbol("y"), x)   → True
        _is_const_wrt(Add(x, 1), x)       → False
        _is_const_wrt(Add(y, 1), x)       → True  (y ≠ x)
    """
    if isinstance(node, IRSymbol):
        return node != var
    if isinstance(node, (IRInteger, IRRational)):
        return True
    if isinstance(node, IRApply):
        # The head of an IRApply is always an operator symbol (Add, Sin, …),
        # not a free variable — skip it and check only the argument list.
        return all(_is_const_wrt(arg, var) for arg in node.args)
    return True  # IRFloat, IRString — treat as constant


# ---------------------------------------------------------------------------
# Section 2 — Second-order constant-coefficient recogniser
# ---------------------------------------------------------------------------


def _collect_second_order_coeffs(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
) -> tuple[Fraction, Fraction, Fraction] | None:
    """Try to read ``a``, ``b``, ``c`` from ``a·y'' + b·y' + c·y = 0``.

    The expression ``expr`` represents the left-hand side (which equals
    zero).  We look for terms of the form:

    - ``D(D(y, x), x)`` — second derivative, coefficient ``a``
    - ``D(y, x)`` — first derivative, coefficient ``b``
    - ``y`` — dependent variable itself, coefficient ``c``

    All three coefficients must be rational numbers (``IRInteger`` or
    ``IRRational``); if any term has a non-constant coefficient, we
    return ``None`` and the caller falls through to the unevaluated form.

    The function handles expressions structured as a top-level ``Add``
    with two sub-terms (since the compiler always generates left-
    associative binary trees), nested ``Add`` chains, or a single atom.

    Parameters
    ----------
    expr:
        The ODE expression that equals zero.
    y:
        The dependent variable (the function being solved for).
    x:
        The independent variable.

    Returns
    -------
    (a, b, c) as Fractions if the pattern matches, or None otherwise.
    """
    # Build canonical IR nodes for y'' and y' so we can match them.
    y_prime = IRApply(D, (y, x))         # D(y, x)
    y_double = IRApply(D, (y_prime, x))  # D(D(y, x), x)

    # Flatten the expression into a sum of terms.
    terms = _flatten_add(expr)

    a = Fraction(0)
    b = Fraction(0)
    c = Fraction(0)
    matched = 0  # how many distinct ODE terms we've matched

    for term in terms:
        coeff, base = _extract_coeff(term, x)
        if coeff is None:
            return None  # Non-constant coefficient — not a const-coeff ODE
        if base == y_double:
            a += coeff
            matched += 1
        elif base == y_prime:
            b += coeff
            matched += 1
        elif base == y:
            c += coeff
            matched += 1
        else:
            return None  # Unknown term (e.g. sin(x)*y, forcing term)

    # A second-order ODE must mention y'' at minimum.
    if a == 0 or matched < 2:
        return None

    return (a, b, c)


def _flatten_add(expr: IRNode) -> list[IRNode]:
    """Recursively flatten an ``Add`` tree into a list of summands.

    ``Add(Add(a, b), c)`` → ``[a, b, c]``.
    ``Sub(a, b)``         → ``[a, Neg(b)]``.
    Any other node        → ``[node]``.

    Double negation is simplified: ``Neg(Neg(x))`` → ``x``.

    This handles the left-associative binary representation the MACSYMA
    compiler generates for sums of three or more terms.
    """
    if isinstance(expr, IRApply) and isinstance(expr.head, IRSymbol):
        if expr.head == ADD and len(expr.args) == 2:
            return _flatten_add(expr.args[0]) + _flatten_add(expr.args[1])
        if expr.head == SUB and len(expr.args) == 2:
            return _flatten_add(expr.args[0]) + [_neg(expr.args[1])]
        if expr.head == NEG and len(expr.args) == 1:
            inner_node = expr.args[0]
            # Simplify double negation: Neg(Neg(x)) → x
            if (
                isinstance(inner_node, IRApply)
                and inner_node.head == NEG
                and len(inner_node.args) == 1
            ):
                return _flatten_add(inner_node.args[0])
            # Neg(Add(a, b)) → [-a, -b]
            inner = _flatten_add(inner_node)
            return [_neg(t) for t in inner]
    return [expr]


def _extract_coeff(
    term: IRNode, x: IRSymbol
) -> tuple[Fraction, IRNode] | tuple[None, None]:
    """Split a term into ``(rational_coefficient, base_expression)``.

    Handles (with recursive unwrapping):
    - ``Mul(coeff, base)`` where coeff is an ``IRInteger`` / ``IRRational``
    - ``Neg(base)``  → coefficient is ``-1``
    - ``Neg(Mul(coeff, base))`` → coefficient is ``-coeff``
    - bare ``base``  → coefficient is ``1``

    Returns ``(None, None)`` if the coefficient is not a rational constant
    (e.g. ``x * y``).

    Examples::

        _extract_coeff(Mul(2, D(y, x)), x)        → (Fraction(2), D(y, x))
        _extract_coeff(Neg(y), x)                  → (Fraction(-1), y)
        _extract_coeff(Neg(Mul(2, D(y,x))), x)    → (Fraction(-2), D(y, x))
        _extract_coeff(y, x)                       → (Fraction(1), y)
        _extract_coeff(Mul(x, y), x)               → (None, None)
    """
    if isinstance(term, IRApply) and term.head == MUL and len(term.args) == 2:
        left, right = term.args
        # Try left as the coefficient.
        if isinstance(left, IRInteger) and _is_const_wrt(left, x):
            return (Fraction(left.value), right)
        if isinstance(left, IRRational):
            return (Fraction(left.numer, left.denom), right)
        # Try right as the coefficient.
        if isinstance(right, IRInteger) and _is_const_wrt(right, x):
            return (Fraction(right.value), left)
        if isinstance(right, IRRational):
            return (Fraction(right.numer, right.denom), left)
        # Both sides are symbolic — coefficient is not a plain rational.
        return (None, None)
    if isinstance(term, IRApply) and term.head == NEG and len(term.args) == 1:
        inner = term.args[0]
        # Recursively extract coefficient from the negated sub-expression.
        # e.g. Neg(Mul(2, D(y,x))) → coeff=-2, base=D(y,x)
        inner_coeff, inner_base = _extract_coeff(inner, x)
        if inner_coeff is not None:
            return (Fraction(-1) * inner_coeff, inner_base)
        return (Fraction(-1), inner)
    if isinstance(term, IRInteger):
        return (Fraction(term.value), _ONE)
    if isinstance(term, IRRational):
        return (Fraction(term.numer, term.denom), _ONE)
    # Bare node with implicit coefficient 1.
    return (Fraction(1), term)


# ---------------------------------------------------------------------------
# Section 3 — Second-order solver
# ---------------------------------------------------------------------------


def solve_second_order_const_coeff(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    y: IRSymbol,
    x: IRSymbol,
) -> IRNode:
    """Solve ``a·y'' + b·y' + c·y = 0`` by the characteristic equation.

    The characteristic equation is::

        a·r² + b·r + c = 0

    whose discriminant is ``Δ = b² − 4ac``.  Three cases arise:

    1. **Two distinct real roots** (``Δ > 0``):
       ``y = C1·exp(r1·x) + C2·exp(r2·x)``

    2. **Repeated root** (``Δ = 0``):
       ``y = (C1 + C2·x)·exp(r·x)``

    3. **Complex conjugate roots** (``Δ < 0``):
       ``α ± βi`` where ``α = -b/(2a)`` and ``β = √(-Δ)/(2a)``
       ``y = exp(αx)·(C1·cos(βx) + C2·sin(βx))``

    All arithmetic on the roots is done in Python using
    :class:`fractions.Fraction` for exact rational computations. Complex
    roots are only valid when ``α`` and ``β`` are rational; if the
    discriminant is a non-perfect-square, we produce an IR tree with
    ``Pow(-discriminant_numerics, Rational(1,2))`` (i.e. ``sqrt``) nodes.

    Parameters
    ----------
    a, b, c:
        Coefficients of the characteristic polynomial (Fractions).
    y:
        The dependent variable symbol.
    x:
        The independent variable symbol.

    Returns
    -------
    ``Equal(y, solution)`` IR node.
    """
    # Normalise: divide through by ``a`` so the characteristic polynomial
    # is monic.  This keeps subsequent fraction arithmetic simpler.
    # Characteristic equation: r^2 + (b/a)*r + (c/a) = 0
    # Discriminant: Δ = (b/a)^2 - 4*(c/a) = (b^2 - 4*a*c) / a^2
    #
    # We work with the un-normalised discriminant ``disc_numer = b^2 - 4ac``
    # (the sign of Δ equals the sign of ``disc_numer`` since a^2 > 0).
    disc_numer = b * b - Fraction(4) * a * c

    def _ir_from_frac(f: Fraction) -> IRNode:
        """Return the IR literal for a possibly-negative Fraction."""
        if f >= 0:
            return _frac_to_ir(f)
        return _neg(_frac_to_ir(-f))

    if disc_numer > 0:
        # ---- Case 1: two distinct real roots --------------------------------
        # r1, r2 = (-b ± sqrt(b^2 - 4ac)) / (2a)
        #
        # We try to keep the roots exact (rational) by checking whether
        # disc_numer * a^2 is a perfect square in the numerator.
        # The denominator is always (2a)^2, so we only need to check the
        # numerator disc_numer * (a.denom)^2.
        #
        # disc_numer is already b^2 - 4ac (over the integers of Fraction).
        # The actual discriminant as a Fraction is disc_numer (since we kept
        # a,b,c as Fractions we need to convert):
        disc_frac = disc_numer  # already Fraction
        sqrt_disc_exact = _exact_sqrt_fraction(disc_frac)

        if sqrt_disc_exact is not None:
            # Exact rational roots.
            r1 = (-b + sqrt_disc_exact) / (2 * a)
            r2 = (-b - sqrt_disc_exact) / (2 * a)
            term1 = _mul(C1, _exp(_mul(_ir_from_frac(r1), x)))
            term2 = _mul(C2, _exp(_mul(_ir_from_frac(r2), x)))
        else:
            # Irrational discriminant — represent sqrt symbolically.
            sqrt_disc_ir = _pow(_frac_to_ir(disc_frac), IRRational(1, 2))
            denom_ir = _ir_from_frac(2 * a)
            r1_ir = _div(_add(_ir_from_frac(-b), sqrt_disc_ir), denom_ir)
            r2_ir = _div(_sub(_ir_from_frac(-b), sqrt_disc_ir), denom_ir)
            term1 = _mul(C1, _exp(_mul(r1_ir, x)))
            term2 = _mul(C2, _exp(_mul(r2_ir, x)))

        solution = _add(term1, term2)

    elif disc_numer == 0:
        # ---- Case 2: repeated root ------------------------------------------
        # r = -b / (2a)
        r = (-b) / (2 * a)
        r_ir = _ir_from_frac(r)
        # y = (C1 + C2*x) * exp(r*x)
        inside = _add(C1, _mul(C2, x))
        exp_part = _exp(_mul(r_ir, x))
        solution = _mul(inside, exp_part)

    else:
        # ---- Case 3: complex conjugate roots --------------------------------
        # α = -b/(2a),  β = sqrt(|disc_numer|) / (2a)  [both rational or not]
        alpha = (-b) / (2 * a)
        abs_disc = -disc_numer  # positive
        beta_sq = abs_disc / (4 * a * a)
        sqrt_beta_sq = _exact_sqrt_fraction(beta_sq)

        alpha_ir = _ir_from_frac(alpha)

        if sqrt_beta_sq is not None:
            beta_ir = _ir_from_frac(sqrt_beta_sq)
        else:
            beta_ir = _pow(_frac_to_ir(beta_sq), IRRational(1, 2))

        # y = exp(α·x) · (C1·cos(β·x) + C2·sin(β·x))
        cos_term = _mul(C1, _cos(_mul(beta_ir, x)))
        sin_term = _mul(C2, _sin(_mul(beta_ir, x)))
        trig_sum = _add(cos_term, sin_term)
        exp_part = _exp(_mul(alpha_ir, x))
        solution = _mul(exp_part, trig_sum)

    return IRApply(EQUAL, (y, solution))


def _exact_sqrt_fraction(f: Fraction) -> Fraction | None:
    """Return the exact rational square root of ``f``, or ``None``.

    Works for positive Fractions whose numerator and denominator are both
    perfect squares.

    Examples::

        _exact_sqrt_fraction(Fraction(4))     → Fraction(2)
        _exact_sqrt_fraction(Fraction(1, 4))  → Fraction(1, 2)
        _exact_sqrt_fraction(Fraction(2))     → None  (irrational)
        _exact_sqrt_fraction(Fraction(0))     → Fraction(0)
    """
    if f < 0:
        return None
    if f == 0:
        return Fraction(0)
    p = f.numerator
    q = f.denominator
    sp = _isqrt_exact(p)
    sq = _isqrt_exact(q)
    if sp is None or sq is None:
        return None
    return Fraction(sp, sq)


def _isqrt_exact(n: int) -> int | None:
    """Return the integer square root of ``n`` if ``n`` is a perfect square.

    Uses :func:`math.isqrt` (Python 3.8+) and verifies by squaring.

    Examples::

        _isqrt_exact(9)  → 3
        _isqrt_exact(2)  → None
    """
    if n < 0:
        return None
    r = math.isqrt(n)
    return r if r * r == n else None


# ---------------------------------------------------------------------------
# Section 4 — First-order linear ODE recogniser
# ---------------------------------------------------------------------------


def _is_y_prime(node: IRNode, y: IRSymbol, x: IRSymbol) -> bool:
    """Return True if ``node`` represents ``D(y, x)`` (= dy/dx)."""
    return (
        isinstance(node, IRApply)
        and node.head == D
        and len(node.args) == 2
        and node.args[0] == y
        and node.args[1] == x
    )


def _collect_linear_first_order(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
) -> tuple[IRNode, IRNode] | None:
    """Try to read ``P(x)`` and ``Q(x)`` from ``y' + P(x)·y - Q(x) = 0``.

    We look for exactly three types of terms in the flattened sum:

    - A term equal to ``D(y, x)`` (possibly with coefficient 1 — but
      coefficients other than ±1 are valid too).
    - Terms whose "y-free" factor is just ``y`` — these contribute to P(x).
    - Terms that don't involve ``y`` at all — these contribute to −Q(x).

    Returns ``(P_ir, Q_ir)`` where both are IR expressions in ``x`` only.
    Returns ``None`` if the ODE is not in the standard linear form.

    Standard form::

        dy/dx + P(x)·y = Q(x)
        ≡  dy/dx + P(x)·y − Q(x) = 0   (everything moved to LHS)

    P and Q are the integrating-factor components passed to
    :func:`solve_linear_first_order`.
    """
    terms = _flatten_add(expr)
    yprime_coeff = Fraction(0)
    p_terms: list[tuple[IRNode, bool]] = []  # (x_factor, negated)
    q_terms: list[tuple[IRNode, bool]] = []  # (x_factor, negated)

    for term in terms:
        neg, core = _unwrap_neg(term)

        # ----- y' term -------------------------------------------------------
        if _is_y_prime(core, y, x):
            yprime_coeff += Fraction(-1 if neg else 1)
            continue

        # ----- coefficient * y' ----------------------------------------------
        mul_result = _split_coeff_base(core)
        if mul_result is not None:
            coeff_node, base = mul_result
            if _is_y_prime(base, y, x) and _is_const_wrt(coeff_node, x):
                f, _ = _extract_coeff(coeff_node, x)
                if f is not None:
                    yprime_coeff += Fraction(-1 if neg else 1) * f
                    continue

        # ----- coeff * y term ------------------------------------------------
        if mul_result is not None:
            coeff_node, base = mul_result
            if base == y and _is_const_wrt(coeff_node, x):
                p_terms.append((coeff_node, neg))
                continue
            if base == y:
                p_terms.append((coeff_node, neg))
                continue

        if core == y:
            p_terms.append((_ONE, neg))
            continue

        # ----- constant (w.r.t. y) term: goes into Q(x) ---------------------
        if _is_const_wrt(core, y):
            q_terms.append((core, not neg))  # Q appears as -Q(x) on LHS
            continue

        # Some other structure we don't recognise.
        return None

    # y' must appear with coefficient 1 (after normalisation).
    if yprime_coeff == 0:
        return None

    # Build P(x) from collected terms, dividing by y' coefficient.
    p_ir = _sum_of_terms(p_terms)
    q_ir = _sum_of_terms(q_terms)

    if yprime_coeff != 1:
        # Divide through — y' + P/yprime * y = Q/yprime
        p_ir = _div(p_ir, _frac_to_ir(yprime_coeff))
        q_ir = _div(q_ir, _frac_to_ir(yprime_coeff))

    return (p_ir, q_ir)


def _unwrap_neg(node: IRNode) -> tuple[bool, IRNode]:
    """Return ``(is_negated, inner)`` for a possibly-``Neg``-wrapped node.

    Examples::

        _unwrap_neg(Neg(x))  → (True, x)
        _unwrap_neg(x)       → (False, x)
    """
    if isinstance(node, IRApply) and node.head == NEG and len(node.args) == 1:
        return (True, node.args[0])
    return (False, node)


def _split_coeff_base(node: IRNode) -> tuple[IRNode, IRNode] | None:
    """If ``node`` is ``Mul(a, b)`` return ``(a, b)``, else ``None``."""
    if isinstance(node, IRApply) and node.head == MUL and len(node.args) == 2:
        return (node.args[0], node.args[1])
    return None


def _sum_of_terms(terms: list[tuple[IRNode, bool]]) -> IRNode:
    """Accumulate a list of ``(expr, negated)`` pairs into a sum.

    Returns ``IRInteger(0)`` for an empty list.
    """
    if not terms:
        return _ZERO
    result: IRNode = _ZERO
    for expr, neg in terms:
        term_ir = _neg(expr) if neg else expr
        result = _add(result, term_ir)
    return result


# ---------------------------------------------------------------------------
# Section 5 — First-order linear solver (integrating factor)
# ---------------------------------------------------------------------------


def solve_linear_first_order(
    p_ir: IRNode,
    q_ir: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode:
    """Solve ``y' + P(x)·y = Q(x)`` by the integrating-factor method.

    Algorithm
    ---------
    The integrating factor is::

        μ(x) = exp(∫ P(x) dx)

    Multiplying both sides by μ turns the left side into the derivative
    of μ·y::

        d/dx (μ·y) = μ·Q(x)

    Integrating both sides::

        μ·y = ∫ μ·Q(x) dx + C

    Solving for y::

        y = (1/μ) · (∫ μ·Q(x) dx + C)

    The integration constants from ∫P dx and ∫μQ dx are suppressed (set
    to zero) because they get absorbed into the free constant C (= ``%c``).

    Parameters
    ----------
    p_ir:
        The coefficient function ``P(x)`` in the standard form.
    q_ir:
        The right-hand side ``Q(x)`` in the standard form.
    y, x:
        The dependent and independent variable symbols.
    vm:
        A live VM instance, used only to evaluate ``Integrate(f, x)``.

    Returns
    -------
    ``Equal(y, solution)`` IR node, or the unevaluated ODE if integration
    fails (i.e. if the VM returns unevaluated ``Integrate(…)``).
    """

    def _integrate(f: IRNode) -> IRNode:
        """Call the VM's Integrate handler to compute ∫ f dx."""
        return vm.eval(IRApply(INTEGRATE, (f, x)))

    # Step 1: Compute ∫ P dx (no constant of integration).
    int_p = _integrate(p_ir)
    if _is_unevaluated_integrate(int_p, x):
        return _unevaluated_ode(y, x, p_ir, q_ir, vm)

    # Step 2: μ = exp(∫ P dx)
    mu = _exp(int_p)

    # Step 3: Compute ∫ μ·Q dx.
    mu_q = vm.eval(_mul(mu, q_ir))
    int_mu_q = _integrate(mu_q)
    if _is_unevaluated_integrate(int_mu_q, x):
        return _unevaluated_ode(y, x, p_ir, q_ir, vm)

    # Step 4: y = (1/μ) · (∫ μQ dx + C_CONST)
    numerator = _add(int_mu_q, C_CONST)
    solution = vm.eval(_mul(_div(_ONE, mu), numerator))

    return IRApply(EQUAL, (y, solution))


def _is_unevaluated_integrate(node: IRNode, x: IRSymbol) -> bool:
    """Return True if ``node`` is still an ``Integrate(f, x)`` application.

    This happens when the VM cannot compute the integral symbolically.
    We use it to detect "integration failed" so we can fall through to
    the unevaluated ODE form.
    """
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head == INTEGRATE
        and len(node.args) == 2
        and node.args[1] == x
    )


def _unevaluated_ode(y: IRSymbol, x: IRSymbol, *args: IRNode, **_: object) -> IRNode:
    """Return the unevaluated sentinel (not a full ODE2 node — just a flag).

    When the solver gives up, the caller (handlers.py) returns the
    original ``ODE2(expr, y, x)`` node unchanged.  This helper is used
    internally to signal failure; callers check the returned type.
    """
    return IRApply(ODE2, (IRInteger(-999), y, x))  # sentinel


# ---------------------------------------------------------------------------
# Section 6 — Separable ODE recogniser and solver
# ---------------------------------------------------------------------------


def _try_separable(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Try to solve ``expr = 0`` as a separable ODE ``y' = f(x)·g(y)``.

    A first-order separable ODE has the form::

        dy/dx = f(x) · g(y)

    which rearranges to::

        dy/g(y) = f(x) dx

    Integrating both sides::

        ∫ dy/g(y) = ∫ f(x) dx + %c

    We solve for ``y`` when both integrals can be computed.

    Pattern recognition
    -------------------
    We rearrange the ODE to the form ``y' - rhs = 0`` and examine ``rhs``:

    - If ``rhs`` is purely in ``x`` (independent of ``y``): P=0 case of
      linear ODE, handled as ``y' = f(x)``.
    - If ``rhs`` is purely in ``y`` (no ``x`` dependence): separable with
      ``f(x) = 1``, ``g(y) = rhs``.
    - If ``rhs = Mul(f_x, g_y)`` where ``f_x`` is ``x``-only and ``g_y``
      is ``y``-only: fully separable.

    Parameters
    ----------
    expr:
        The ODE as a zero expression (``lhs - rhs = 0`` form).
    y, x:
        Dependent and independent variable.
    vm:
        Live VM handle for symbolic integration.

    Returns
    -------
    ``Equal(y, solution)`` if successful, or ``None`` to signal fall-through.
    """
    terms = _flatten_add(expr)
    y_prime = IRApply(D, (y, x))

    # Find y' term in the sum; everything else is the negative of rhs.
    yprime_found = False
    other_terms: list[IRNode] = []

    for term in terms:
        neg, core = _unwrap_neg(term)
        if core == y_prime and not neg:
            yprime_found = True
        elif core == y_prime and neg:
            # -y' on the LHS would mean a sign issue — not standard form.
            return None
        else:
            other_terms.append(_neg(term))  # move to rhs: rhs = -term

    if not yprime_found:
        return None

    # rhs = sum of other_terms (with signs flipped to move across the =0).
    if not other_terms:
        rhs: IRNode = _ZERO
    elif len(other_terms) == 1:
        rhs = other_terms[0]
    else:
        rhs = other_terms[0]
        for t in other_terms[1:]:
            rhs = _add(rhs, t)

    rhs = vm.eval(rhs)

    # ---- Classify rhs -------------------------------------------------------

    def _integrate(f: IRNode) -> IRNode:
        return vm.eval(IRApply(INTEGRATE, (f, x)))

    def _integrate_y(f: IRNode) -> IRNode:
        return vm.eval(IRApply(INTEGRATE, (f, y)))

    if _is_const_wrt(rhs, y):
        # Case 1: y' = f(x)  — integrate directly.
        int_f = _integrate(rhs)
        if _is_unevaluated_integrate(int_f, x):
            return None
        solution = _add(int_f, C_CONST)
        return IRApply(EQUAL, (y, vm.eval(solution)))

    if _is_const_wrt(rhs, x):
        # Case 2: y' = g(y)  →  dy/g(y) = dx  →  ∫ dy/g(y) = x + %c
        # We can't always invert, so instead we use the integrating-factor
        # route: write as y' - g(y) = 0, i.e. y' + P*y = Q where P=0 and
        # g(y) is the entire nonlinear piece.
        # For the pure g(y) = k*y case (linear in y), delegate to linear solver.
        g_factor, g_base = _extract_coeff(rhs, x)
        if g_base == y and g_factor is not None:
            # y' = k*y  →  y' - k*y = 0  →  P = -k, Q = 0
            return solve_linear_first_order(
                _frac_to_ir(-g_factor), _ZERO, y, x, vm
            )
        return None  # Nonlinear g(y) — not handled

    # Case 3: rhs = Mul(f_x, g_y)  or some combination.
    if isinstance(rhs, IRApply) and rhs.head == MUL and len(rhs.args) == 2:
        left, right = rhs.args
        f_x: IRNode | None = None
        g_y: IRNode | None = None
        if _is_const_wrt(left, y) and _is_const_wrt(right, x):
            f_x, g_y = left, right
        elif _is_const_wrt(right, y) and _is_const_wrt(left, x):
            f_x, g_y = right, left

        if f_x is not None and g_y is not None:
            # dy/g(y) = f(x) dx
            # Check for linear g(y) = k*y first (delegate to integrating factor).
            g_factor, g_base = _extract_coeff(g_y, y)
            if g_base == y and g_factor is not None:
                # y' = f(x)*k*y  →  y' - k*f(x)*y = 0  →  P = -k*f(x), Q = 0
                p_new = vm.eval(_mul(_frac_to_ir(-g_factor), f_x))
                return solve_linear_first_order(p_new, _ZERO, y, x, vm)

    # Generic rhs: try linear first-order decomposition.
    # If rhs = P_coeff * y + Q_expr we have linear form.
    terms_rhs = _flatten_add(rhs)
    y_terms_coeffs: list[IRNode] = []
    x_only_terms: list[IRNode] = []
    for t in terms_rhs:
        neg2, core2 = _unwrap_neg(t)
        mul2 = _split_coeff_base(core2)
        if core2 == y:
            y_terms_coeffs.append(_neg(_ONE) if neg2 else _ONE)
        elif mul2 is not None and mul2[1] == y and _is_const_wrt(mul2[0], y):
            coeff_node = _neg(mul2[0]) if neg2 else mul2[0]
            y_terms_coeffs.append(coeff_node)
        elif _is_const_wrt(core2, y):
            x_only_terms.append(_neg(core2) if neg2 else core2)
        else:
            return None  # Mixed term we can't handle

    if y_terms_coeffs:
        p_neg = _sum_of_terms([(c, False) for c in y_terms_coeffs])
        p_ir = vm.eval(_neg(p_neg))  # P = -coefficient of y in rhs
        q_ir = _sum_of_terms([(t, False) for t in x_only_terms])
        return solve_linear_first_order(p_ir, q_ir, y, x, vm)

    return None


# ---------------------------------------------------------------------------
# Section 8 — Bernoulli ODE recogniser and solver
# ---------------------------------------------------------------------------


def _is_pow_y(node: IRNode, y: IRSymbol) -> int | None:
    """Return exponent ``n`` if ``node`` is ``Pow(y, n)`` with ``n`` an
    integer that is neither 0 nor 1; otherwise return ``None``.

    We use this in the Bernoulli recogniser to locate the ``y^n`` term.

    Examples::

        _is_pow_y(Pow(y, 3), y)   → 3
        _is_pow_y(Pow(y, 1), y)   → None  (n=1 is linear, not Bernoulli)
        _is_pow_y(Pow(y, 0), y)   → None  (n=0 is trivially handled)
        _is_pow_y(Pow(x, 2), y)   → None  (base is not y)
    """
    if (
        isinstance(node, IRApply)
        and node.head == POW
        and len(node.args) == 2
        and node.args[0] == y
        and isinstance(node.args[1], IRInteger)
    ):
        n = node.args[1].value
        if n != 0 and n != 1:
            return n
    return None


def _try_bernoulli(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Solve ``y' + P(x)·y = Q(x)·y^n``  (``n ≠ 0, 1``) via Bernoulli's
    substitution ``v = y^(1-n)``.

    Background
    ----------
    The substitution ``v = y^(1-n)`` converts the nonlinear Bernoulli
    equation into a first-order **linear** ODE::

        v' + (1-n)·P(x)·v = (1-n)·Q(x)

    We solve this linear ODE for ``v`` with the existing integrating-factor
    solver, then back-substitute::

        y = v^{1/(1-n)}

    Recognition (zero form)
    -----------------------
    The ODE arrives as ``D(y,x) + P(x)·y − Q(x)·y^n = 0``.  We scan the
    flattened summands looking for:

    - ``D(y, x)`` — the ``y'`` term (coefficient must be 1).
    - ``y^n`` — the nonlinear power (any integer ``n ≠ 0,1``).  Its
      coefficient (times ``−1``) is ``Q(x)``.
    - ``y`` — any remaining y-terms contribute to ``P(x)``.
    - Unknown ``y``-dependent terms → give up (return ``None``).

    Parameters
    ----------
    expr : IRNode
        The ODE expression in zero form.
    y, x : IRSymbol
        Dependent and independent variable symbols.
    vm : VM
        Live symbolic VM — passed to the inner linear solver.

    Returns
    -------
    ``Equal(y, solution)`` on success, or ``None`` to signal fall-through.
    """
    terms = _flatten_add(expr)
    y_prime = IRApply(D, (y, x))

    yprime_coeff = Fraction(0)
    p_terms: list[tuple[IRNode, bool]] = []   # coefficient × y  → P
    q_terms: list[tuple[IRNode, bool]] = []   # coefficient × y^n → Q (negated)
    n_power: int | None = None

    for term in terms:
        neg, core = _unwrap_neg(term)

        # ---- D(y, x) bare or coeff * D(y, x) --------------------------------
        if core == y_prime:
            yprime_coeff += Fraction(-1 if neg else 1)
            continue

        mul_res = _split_coeff_base(core)
        if mul_res is not None:
            coeff_node, base = mul_res
            if base == y_prime and _is_const_wrt(coeff_node, y):
                f, _ = _extract_coeff(coeff_node, x)
                if f is not None:
                    yprime_coeff += Fraction(-1 if neg else 1) * f
                    continue

        # ---- Pow(y, n) term or coeff * Pow(y, n) ----------------------------
        base_node = core
        coeff_node2: IRNode = _ONE
        if mul_res is not None:
            coeff_node2, base_node = mul_res

        pn = _is_pow_y(base_node, y)
        if pn is not None and _is_const_wrt(coeff_node2, y):
            if n_power is None:
                n_power = pn
            elif n_power != pn:
                return None  # Two different y-powers — not Bernoulli
            # In zero form the y^n appears as "−Q(x)·y^n = 0", so Q = coeff.
            q_terms.append((coeff_node2, not neg))
            continue

        # ---- y term (contributes to P) --------------------------------------
        if base_node == y and _is_const_wrt(coeff_node2, y):
            p_terms.append((coeff_node2, neg))
            continue
        if core == y:
            p_terms.append((_ONE, neg))
            continue

        # ---- Unknown term — not a Bernoulli ODE -----------------------------
        return None

    if yprime_coeff != 1 or n_power is None:
        return None  # y' missing or no y^n power — not Bernoulli

    n = n_power
    one_minus_n = Fraction(1 - n)

    # Build P(x) and Q(x) from collected terms.
    p_ir = _sum_of_terms(p_terms)
    q_ir = _sum_of_terms(q_terms)

    # Reduced linear ODE: v' + (1-n)·P·v = (1-n)·Q
    # We reuse solve_linear_first_order with y as the v-variable name, which
    # is valid because P and Q are x-only for a proper Bernoulli equation.
    omn_ir = _frac_to_ir(one_minus_n)
    new_p = vm.eval(_mul(omn_ir, p_ir))
    new_q = vm.eval(_mul(omn_ir, q_ir))

    lin_result = solve_linear_first_order(new_p, new_q, y, x, vm)

    # Detect sentinel failure (integration fell through).
    if (
        isinstance(lin_result, IRApply)
        and lin_result.head == ODE2
        and len(lin_result.args) >= 1
        and isinstance(lin_result.args[0], IRInteger)
        and lin_result.args[0].value == -999
    ):
        return None

    if not (isinstance(lin_result, IRApply) and lin_result.head == EQUAL):
        return None

    v_sol = lin_result.args[1]  # solution for v = y^(1-n)

    # Back-substitute: y = v^(1/(1-n))
    exp_frac = Fraction(1) / one_minus_n   # 1 / (1-n)
    exp_ir = _frac_to_ir(exp_frac)
    y_sol = vm.eval(_pow(v_sol, exp_ir))

    return IRApply(EQUAL, (y, y_sol))


# ---------------------------------------------------------------------------
# Section 9 — Exact ODE recogniser and solver
# ---------------------------------------------------------------------------


def _fold_numeric(node: IRNode) -> IRNode:
    """Recursively collapse ``Mul(rational, Mul(rational, expr))`` patterns.

    The VM's differentiation rule for ``d/dx (k·xⁿ)`` produces
    ``Mul(k, Mul(n, x^(n-1)))`` rather than ``Mul(k·n, x^(n-1))``.  This
    prevents structural equality between mathematically equal expressions like
    ``Mul(3, Mul(2, x))`` and ``Mul(6, x)``.

    ``_fold_numeric`` makes one full pass over the tree and folds adjacent
    rational constants in ``Mul`` nodes so that structural equality works for
    the exactness check ``∂M/∂y = ∂N/∂x``.

    Examples::

        _fold_numeric(Mul(3, Mul(2, x)))  → Mul(6, x)
        _fold_numeric(Mul(2, Mul(3, y)))  → Mul(6, y)
        _fold_numeric(Add(Mul(2, x), 3))  → Add(Mul(2, x), 3)
    """
    if not isinstance(node, IRApply):
        return node
    # Recurse into arguments first.
    new_args = tuple(_fold_numeric(a) for a in node.args)
    folded = IRApply(node.head, new_args)

    # Fold Mul(rational, Mul(rational, expr)) → Mul(product, expr)
    if folded.head == MUL and len(folded.args) == 2:
        left, right = folded.args
        if isinstance(left, (IRInteger, IRRational)):
            a_frac = (
                Fraction(left.value)
                if isinstance(left, IRInteger)
                else Fraction(left.numer, left.denom)
            )
            if (
                isinstance(right, IRApply)
                and right.head == MUL
                and len(right.args) == 2
            ):
                rl, rr = right.args
                if isinstance(rl, (IRInteger, IRRational)):
                    b_frac = (
                        Fraction(rl.value)
                        if isinstance(rl, IRInteger)
                        else Fraction(rl.numer, rl.denom)
                    )
                    return _mul(_frac_to_ir(a_frac * b_frac), rr)
    return folded


def _eval_at_xy(
    node: IRNode,
    x_sym: IRSymbol,
    y_sym: IRSymbol,
    x_val: float,
    y_val: float,
) -> float:
    """Numerically evaluate ``node`` at ``(x, y) = (x_val, y_val)``.

    Used only for the exactness check.  Raises ``ValueError`` for
    unsupported heads (caller catches and falls through).
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRSymbol):
        if node == x_sym:
            return x_val
        if node == y_sym:
            return y_val
        raise ValueError(f"Unknown symbol: {node.name}")
    if not isinstance(node, IRApply):
        raise ValueError(f"Unsupported node: {node!r}")
    head = node.head.name
    ev = lambda n: _eval_at_xy(n, x_sym, y_sym, x_val, y_val)  # noqa: E731
    if head == "Add":
        return ev(node.args[0]) + ev(node.args[1])
    if head == "Sub":
        return ev(node.args[0]) - ev(node.args[1])
    if head == "Mul":
        return ev(node.args[0]) * ev(node.args[1])
    if head == "Div":
        return ev(node.args[0]) / ev(node.args[1])
    if head == "Neg":
        return -ev(node.args[0])
    if head == "Pow":
        return ev(node.args[0]) ** ev(node.args[1])
    if head == "Exp":
        return math.exp(ev(node.args[0]))
    if head == "Log":
        return math.log(abs(ev(node.args[0])))
    if head == "Sin":
        return math.sin(ev(node.args[0]))
    if head == "Cos":
        return math.cos(ev(node.args[0]))
    raise ValueError(f"Unsupported head: {head}")


def _exprs_equal_numerically(
    a: IRNode,
    b: IRNode,
    x: IRSymbol,
    y: IRSymbol,
    tol: float = 1e-9,
) -> bool:
    """Return ``True`` if ``a`` and ``b`` are numerically equal at four interior
    test points ``(x, y) ∈ {(0.5,0.5), (1.3,0.7), (0.8,1.2), (2.0,0.3)}``.

    Falls back to ``False`` if numerical evaluation fails (e.g. division by
    zero, unsupported node type).

    This is used exclusively for the exactness check ``∂M/∂y = ∂N/∂x`` where
    the VM may return mathematically equal but structurally different IR trees.
    """
    test_pts = [(0.5, 0.5), (1.3, 0.7), (0.8, 1.2), (2.0, 0.3)]
    try:
        for xv, yv in test_pts:
            va = _eval_at_xy(a, x, y, xv, yv)
            vb = _eval_at_xy(b, x, y, xv, yv)
            if abs(va - vb) > tol:
                return False
        return True
    except (ValueError, ZeroDivisionError, OverflowError):
        return False


def _try_exact(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Solve ``M(x,y)·dx + N(x,y)·dy = 0`` when ``∂M/∂y = ∂N/∂x``.

    Background
    ----------
    A first-order ODE is *exact* when the vector field ``(M, N)`` is the
    gradient of some scalar potential ``F(x, y)``::

        ∂F/∂x = M(x,y)    ∂F/∂y = N(x,y)

    The necessary and sufficient exactness condition is ``∂M/∂y = ∂N/∂x``.
    When it holds, the solution is the implicit equation ``F(x, y) = C``.

    We compute ``F`` in two steps:

    1.  ``F = ∫ M(x,y) dx``  — integrate ``M`` with respect to ``x``,
        treating ``y`` as a parameter.  The "constant of integration" is
        an arbitrary function ``g(y)`` yet to be determined.

    2.  Differentiate ``F`` with respect to ``y`` and match to ``N``::

            ∂F/∂y + g'(y) = N   →   g'(y) = N − ∂F/∂y

        Integrate ``g'`` with respect to ``y`` to get ``g``.  Then
        ``F + g = C`` is the full implicit solution.

    Input form
    ----------
    The ODE arrives normalised as ``M + N·D(y,x) = 0`` (zero form).
    We separate terms by whether they contain ``D(y,x)``.

    Parameters
    ----------
    expr : IRNode
        ODE in zero form.
    y, x : IRSymbol
        Dependent and independent variables.
    vm : VM
        Live VM — used for differentiation and integration.

    Returns
    -------
    ``Equal(F + g, C_CONST)`` (implicit solution) on success, or ``None``.
    """
    terms = _flatten_add(expr)
    y_prime = IRApply(D, (y, x))

    n_parts: list[tuple[IRNode, bool]] = []  # coefficient of y'
    m_parts: list[tuple[IRNode, bool]] = []  # y'-free terms

    for term in terms:
        neg, core = _unwrap_neg(term)
        mul_res = _split_coeff_base(core)

        if core == y_prime:
            n_parts.append((_ONE, neg))
            continue

        if mul_res is not None:
            coeff_node, base = mul_res
            if base == y_prime:
                n_parts.append((coeff_node, neg))
                continue
            if coeff_node == y_prime:
                # y' * something — treat 'something' as coefficient of y'
                n_parts.append((base, neg))
                continue

        m_parts.append((core, neg))

    if not n_parts:
        return None  # No y' term — not this form

    M_raw = _sum_of_terms(m_parts)
    N_raw = _sum_of_terms(n_parts)
    M = vm.eval(M_raw)
    N = vm.eval(N_raw)

    # Exactness check: ∂M/∂y = ∂N/∂x
    # The VM may return structurally different but mathematically equal forms
    # (e.g. Mul(3,Mul(2,x)) vs Mul(6,x)), so we verify numerically at four
    # interior test points rather than relying on structural equality.
    dM_dy = vm.eval(IRApply(D, (M, y)))
    dN_dx = vm.eval(IRApply(D, (N, x)))
    if not _exprs_equal_numerically(dM_dy, dN_dx, x, y):
        return None

    # Step 1: F = ∫ M dx  (y treated as a parameter by the VM)
    F = vm.eval(IRApply(INTEGRATE, (M, x)))
    if _is_unevaluated_integrate(F, x):
        return None

    # Step 2: g'(y) = N − ∂F/∂y
    dF_dy = vm.eval(IRApply(D, (F, y)))
    g_prime = vm.eval(_sub(N, dF_dy))

    # Step 3: g = ∫ g'(y) dy
    g = vm.eval(IRApply(INTEGRATE, (g_prime, y)))
    if _is_unevaluated_integrate(g, y):
        return None

    # Implicit solution: F(x,y) + g(y) = C
    potential = vm.eval(_add(F, g))
    return IRApply(EQUAL, (potential, C_CONST))


# ---------------------------------------------------------------------------
# Section 10 — 2nd-order non-homogeneous: undetermined coefficients
# ---------------------------------------------------------------------------


def _collect_second_order_nonhom(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
) -> tuple[Fraction, Fraction, Fraction, IRNode] | None:
    """Try to read ``(a, b, c, f)`` from ``a·y'' + b·y' + c·y − f(x) = 0``.

    Extends :func:`_collect_second_order_coeffs` to also collect the
    forcing term ``f(x)`` — the x-only terms that do not involve ``y``.

    The forcing terms appear on the left-hand side of the zero form, so they
    carry a sign flip: a term ``−sin(x)`` in the zero form contributes
    ``sin(x)`` to ``f``.

    Returns ``(a, b, c, f_ir)`` if the ODE is second-order constant-
    coefficient with a non-trivial forcing.  Returns ``None`` if:

    - The leading coefficient ``a`` is zero (not second-order).
    - The ODE is homogeneous (no x-only forcing terms).
    - Any coefficient is non-constant.
    - Any term is y-dependent in a way that doesn't fit the pattern.
    """
    y_prime = IRApply(D, (y, x))
    y_double = IRApply(D, (y_prime, x))
    terms = _flatten_add(expr)

    a = Fraction(0)
    b = Fraction(0)
    c = Fraction(0)
    forcing: list[IRNode] = []

    for term in terms:
        coeff, base = _extract_coeff(term, x)
        if base == y_double:
            if coeff is None:
                return None   # Variable coefficient — not handled
            a += coeff
        elif base == y_prime:
            if coeff is None:
                return None
            b += coeff
        elif base == y:
            if coeff is None:
                return None
            c += coeff
        elif _is_const_wrt(term, y):
            # This term is part of f(x) — it appears as −f on the LHS.
            forcing.append(_neg(term))
        else:
            return None  # y-dependent forcing — not handled

    if a == Fraction(0):
        return None   # Not second-order
    if not forcing:
        return None   # Homogeneous — route to the homogeneous solver

    f_ir: IRNode = forcing[0]
    for t in forcing[1:]:
        f_ir = _add(f_ir, t)

    return (a, b, c, f_ir)


def _extract_linear_coeff_x(arg: IRNode, x: IRSymbol) -> Fraction | None:
    """Return ``α`` if ``arg = α·x`` (no constant term, α rational).

    Handles the patterns produced by the MACSYMA compiler:

    - ``x``               → ``Fraction(1)``
    - ``Mul(α, x)``       → ``α``
    - ``Neg(x)``          → ``Fraction(-1)``
    - ``Neg(Mul(α, x))``  → ``-α``

    Returns ``None`` for anything else (constant, polynomial, etc.).
    """
    if arg == x:
        return Fraction(1)
    if isinstance(arg, IRApply) and arg.head == NEG and len(arg.args) == 1:
        inner = _extract_linear_coeff_x(arg.args[0], x)
        return (-inner) if inner is not None else None
    if isinstance(arg, IRApply) and arg.head == MUL and len(arg.args) == 2:
        left, right = arg.args
        if right == x and _is_const_wrt(left, x):
            c, _ = _extract_coeff(left, x)
            return c
        if left == x and _is_const_wrt(right, x):
            c, _ = _extract_coeff(right, x)
            return c
    return None


def _try_polynomial_forcing(
    f: IRNode, x: IRSymbol
) -> list[Fraction] | None:
    """Try to extract polynomial coefficients from ``f``.

    Returns ``coeffs`` where ``coeffs[k] = coefficient of x^k``, or
    ``None`` if ``f`` is not a polynomial we can represent.

    Supported degrees: 0, 1, 2.  Degree 3+ returns ``None``.

    Examples::

        _try_polynomial_forcing(IRInteger(3), x)        → [Fraction(3)]
        _try_polynomial_forcing(Add(Mul(2,x), 1), x)   → [Fraction(1), Fraction(2)]
        _try_polynomial_forcing(Add(Mul(3,Pow(x,2)),x), x)  → [0, 1, 3]
    """
    terms = _flatten_add(f)
    by_degree: dict[int, Fraction] = {}

    for term in terms:
        neg, core = _unwrap_neg(term)
        sign = Fraction(-1) if neg else Fraction(1)

        if _is_const_wrt(core, x):
            c, _ = _extract_coeff(core, x)
            if c is None:
                return None
            by_degree[0] = by_degree.get(0, Fraction(0)) + sign * c
            continue

        if core == x:
            by_degree[1] = by_degree.get(1, Fraction(0)) + sign
            continue

        mul_res = _split_coeff_base(core)
        if mul_res is not None:
            coeff_node, base = mul_res

            # Degree-1: coeff * x
            if base == x and _is_const_wrt(coeff_node, x):
                c, _ = _extract_coeff(coeff_node, x)
                if c is None:
                    return None
                by_degree[1] = by_degree.get(1, Fraction(0)) + sign * c
                continue

            # Degree-2: coeff * x^2
            if (
                isinstance(base, IRApply)
                and base.head == POW
                and len(base.args) == 2
                and base.args[0] == x
                and isinstance(base.args[1], IRInteger)
                and base.args[1].value == 2
                and _is_const_wrt(coeff_node, x)
            ):
                c, _ = _extract_coeff(coeff_node, x)
                if c is None:
                    return None
                by_degree[2] = by_degree.get(2, Fraction(0)) + sign * c
                continue

        # Bare x^n (degree ≥ 2, no coefficient)
        if (
            isinstance(core, IRApply)
            and core.head == POW
            and len(core.args) == 2
            and core.args[0] == x
            and isinstance(core.args[1], IRInteger)
        ):
            k = core.args[1].value
            if k > 2:
                return None   # Degree too high for this implementation
            by_degree[k] = by_degree.get(k, Fraction(0)) + sign
            continue

        return None   # Unrecognised term

    if not by_degree:
        return None
    max_deg = max(by_degree.keys())
    return [by_degree.get(k, Fraction(0)) for k in range(max_deg + 1)]


def _classify_forcing(f: IRNode, x: IRSymbol) -> tuple | None:
    """Classify the forcing function for the method of undetermined coefficients.

    The method of undetermined coefficients applies when ``f(x)`` belongs
    to the exponential-polynomial-trig (EPT) family.  We recognise six
    sub-families and return a tuple tag:

    - ``('const', k)``              — constant ``k``
    - ``('poly', coeffs)``          — polynomial ``Σcₖxᵏ``, degree ≤ 2
    - ``('exp', α)``                — ``e^(α·x)``
    - ``('sin', β)``                — ``sin(β·x)``
    - ``('cos', β)``                — ``cos(β·x)``
    - ``('exp_sin', α, β)``         — ``e^(α·x)·sin(β·x)``
    - ``('exp_cos', α, β)``         — ``e^(α·x)·cos(β·x)``

    Returns ``None`` for anything else (e.g. ``ln(x)``, ``tanh(x)``).

    All ``α``, ``β`` values are rational ``Fraction``; ``β > 0`` by
    convention.  The polynomial ``coeffs`` list has ``coeffs[k] = coeff
    of x^k``.
    """
    # Constant
    if _is_const_wrt(f, x):
        coeff, _ = _extract_coeff(f, x)
        if coeff is not None:
            return ('const', coeff)
        return None

    # Exp(α·x)
    if isinstance(f, IRApply) and f.head == EXP and len(f.args) == 1:
        alpha = _extract_linear_coeff_x(f.args[0], x)
        if alpha is not None:
            return ('exp', alpha)

    # Sin(β·x) / Cos(β·x)
    if isinstance(f, IRApply) and f.head in (SIN, COS) and len(f.args) == 1:
        beta = _extract_linear_coeff_x(f.args[0], x)
        if beta is not None and beta > 0:
            return ('sin' if f.head == SIN else 'cos', beta)

    # Mul(Exp(α·x), Sin/Cos(β·x)) or swapped order
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        left, right = f.args
        for e_part, trig_part in [(left, right), (right, left)]:
            if not (isinstance(e_part, IRApply) and e_part.head == EXP):
                continue
            alpha = _extract_linear_coeff_x(e_part.args[0], x)
            if alpha is None:
                continue
            if isinstance(trig_part, IRApply) and trig_part.head in (SIN, COS):
                beta = _extract_linear_coeff_x(trig_part.args[0], x)
                if beta is not None and beta > 0:
                    kind = 'exp_sin' if trig_part.head == SIN else 'exp_cos'
                    return (kind, alpha, beta)

    # Polynomial (sum of rational multiples of 1, x, x²)
    poly = _try_polynomial_forcing(f, x)
    if poly is not None:
        return ('poly', poly)

    return None


def _char_poly_at(a: Fraction, b: Fraction, c: Fraction, r: Fraction) -> Fraction:
    """Evaluate the characteristic polynomial ``a·r² + b·r + c`` at ``r``."""
    return a * r * r + b * r + c


def _compute_particular(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    forcing: tuple,
    x: IRSymbol,
) -> IRNode | None:
    """Compute the particular solution ``y_p`` by undetermined coefficients.

    Given the ODE ``a·y'' + b·y' + c·y = f(x)`` and the classified forcing
    tag from :func:`_classify_forcing`, we determine the ansatz for ``y_p``,
    substitute into the ODE, and solve the resulting algebraic system for the
    undetermined coefficients.

    Resonance
    ---------
    If the "natural frequency" of the forcing matches a root of the
    characteristic polynomial, the standard ansatz fails.  We multiply by
    ``x^s`` where ``s`` is the multiplicity of the root:

    - ``s = 0``:  no resonance
    - ``s = 1``:  simple resonance
    - ``s = 2``:  double resonance

    All arithmetic is exact rational (``Fraction``).

    Parameters
    ----------
    a, b, c : Fraction
        Characteristic polynomial coefficients.
    forcing : tuple
        Output of :func:`_classify_forcing`.
    x : IRSymbol
        Independent variable.

    Returns
    -------
    IR expression for ``y_p``, or ``None`` if the case is not handled.
    """
    kind = forcing[0]

    # ------------------------------------------------------------------
    # Constant forcing: f(x) = k
    # Ansatz y_p = A·x^s (s depends on resonance)
    # ------------------------------------------------------------------
    if kind == 'const':
        k: Fraction = forcing[1]
        if c != 0:
            return _frac_to_ir(k / c)
        if b != 0:
            return _mul(_frac_to_ir(k / b), x)
        if a != 0:
            return _mul(_frac_to_ir(k / (2 * a)), _pow(x, _TWO))
        return None

    # ------------------------------------------------------------------
    # Polynomial forcing: f(x) = Σ coeffs[k]·x^k  (degree ≤ 2)
    # Ansatz y_p = x^s · Σ Aₖ·x^k  matching degrees.
    # ------------------------------------------------------------------
    if kind == 'poly':
        coeffs: list[Fraction] = forcing[1]
        n = len(coeffs) - 1   # degree of f

        # Resonance shift: s=0 if 0 is not a char-root; s=1 if simple root; s=2 double
        if c != 0:
            s = 0
        elif b != 0:
            s = 1
        else:
            s = 2

        if n == 0:
            # Delegate to 'const' logic.
            return _compute_particular(a, b, c, ('const', coeffs[0]), x)

        if n == 1:
            k0, k1 = coeffs[0], coeffs[1]
            if s == 0:
                # y_p = A0 + A1·x;  y_p'' = 0,  y_p' = A1
                # Equations:  c·A1 = k1,  c·A0 + b·A1 = k0
                if c == 0:
                    return None
                A1 = k1 / c
                A0 = (k0 - b * A1) / c
                return _add(_frac_to_ir(A0), _mul(_frac_to_ir(A1), x))
            if s == 1:
                # y_p = A0·x + A1·x²;  y_p' = A0 + 2A1·x,  y_p'' = 2A1
                # c=0.  Equations: (b·A0 + 2a·A1) = k0,  2b·A1 = k1
                if b == 0:
                    return None
                A1 = k1 / (2 * b)
                A0 = (k0 - 2 * a * A1) / b
                return _mul(x, _add(_frac_to_ir(A0), _mul(_frac_to_ir(A1), x)))
            return None

        if n == 2:
            k0, k1, k2 = coeffs[0], coeffs[1], coeffs[2]
            if s == 0:
                # y_p = A0 + A1·x + A2·x²
                # y_p' = A1 + 2A2·x,  y_p'' = 2A2
                # Equations (by power of x):
                #   x^2: c·A2 = k2
                #   x^1: 2b·A2 + c·A1 = k1
                #   x^0: 2a·A2 + b·A1 + c·A0 = k0
                if c == 0:
                    return None
                A2 = k2 / c
                A1 = (k1 - 2 * b * A2) / c
                A0 = (k0 - 2 * a * A2 - b * A1) / c
                result: IRNode = _frac_to_ir(A0)
                if A1 != 0:
                    result = _add(result, _mul(_frac_to_ir(A1), x))
                if A2 != 0:
                    result = _add(result, _mul(_frac_to_ir(A2), _pow(x, _TWO)))
                return result
            return None  # s ≥ 1 with quadratic forcing — skip

        return None   # Degree > 2

    # ------------------------------------------------------------------
    # Exponential forcing: f(x) = e^(α·x)
    # Ansatz: A·x^s·e^(αx)
    # ------------------------------------------------------------------
    if kind == 'exp':
        alpha: Fraction = forcing[1]
        arg_ir = _mul(_frac_to_ir(alpha), x)

        char_val = _char_poly_at(a, b, c, alpha)
        if char_val != 0:
            # s = 0:  A·e^(αx),  equation: char_val·A = 1
            return _mul(_frac_to_ir(Fraction(1) / char_val), _exp(arg_ir))

        # s = 1:  A·x·e^(αx)
        # Substituting y_p = A·x·e^(αx):
        #   y_p'  = A·e^(αx) · (1 + αx)
        #   y_p'' = A·e^(αx) · (2α + α²x)
        # Coefficient of e^(αx) (after the x-term cancels by resonance):
        #   (2aα + b) · A = 1
        char_prime = 2 * a * alpha + b   # p'(α)
        if char_prime != 0:
            return _mul(_mul(_frac_to_ir(Fraction(1) / char_prime), x), _exp(arg_ir))

        # s = 2:  A·x²·e^(αx) — double resonance
        # Equation: 2a·A = 1
        if a != 0:
            return _mul(
                _mul(_frac_to_ir(Fraction(1) / (2 * a)), _pow(x, _TWO)),
                _exp(arg_ir),
            )
        return None

    # ------------------------------------------------------------------
    # Trig forcing: f = sin(β·x) or cos(β·x)
    # Ansatz: A·cos(β·x) + B·sin(β·x)
    #
    # Substituting into a·y_p'' + b·y_p' + c·y_p:
    #   y_p''  = −β²·y_p
    #   y_p'   = β·(−A·sin + B·cos)
    # Collecting by cos and sin:
    #   cos coefficient: (c − a·β²)·A + b·β·B = rhs_cos
    #   sin coefficient: (c − a·β²)·B − b·β·A = rhs_sin
    # where rhs_cos=1, rhs_sin=0 for cos forcing, and vice-versa for sin.
    # ------------------------------------------------------------------
    if kind in ('sin', 'cos'):
        beta: Fraction = forcing[1]
        p = c - a * beta * beta          # c − a·β²
        q_val = b * beta                 # b·β
        det = p * p + q_val * q_val
        if det == 0:
            # Resonance — x·(A·cos + B·sin) ansatz not yet implemented.
            return None

        if kind == 'cos':
            # p·A + q·B = 1,   −q·A + p·B = 0
            A = p / det
            B = q_val / det
        else:  # 'sin'
            # p·A + q·B = 0,   −q·A + p·B = 1
            A = -q_val / det
            B = p / det

        arg_trig = _mul(_frac_to_ir(beta), x)
        cos_term = _mul(_frac_to_ir(A), _cos(arg_trig))
        sin_term = _mul(_frac_to_ir(B), _sin(arg_trig))

        if A == 0:
            return sin_term
        if B == 0:
            return cos_term
        return _add(cos_term, sin_term)

    # ------------------------------------------------------------------
    # Exponential × trig: f = e^(αx)·sin(βx) or e^(αx)·cos(βx)
    # Ansatz: e^(αx)·(A·cos(βx) + B·sin(βx))
    #
    # Exponential shift theorem: L[e^(αx)·u] = e^(αx)·Lα[u] where
    #   Lα[u] = a·u'' + (2aα + b)·u' + (aα² + bα + c)·u
    # So we reduce to the pure trig case with:
    #   b_eff = 2aα + b
    #   c_eff = a·α² + b·α + c = char_poly(α)
    # ------------------------------------------------------------------
    if kind in ('exp_sin', 'exp_cos'):
        alpha2: Fraction = forcing[1]
        beta2: Fraction = forcing[2]

        b_eff = 2 * a * alpha2 + b
        c_eff = _char_poly_at(a, b, c, alpha2)

        p2 = c_eff - a * beta2 * beta2
        q2 = b_eff * beta2
        det2 = p2 * p2 + q2 * q2
        if det2 == 0:
            return None   # Resonance — not handled

        if kind == 'exp_sin':
            A2 = -q2 / det2
            B2 = p2 / det2
        else:  # 'exp_cos'
            A2 = p2 / det2
            B2 = q2 / det2

        exp_part = _exp(_mul(_frac_to_ir(alpha2), x))
        cos_part = _mul(_frac_to_ir(A2), _cos(_mul(_frac_to_ir(beta2), x)))
        sin_part = _mul(_frac_to_ir(B2), _sin(_mul(_frac_to_ir(beta2), x)))

        if A2 == 0:
            trig_part: IRNode = sin_part
        elif B2 == 0:
            trig_part = cos_part
        else:
            trig_part = _add(cos_part, sin_part)

        return _mul(exp_part, trig_part)

    return None


def _try_second_order_nonhom(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Solve ``a·y'' + b·y' + c·y = f(x)`` via undetermined coefficients.

    Algorithm
    ---------
    1.  :func:`_collect_second_order_nonhom` — extract (a, b, c, f).
    2.  :func:`_classify_forcing` — identify the forcing-function family.
    3.  :func:`_compute_particular` — compute the particular solution y_p.
    4.  Homogeneous solution y_h via :func:`solve_second_order_const_coeff`.
    5.  General solution: ``Equal(y, y_h + y_p)``.

    Parameters
    ----------
    expr : IRNode
        ODE in zero form.
    y, x : IRSymbol
        Dependent and independent variable.
    vm : VM
        Live VM — used to simplify the combined solution.

    Returns
    -------
    ``Equal(y, y_h + y_p)`` on success, or ``None`` to fall through.
    """
    collected = _collect_second_order_nonhom(expr, y, x)
    if collected is None:
        return None

    a, b, c, f_ir = collected
    # Simplify f_ir through the VM to eliminate double negations and other
    # structural noise that would confuse _classify_forcing.
    f_ir = vm.eval(f_ir)

    forcing = _classify_forcing(f_ir, x)
    if forcing is None:
        return None

    y_p = _compute_particular(a, b, c, forcing, x)
    if y_p is None:
        return None

    hom_result = solve_second_order_const_coeff(a, b, c, y, x)
    if not (isinstance(hom_result, IRApply) and hom_result.head == EQUAL):
        return None

    y_h = hom_result.args[1]
    y_gen = vm.eval(_add(y_h, y_p))
    return IRApply(EQUAL, (y, y_gen))


# ---------------------------------------------------------------------------
# Section 11 — Top-level dispatcher
# ---------------------------------------------------------------------------


def solve_ode(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Dispatch an ODE to the appropriate solver.

    The ``expr`` argument is the ODE in zero form: all terms have been
    moved to the left-hand side so ``expr = 0``.

    Dispatch order
    --------------
    1. Check for **non-homogeneous** 2nd-order (new Phase 18) — must come
       before the homogeneous check, because the homogeneous recogniser
       silently ignores forcing terms and would mis-classify.
    2. Check for second-order const-coeff **homogeneous**.
    3. Check for **Bernoulli** (new Phase 18).
    4. Check for **exact** (new Phase 18).
    5. Check for first-order **linear** ``y' + P(x)·y = Q(x)``.
    6. Check for **separable** ``y' = f(x)·g(y)``.

    Returns ``Equal(y, solution)`` or ``Equal(F, C)`` (exact) on success,
    or ``None`` on failure.

    Parameters
    ----------
    expr:
        The ODE expression that equals zero (LHS of the ODE).
    y:
        The dependent variable (what we solve for).
    x:
        The independent variable.
    vm:
        The live symbolic VM — needed for calling ``Integrate``.

    Returns
    -------
    An ``Equal`` IR node if a solver matches, else ``None``.
    """
    # ---- Phase 18: 2nd-order non-homogeneous --------------------------------
    nonhom = _try_second_order_nonhom(expr, y, x, vm)
    if nonhom is not None:
        return nonhom

    # ---- 2nd-order homogeneous (Phase 0.1.0) --------------------------------
    coeffs = _collect_second_order_coeffs(expr, y, x)
    if coeffs is not None:
        a, b, c = coeffs
        return solve_second_order_const_coeff(a, b, c, y, x)

    # ---- Phase 18: Bernoulli ------------------------------------------------
    bern = _try_bernoulli(expr, y, x, vm)
    if bern is not None:
        return bern

    # ---- First-order linear (Phase 0.1.0) -----------------------------------
    # Linear runs before exact so that y' + P(x)y = Q(x) gives an explicit
    # y = f(x) solution rather than being caught by the exact solver in
    # implicit form.
    linear = _collect_linear_first_order(expr, y, x)
    if linear is not None:
        p_ir, q_ir = linear
        result = solve_linear_first_order(p_ir, q_ir, y, x, vm)
        # Detect sentinel (integration failed).
        if (
            isinstance(result, IRApply)
            and result.head == ODE2
            and result.args[0] == IRInteger(-999)
        ):
            return None
        return result

    # ---- Separable (Phase 0.1.0) --------------------------------------------
    # Separable also before exact for the same reason (prefer explicit form).
    sep = _try_separable(expr, y, x, vm)
    if sep is not None:
        return sep

    # ---- Phase 18: Exact ODE (last resort for first-order) ------------------
    # Exact runs last so that explicitly solvable ODEs (y'=f(x), y'+Py=Q)
    # have already been handled and return an explicit y = solution form.
    # Exact ODEs with truly y-dependent M or N reach here only.
    exact = _try_exact(expr, y, x, vm)
    if exact is not None:
        return exact

    return None
