"""Transcendental equation solving — Phase 26.

Extends ``cas-solve`` to handle equation families that are *not* polynomial in
the solve variable, but can still be inverted or reduced to simpler forms.

Covered families
----------------

26a — Trigonometric (periodic, linear argument)::

    sin(ax + b) = c  →  [arcsin(c) + 2k·π,  π − arcsin(c) + 2k·π]
                         (divide by a, subtract b/a for the x solution)
    cos(ax + b) = c  →  [arccos(c) + 2k·π, −arccos(c) + 2k·π]
    tan(ax + b) = c  →  [arctan(c) + k·π]

The free-integer constant ``%k`` is represented as ``FREE_INTEGER`` in the IR.

26b — Exponential / logarithmic (linear argument)::

    exp(ax + b) = c  →  ax + b = log(c)   →  x = (log(c) − b) / a
    log(ax + b) = c  →  ax + b = exp(c)   →  x = (exp(c) − b) / a

26c — Lambert W (principal branch, linear inner function)::

    f(x) · exp(f(x)) = c, f linear in x  →  f(x) = W(c)  →  x = (W(c) − b) / a

26d — Hyperbolic (linear argument)::

    sinh(ax + b) = c  →  ax + b = asinh(c)  (unique)
    cosh(ax + b) = c  →  ax + b = ±acosh(c)  (two branches)
    tanh(ax + b) = c  →  ax + b = atanh(c)  (unique)

26e — Compound (polynomial in a single transcendental substitution)::

    sin(x)² + sin(x) = 0  →  u = sin(x),  u² + u = 0  →  u ∈ {−1, 0}
                              then solve sin(x) = u for each u recursively.
    exp(x)² − 3·exp(x) + 2 = 0  →  u = exp(x),  u² − 3u + 2 = 0  →  u ∈ {1, 2}
                                     then solve exp(x) = u for each u.

Public API
----------

    try_solve_transcendental(eq_ir, var) → list[IRNode] | None

``eq_ir`` is either ``Equal(lhs, rhs)`` or a bare expression (= 0).
``var`` is an ``IRSymbol`` for the solve variable.
Returns a list of IR solution nodes, or ``None`` if no pattern matches.
"""

from __future__ import annotations

from fractions import Fraction
from typing import TYPE_CHECKING

from symbolic_ir import (
    ACOS,
    ACOSH,
    ADD,
    ASIN,
    ASINH,
    ATAN,
    ATANH,
    DIV,
    EXP,
    FREE_INTEGER,
    LAMBERT_W,
    LOG,
    MUL,
    NEG,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

if TYPE_CHECKING:
    pass

# ---------------------------------------------------------------------------
# Module-level constants
# ---------------------------------------------------------------------------

# %pi as used throughout the VM.  The same singleton used everywhere.
_PI = IRSymbol("%pi")

# The free-integer constant %k that appears in periodic trig solutions.
# FreeInteger is the IR head; the MACSYMA-facing name is %k.
_K = FREE_INTEGER  # IRSymbol("FreeInteger")


# ---------------------------------------------------------------------------
# Helper: lift a Fraction to the canonical IR literal
# ---------------------------------------------------------------------------


def _frac_ir(c: Fraction) -> IRNode:
    """Convert a ``Fraction`` to its canonical IR literal.

    A Fraction with denominator 1 becomes an ``IRInteger``; others become
    ``IRRational``.  Zero is represented as ``IRInteger(0)``.
    """
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


# ---------------------------------------------------------------------------
# Helper: check if a node is free of (does not mention) a given variable
# ---------------------------------------------------------------------------


def _is_const_wrt(node: IRNode, var: IRSymbol) -> bool:
    """Return ``True`` if ``node`` contains no occurrence of ``var``.

    Walks the tree recursively.  All concrete values (integers, rationals,
    floats, symbols other than ``var``) are trivially constant.  For
    ``IRApply`` nodes every sub-argument is checked.
    """
    if isinstance(node, IRSymbol):
        # Use equality, not identity: IRSymbol is not interned — two separate
        # IRSymbol("x") objects are equal but not the same Python object.
        return node != var
    if isinstance(node, (IRInteger, IRRational, IRFloat)):
        return True
    if isinstance(node, IRApply):
        return all(_is_const_wrt(a, var) for a in node.args)
    return True


# ---------------------------------------------------------------------------
# Helper: extract linear coefficients from an argument
# ---------------------------------------------------------------------------


def _extract_linear(arg: IRNode, var: IRSymbol) -> tuple[Fraction, Fraction] | None:
    """Try to express ``arg`` as ``a·var + b`` with ``a ≠ 0``.

    Uses the polynomial bridge's ``to_rational`` to get exact Fraction
    coefficients.  Returns ``(a, b)`` on success or ``None`` if ``arg`` is
    not a degree-1 polynomial in ``var``.

    Examples::

        _extract_linear(x, x)           →  (1, 0)
        _extract_linear(2*x + 3, x)     →  (2, 3)
        _extract_linear(x/2 - 1, x)     →  (1/2, -1)
        _extract_linear(x^2, x)         →  None   (degree 2)
        _extract_linear(sin(x), x)      →  None   (transcendental)
    """
    # Import here to avoid a circular dependency at module load time.
    # The polynomial bridge lives in symbolic_vm; cas_solve is lower-level,
    # but when called from within the VM context the bridge is available.
    try:
        from symbolic_vm.polynomial_bridge import to_rational
    except ImportError:
        return None

    result = to_rational(arg, var)
    if result is None:
        return None
    num_frac, den_frac = result
    _ONE_FRAC = (Fraction(1),)
    if den_frac != _ONE_FRAC:
        return None  # rational function, not polynomial
    # num_frac is (c0, c1, ..., cn) ascending degree.
    # Degree 1 means exactly two entries: (b, a) where a ≠ 0.
    if len(num_frac) != 2:
        return None  # not linear (could be constant or higher degree)
    b, a = num_frac  # ascending order: c0 is the constant, c1 is coeff of x
    if a == Fraction(0):
        return None  # degenerate
    return (a, b)


# ---------------------------------------------------------------------------
# Helper: solve a·var + b = val_ir  for  var
# ---------------------------------------------------------------------------


def _solve_linear_for_val(
    a: Fraction,
    b: Fraction,
    val_ir: IRNode,
    _var: IRSymbol,
) -> IRNode:
    """Return the IR node for ``var = (val_ir − b) / a``.

    Simplifies the obvious special cases (b = 0, a = 1) to avoid building
    unnecessarily deep trees.
    """
    # Step 1: val_ir − b
    if b == Fraction(0):
        shifted = val_ir
    else:
        b_ir = _frac_ir(b)
        shifted = IRApply(SUB, (val_ir, b_ir))

    # Step 2: divide by a
    if a == Fraction(1):
        return shifted
    a_ir = _frac_ir(a)
    return IRApply(DIV, (shifted, a_ir))


# ---------------------------------------------------------------------------
# Helper: check if a node is Equal(lhs, rhs)
# ---------------------------------------------------------------------------


def _split_equal(eq_ir: IRNode) -> tuple[IRNode, IRNode] | None:
    """Return ``(lhs, rhs)`` if ``eq_ir = Equal(lhs, rhs)``, else ``None``."""
    if (
        isinstance(eq_ir, IRApply)
        and isinstance(eq_ir.head, IRSymbol)
        and eq_ir.head.name == "Equal"
        and len(eq_ir.args) == 2
    ):
        return eq_ir.args[0], eq_ir.args[1]
    return None


# ---------------------------------------------------------------------------
# 26a/26b/26d — simple "f(linear) = constant" pattern
# ---------------------------------------------------------------------------


def _try_func_eq_const(
    func_side: IRNode,
    const_side: IRNode,
    var: IRSymbol,
) -> list[IRNode] | None:
    """Attempt to solve ``func_side = const_side`` when ``func_side`` is a
    recognised transcendental function of a linear expression in ``var``.

    ``const_side`` must be free of ``var``; if it contains ``var`` the
    function returns ``None`` immediately.

    The "two_pi_k" factor used in trig solutions is::

        2 · %pi · %k  = Mul(2, Mul(%pi, FreeInteger))

    which represents  2 · π · k  for any integer k.

    Returns a list of IR solution nodes or ``None``.
    """
    if not _is_const_wrt(const_side, var):
        return None
    if not isinstance(func_side, IRApply):
        return None
    head = func_side.head
    if not isinstance(head, IRSymbol):
        return None
    if len(func_side.args) != 1:
        return None  # multi-argument: not a simple f(u)

    arg = func_side.args[0]
    lin = _extract_linear(arg, var)
    if lin is None:
        return None
    a, b = lin  # arg = a·var + b
    c = const_side

    # 2·π·%k  — used in periodic (trig) solutions.
    two_pi_k = IRApply(MUL, (IRInteger(2), IRApply(MUL, (_PI, _K))))

    fname = head.name

    # ---------------------------------------------------------------
    # Trigonometric — 26a (periodic families)
    # ---------------------------------------------------------------

    if fname == "Sin":
        # sin(u) = c  →  u = arcsin(c) + 2k·π  OR  u = π − arcsin(c) + 2k·π
        asin_c = IRApply(ASIN, (c,))
        arg_val1 = IRApply(ADD, (asin_c, two_pi_k))
        arg_val2 = IRApply(ADD, (IRApply(SUB, (_PI, asin_c)), two_pi_k))
        sol1 = _solve_linear_for_val(a, b, arg_val1, var)
        sol2 = _solve_linear_for_val(a, b, arg_val2, var)
        return [sol1, sol2]

    if fname == "Cos":
        # cos(u) = c  →  u = arccos(c) + 2k·π  OR  u = −arccos(c) + 2k·π
        acos_c = IRApply(ACOS, (c,))
        arg_val1 = IRApply(ADD, (acos_c, two_pi_k))
        arg_val2 = IRApply(ADD, (IRApply(NEG, (acos_c,)), two_pi_k))
        sol1 = _solve_linear_for_val(a, b, arg_val1, var)
        sol2 = _solve_linear_for_val(a, b, arg_val2, var)
        return [sol1, sol2]

    if fname == "Tan":
        # tan(u) = c  →  u = arctan(c) + k·π
        atan_c = IRApply(ATAN, (c,))
        pi_k = IRApply(MUL, (_PI, _K))
        arg_val = IRApply(ADD, (atan_c, pi_k))
        return [_solve_linear_for_val(a, b, arg_val, var)]

    # ---------------------------------------------------------------
    # Exponential / logarithmic — 26b (unique inverse)
    # ---------------------------------------------------------------

    if fname == "Exp":
        # exp(u) = c  →  u = log(c)
        log_c = IRApply(LOG, (c,))
        return [_solve_linear_for_val(a, b, log_c, var)]

    if fname == "Log":
        # log(u) = c  →  u = exp(c)
        exp_c = IRApply(EXP, (c,))
        return [_solve_linear_for_val(a, b, exp_c, var)]

    # ---------------------------------------------------------------
    # Hyperbolic — 26d
    # ---------------------------------------------------------------

    if fname == "Sinh":
        # sinh(u) = c  →  u = asinh(c)  (unique)
        asinh_c = IRApply(ASINH, (c,))
        return [_solve_linear_for_val(a, b, asinh_c, var)]

    if fname == "Cosh":
        # cosh(u) = c  →  u = acosh(c)  OR  u = −acosh(c)
        acosh_c = IRApply(ACOSH, (c,))
        sol1 = _solve_linear_for_val(a, b, acosh_c, var)
        sol2 = _solve_linear_for_val(a, b, IRApply(NEG, (acosh_c,)), var)
        return [sol1, sol2]

    if fname == "Tanh":
        # tanh(u) = c  →  u = atanh(c)  (unique)
        atanh_c = IRApply(ATANH, (c,))
        return [_solve_linear_for_val(a, b, atanh_c, var)]

    return None  # unrecognised transcendental head


# ---------------------------------------------------------------------------
# 26c — Lambert W: f(x)·exp(f(x)) = c
# ---------------------------------------------------------------------------


def _try_lambert(lhs: IRNode, rhs: IRNode, var: IRSymbol) -> list[IRNode] | None:
    """Detect the Lambert-W pattern  f(var) · exp(f(var)) = c  and return
    ``[var = (W(c) − b) / a]`` where ``f(var) = a·var + b``.

    The pattern we look for on the *lhs*:
      ``MUL(u, Exp(u))``  where ``u = a·var + b`` (linear in var)

    and *rhs* must be constant w.r.t. var.

    Also handles the *rhs-form*: ``c = f(var)·exp(f(var))``.
    """
    return _lambert_one_side(lhs, rhs, var) or _lambert_one_side(rhs, lhs, var)


def _lambert_one_side(
    product_side: IRNode,
    const_side: IRNode,
    var: IRSymbol,
) -> list[IRNode] | None:
    """Try to match  ``product_side = f(var) · exp(f(var))``  with ``const_side``
    free of ``var``."""
    if not _is_const_wrt(const_side, var):
        return None
    if not isinstance(product_side, IRApply):
        return None
    if product_side.head is not MUL:
        return None
    if len(product_side.args) != 2:
        return None

    a_arg, b_arg = product_side.args

    # Either of (a_arg, b_arg) could be the Exp(...) factor.
    for linear_node, exp_node in ((a_arg, b_arg), (b_arg, a_arg)):
        if not (
            isinstance(exp_node, IRApply)
            and exp_node.head is EXP
            and len(exp_node.args) == 1
        ):
            continue
        inner = exp_node.args[0]
        # inner and linear_node must both be the same linear expression.
        # We check by extracting linear coefficients from both.
        lin_a = _extract_linear(linear_node, var)
        lin_b = _extract_linear(inner, var)
        if lin_a is None or lin_b is None:
            continue
        if lin_a != lin_b:
            continue
        # Match!  f(var) = a·var + b → W(const_side) = a·var + b
        a, b = lin_a
        w_c = IRApply(LAMBERT_W, (const_side,))
        return [_solve_linear_for_val(a, b, w_c, var)]

    return None


# ---------------------------------------------------------------------------
# 26e — Compound: polynomial in a single transcendental substitution
# ---------------------------------------------------------------------------

# Heads we try as substitution target.
_COMPOUND_HEADS = ("Sin", "Cos", "Tan", "Sinh", "Cosh", "Tanh", "Exp", "Log")


def _substitute_func(
    expr: IRNode,
    func_name: str,
    var: IRSymbol,
    sub: IRSymbol,
) -> IRNode | None:
    """Replace every occurrence of ``FuncName(var)`` with ``sub`` inside
    ``expr``.  Return ``None`` if after substitution ``var`` still appears
    (i.e. it appeared in a different context that we cannot handle here).

    We recognise  ``FuncName(var)``  only when the argument is exactly
    ``var`` — not ``var + offset`` or ``2*var``.  This keeps the compound
    pattern simple; the linear-argument forms are handled by
    ``_try_func_eq_const``.
    """
    result = _subst_walk(expr, func_name, var, sub)
    if result is None:
        return None
    # After substitution, var must not appear.
    if not _is_const_wrt(result, var):
        return None
    return result


def _subst_walk(
    node: IRNode,
    func_name: str,
    var: IRSymbol,
    sub: IRSymbol,
) -> IRNode | None:
    """Recursive walker for ``_substitute_func``."""
    # The variable itself — appears outside any function wrapper.  We
    # cannot substitute it; return ``None`` to signal failure.
    # Use equality (not identity) since IRSymbol is not interned.
    if isinstance(node, IRSymbol) and node == var:
        return None
    if isinstance(node, (IRSymbol, IRInteger, IRRational, IRFloat)):
        return node
    if not isinstance(node, IRApply):
        return None

    head = node.head
    if not isinstance(head, IRSymbol):
        return None

    # Match exactly  FuncName(var).
    # Use equality (not identity) since IRSymbol is not interned.
    if (
        head.name == func_name
        and len(node.args) == 1
        and isinstance(node.args[0], IRSymbol)
        and node.args[0] == var
    ):
        return sub

    # Recurse into all arguments.
    new_args: list[IRNode] = []
    for arg in node.args:
        walked = _subst_walk(arg, func_name, var, sub)
        if walked is None:
            return None
        new_args.append(walked)
    return IRApply(head, tuple(new_args))


def _try_compound(lhs: IRNode, rhs: IRNode, var: IRSymbol) -> list[IRNode] | None:
    """Try to solve by substituting  u = f(var)  and reducing to a
    polynomial in u.

    Only considers substitutions of the form ``f(var)`` — not linear
    offsets like ``f(2x + 1)``.  The linear-offset case is already
    handled by ``_try_func_eq_const``.

    Algorithm:
    1. Compute ``diff = lhs − rhs`` (the equation normalised to = 0).
    2. For each candidate function name f in ``_COMPOUND_HEADS``:
       a. Attempt to replace every ``f(var)`` with a fresh symbol ``_u``.
       b. Check that ``_u`` actually appears (otherwise no substitution occurred).
       c. Check that the result is a polynomial in ``_u`` over Q.
       d. Solve that polynomial (degree 1–4) for ``_u``.
       e. For each root u_sol: recursively call ``try_solve_transcendental``
          on  ``f(var) = u_sol``  to invert the outer function.
       f. Collect all resulting x-solutions.
    """
    # Build the normalised equation expr = 0.
    if isinstance(rhs, IRInteger) and rhs.value == 0:
        diff = lhs
    else:
        diff = IRApply(SUB, (lhs, rhs))

    for func_name in _COMPOUND_HEADS:
        sub_sym = IRSymbol(f"__sub_{func_name}__")
        substituted = _substitute_func(diff, func_name, var, sub_sym)
        if substituted is None:
            continue

        # Only proceed if sub_sym actually appears in substituted.
        if _is_const_wrt(substituted, sub_sym):
            continue

        # Check polynomial structure in sub_sym.
        coeffs = _poly_coeffs(substituted, sub_sym)
        if coeffs is None:
            continue
        deg = len(coeffs) - 1
        if deg < 1:
            continue

        # Solve the polynomial for sub_sym.
        sub_solutions = _solve_poly(coeffs)
        if sub_solutions is None:
            continue

        # For each sub-solution, invert f(var) = u_sol.
        all_x: list[IRNode] = []
        _EQ = IRSymbol("Equal")
        for u_sol in sub_solutions:
            fn_of_var = IRApply(IRSymbol(func_name), (var,))
            func_eq = IRApply(_EQ, (fn_of_var, u_sol))
            inner = try_solve_transcendental(func_eq, var)
            if inner is not None:
                all_x.extend(inner)
        if all_x:
            return all_x

    return None


def _poly_coeffs(expr: IRNode, sym: IRSymbol) -> tuple[Fraction, ...] | None:
    """Extract rational polynomial coefficients of ``expr`` in ``sym``."""
    try:
        from symbolic_vm.polynomial_bridge import to_rational
    except ImportError:
        return None
    result = to_rational(expr, sym)
    if result is None:
        return None
    num_frac, den_frac = result
    if den_frac != (Fraction(1),):
        return None
    return num_frac


def _solve_poly(coeffs: tuple[Fraction, ...]) -> list[IRNode] | None:
    """Solve the polynomial with the given ascending-degree Fraction
    coefficients.  Delegates to the existing ``cas_solve`` solvers for
    degrees 1–4.
    """
    from cas_solve.linear import ALL as _ALL  # noqa: PLC0415
    from cas_solve.linear import solve_linear  # noqa: PLC0415

    deg = len(coeffs) - 1
    if deg < 1:
        return None

    if deg == 1:
        # b + a·u = 0
        a, b = coeffs[1], coeffs[0]
        sol = solve_linear(a, b)
        if sol is _ALL:
            return None
        return list(sol) if sol else []

    if deg == 2:
        from cas_solve.quadratic import solve_quadratic

        c, b, a = coeffs[0], coeffs[1], coeffs[2]
        sols = solve_quadratic(a, b, c)
        return list(sols)

    if deg == 3:
        from cas_solve.cubic import solve_cubic

        d, c, b, a = coeffs[0], coeffs[1], coeffs[2], coeffs[3]
        sols = solve_cubic(a, b, c, d)
        if not sols or isinstance(sols, str):
            return None
        return list(sols)

    if deg == 4:
        from cas_solve.quartic import solve_quartic

        e, d, c, b, a = coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]
        sols = solve_quartic(a, b, c, d, e)
        if not sols or isinstance(sols, str):
            return None
        return list(sols)

    return None  # degree > 4: fall through


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def try_solve_transcendental(
    eq_ir: IRNode,
    var: IRSymbol,
) -> list[IRNode] | None:
    """Try to solve ``eq_ir`` for ``var`` using transcendental techniques.

    ``eq_ir`` is either:
    - ``Equal(lhs, rhs)`` — an explicit equation
    - A bare IR expression (treated as  expr = 0)

    Returns a list of IR nodes representing the solutions, or ``None`` if
    none of the recognised patterns match.  The caller wraps the list in
    ``List(...)`` for the MACSYMA surface output.

    Dispatch order:
    1. Simple  f(linear) = constant  — 26a/26b/26d
    2. Lambert W  f·exp(f) = constant  — 26c
    3. Compound polynomial-in-transcendental substitution  — 26e
    """
    # Normalise equation into (lhs, rhs) form.
    split = _split_equal(eq_ir)
    if split is not None:
        lhs, rhs = split
    else:
        lhs = eq_ir
        rhs = IRInteger(0)

    # ------------------------------------------------------------------
    # Pass 1: simple  f(linear) = constant
    # ------------------------------------------------------------------
    result = _try_func_eq_const(lhs, rhs, var)
    if result is not None:
        return result

    result = _try_func_eq_const(rhs, lhs, var)
    if result is not None:
        return result

    # ------------------------------------------------------------------
    # Pass 2: Lambert W
    # ------------------------------------------------------------------
    result = _try_lambert(lhs, rhs, var)
    if result is not None:
        return result

    # ------------------------------------------------------------------
    # Pass 3: compound (polynomial in transcendental substitution)
    # ------------------------------------------------------------------
    result = _try_compound(lhs, rhs, var)
    if result is not None:
        return result

    return None
