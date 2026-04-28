"""Symbolic ODE solver — the heart of cas-ode.

This module provides pure-Python functions that recognise and solve four
classes of ordinary differential equations whose solutions can always be
written in closed form:

1. **First-order linear**: ``dy/dx + P(x)·y = Q(x)``
2. **Separable**: ``dy/dx = f(x)·g(y)`` (including the degenerate cases
   ``g(y) = 1`` and ``f(x) = 1``)
3. **Bernoulli**: ``dy/dx + P(x)·y = Q(x)·y^n``  (reduces to linear in ``v``)
4. **Second-order constant-coefficient homogeneous**:
   ``a·y'' + b·y' + c·y = 0``

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

1. :func:`_is_const_wrt`  — "does this IR subtree depend on ``x``?"
2. :func:`_collect_second_order_coeffs` — pattern-match ``a·y''+b·y'+c·y``
3. :func:`solve_second_order_const_coeff` — characteristic equation → roots
4. :func:`_collect_linear_first_order` — pattern-match ``y' + P(x)·y``
5. :func:`solve_linear_first_order` — integrating-factor method
6. :func:`solve_separable` — separation of variables
7. :func:`solve_ode` — the top-level dispatcher
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
# Section 7 — Top-level dispatcher
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
    1. Check for second-order constant-coefficient homogeneous form.
    2. Check for first-order linear form ``y' + P(x)·y = Q(x)``.
    3. Check for separable form ``y' = f(x)·g(y)``.

    Returns ``Equal(y, solution)`` on success, or ``None`` on failure.

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
    ``Equal(y, solution)`` if a solver matches, else ``None``.
    """
    # ---- Try second-order first (requires y'') ------------------------------
    coeffs = _collect_second_order_coeffs(expr, y, x)
    if coeffs is not None:
        a, b, c = coeffs
        return solve_second_order_const_coeff(a, b, c, y, x)

    # ---- Try first-order linear ---------------------------------------------
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

    # ---- Try separable -------------------------------------------------------
    sep = _try_separable(expr, y, x, vm)
    if sep is not None:
        return sep

    return None
