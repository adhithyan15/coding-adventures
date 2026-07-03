"""Lie point-symmetry handler for first-order ODEs — Track L1.

A *point symmetry* of a first-order ODE ``y' = f(x, y)`` is a
one-parameter group of transformations ``(x, y) → (X(x, y; ε), Y(x, y; ε))``
that maps solutions to solutions.  When such a symmetry exists, the ODE
can be reduced to a quadrature via a similarity variable that is
invariant under the group action.  This module implements detection and
reduction for three textbook point-symmetry groups that are simple to
test by *numerical invariance*:

1. **Scaling**:   ``(x, y) → (λ·x, λ^k·y)``  for integer ``k ∈ [-3, 3]``.
   Reduce via the similarity variable ``v = y / x^k`` (turning the ODE
   into a separable equation in ``(v, x)``).

2. **Translation in x**:  ``(x, y) → (x + c, y)``.  The ODE is
   *autonomous* — ``f`` has no explicit ``x`` dependence.  Reduce via
   the quadrature ``x = ∫ 1/f(y) dy + C``.

3. **Translation in y**:  ``(x, y) → (x, y + c)``.  The ODE depends only
   on ``x`` — ``f`` has no explicit ``y`` dependence.  Reduce via the
   quadrature ``y = ∫ f(x) dx + C``.

These cases are *deliberately overlap-free with the rest of the
dispatcher*: linear, separable, Bernoulli, exact and homogeneous-type
ODEs will already be caught by the earlier handlers.  The Lie path
fires only on what falls through — autonomous nonlinear ``y' = g(y)``
(translation-in-x) is the canonical example.

Detection strategy
------------------
Rather than computing the symbolic linearised determining equations
(which would require an algebraic-equation solver we don't have), we
test invariance *numerically*.  For each candidate group, we substitute
the transformation into ``f`` and check that ``f(X, Y, Y') == Y' /
X'`` at several sample points.  The number of test points (3 sample
points × 3 sample λ values for scaling) is enough to rule out spurious
coincidences while keeping the runtime trivial.

All iteration is bounded:

- Scaling exponent ``k`` is searched only in ``[-3, 3]`` — a hard
  range with no escape hatch.
- Test points are fixed constants.
- No recursion; no user-string evaluation.

Literate reading guide
----------------------
1.  :func:`_extract_f`              — normalise ``expr = 0`` to ``f(x, y)``
2.  :func:`_eval_f`                 — wrap ``_eval_at_xy`` from ``ode.py``
3.  :func:`_is_x_autonomous`         — does ``f`` depend on ``x``?
4.  :func:`_is_y_autonomous`         — does ``f`` depend on ``y``?
5.  :func:`_detect_scaling_k`       — bounded search for the scaling exponent
6.  :func:`_reduce_translation_x`   — autonomous → ``x = ∫1/f dy + C``
7.  :func:`_reduce_translation_y`   — pure ``y' = f(x)`` → direct integration
8.  :func:`_reduce_scaling`         — similarity ``v = y/x^k`` substitution
9.  :func:`try_lie_symmetry`        — public entry point
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import (
    ADD,
    DIV,
    EQUAL,
    INTEGRATE,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)
from symbolic_ir.nodes import C_CONST, D

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


# ---------------------------------------------------------------------------
# Convenience constructors — keep the algorithm body readable.
# ---------------------------------------------------------------------------

_ZERO = IRInteger(0)
_ONE = IRInteger(1)


def _add(a: IRNode, b: IRNode) -> IRNode:
    if isinstance(a, IRInteger) and a.value == 0:
        return b
    if isinstance(b, IRInteger) and b.value == 0:
        return a
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    if isinstance(a, IRInteger) and a.value == 1:
        return b
    if isinstance(b, IRInteger) and b.value == 1:
        return a
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


# ---------------------------------------------------------------------------
# Section 1 — Normalise the ODE to ``y' = f(x, y)`` form
# ---------------------------------------------------------------------------
#
# The dispatcher hands us ``expr`` in zero form: ``y' - f(x, y) = 0``.  We
# locate the bare ``D(y, x)`` term, move it to the LHS, and treat the
# remaining sum (negated) as ``f(x, y)``.  Anything more exotic (a
# coefficient on y' that depends on x or y) is rejected — the linear
# handler would have caught those.


def _extract_f(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Return ``f(x, y)`` such that the ODE is ``y' = f(x, y)``.

    Returns ``None`` if the ODE is not in the form ``y' + (anything not
    involving y') = 0`` — i.e. if there is no bare ``D(y, x)`` term or if
    a y'-coefficient other than 1 appears.
    """
    # Lazy import to avoid a circular dependency at module load time.
    from cas_ode.ode import _flatten_add, _unwrap_neg

    y_prime = IRApply(D, (y, x))

    found_yprime = False
    rest: list[IRNode] = []
    for term in _flatten_add(expr):
        neg, core = _unwrap_neg(term)
        if core == y_prime:
            if neg:
                # `-y'` appears as a top-level summand — non-standard.
                return None
            found_yprime = True
            continue
        rest.append(term)

    if not found_yprime:
        return None

    # f = -(sum of remaining terms)        (because y' - f = 0 ⇒ f = -rest)
    if not rest:
        f: IRNode = _ZERO
    else:
        acc = rest[0]
        for t in rest[1:]:
            acc = _add(acc, t)
        f = _neg(acc)

    return vm.eval(f)


# ---------------------------------------------------------------------------
# Section 2 — Numerical evaluation helper
# ---------------------------------------------------------------------------
#
# We re-use the existing ``_eval_at_xy`` from ``ode.py`` (already proven by
# the exact-ODE test path).  It supports Add / Sub / Mul / Div / Neg / Pow /
# Exp / Log / Sin / Cos — exactly the operators that appear in textbook
# first-order ODEs.  Anything else (a custom function head, etc.) makes the
# evaluator raise; we treat that as "cannot test, give up".


def _eval_f(
    f: IRNode,
    x: IRSymbol,
    y: IRSymbol,
    xv: float,
    yv: float,
) -> float | None:
    """Evaluate ``f`` at ``(x, y) = (xv, yv)``; return ``None`` on failure."""
    from cas_ode.ode import _eval_at_xy

    try:
        return _eval_at_xy(f, x, y, xv, yv)
    except (ValueError, ZeroDivisionError, OverflowError, TypeError):
        return None


# ---------------------------------------------------------------------------
# Section 3 — Autonomy checks
# ---------------------------------------------------------------------------
#
# An ODE ``y' = f(x, y)`` is:
#
#   * **x-autonomous** (translation-in-x symmetric) iff ``∂f/∂x = 0``.
#     We test this numerically: ``f(x₁, y) == f(x₂, y)`` for several y at
#     two distinct x values.  Cheaper than symbolic differentiation and
#     robust to whatever simplification the VM has applied.
#
#   * **y-autonomous** (translation-in-y symmetric) iff ``∂f/∂y = 0``.


_AUTONOMY_TEST_PTS_XY: tuple[tuple[float, float, float], ...] = (
    # (y_or_x_value, t1, t2) — varying the first slot must not change f.
    (0.7, 1.1, 2.3),
    (1.3, 0.4, 1.9),
    (2.1, 0.9, 3.0),
)


def _is_x_autonomous(
    f: IRNode,
    x: IRSymbol,
    y: IRSymbol,
    tol: float = 1e-9,
) -> bool:
    """Return ``True`` if ``f(x₁, y) ≈ f(x₂, y)`` for several test y values.

    A passing result means ``f`` does not depend on ``x`` — the ODE is
    autonomous and admits the translation symmetry ``(x, y) → (x+c, y)``.
    """
    for yv, x1, x2 in _AUTONOMY_TEST_PTS_XY:
        v1 = _eval_f(f, x, y, x1, yv)
        v2 = _eval_f(f, x, y, x2, yv)
        if v1 is None or v2 is None:
            return False
        if abs(v1 - v2) > tol:
            return False
    return True


def _is_y_autonomous(
    f: IRNode,
    x: IRSymbol,
    y: IRSymbol,
    tol: float = 1e-9,
) -> bool:
    """Return ``True`` if ``f(x, y₁) ≈ f(x, y₂)`` for several test x values.

    A passing result means ``f`` does not depend on ``y`` — the ODE has
    the form ``y' = f(x)`` and admits the translation symmetry
    ``(x, y) → (x, y+c)``.
    """
    for xv, y1, y2 in _AUTONOMY_TEST_PTS_XY:
        v1 = _eval_f(f, x, y, xv, y1)
        v2 = _eval_f(f, x, y, xv, y2)
        if v1 is None or v2 is None:
            return False
        if abs(v1 - v2) > tol:
            return False
    return True


# ---------------------------------------------------------------------------
# Section 4 — Scaling-symmetry detection
# ---------------------------------------------------------------------------
#
# Under ``(x, y) → (λ·x, λ^k·y)``, the derivative transforms as
# ``y' → λ^(k-1)·y'``.  For ``y' = f(x, y)`` to be invariant we need
#
#     f(λx, λ^k y) = λ^(k-1) · f(x, y).
#
# We test this at a handful of sample λ and (x, y) — three each.  If the
# residual is within the tolerance for *all* test points and a given k, we
# accept that exponent.  The search space is the hard-bounded range
# ``k ∈ [-3, 3]`` (seven candidates).  k = 0 is skipped because it
# corresponds to translation in x and we test that separately.

_SCALING_LAMBDAS: tuple[float, ...] = (2.0, 3.0, 0.5)
_SCALING_POINTS: tuple[tuple[float, float], ...] = (
    (1.0, 1.0),
    (2.0, 3.0),
    (1.0, 2.0),
)
_SCALING_K_RANGE: tuple[int, ...] = (-3, -2, -1, 1, 2, 3)
_SCALING_TOL = 1e-7   # transcendental cases (sin, exp) — keep loose


def _detect_scaling_k(
    f: IRNode,
    x: IRSymbol,
    y: IRSymbol,
) -> int | None:
    """Find an integer ``k ∈ [-3, 3] \\ {0}`` such that
    ``f(λx, λ^k y) = λ^(k-1) · f(x, y)`` at all sample points.

    Returns the first matching ``k`` (preferring smaller ``|k|``), or
    ``None`` if no candidate fits.  The iteration is bounded: at most
    ``7 candidates × 3 λ × 3 (x, y) = 63`` evaluations.
    """
    # Order: try positive exponents first (1, 2, 3, -1, -2, -3) so we
    # prefer simple ``v = y/x`` substitutions.
    ordered = (1, 2, 3, -1, -2, -3)
    for k in ordered:
        if k not in _SCALING_K_RANGE:  # defensive; will never trigger
            continue
        all_ok = True
        for lam in _SCALING_LAMBDAS:
            for xv, yv in _SCALING_POINTS:
                lhs = _eval_f(f, x, y, lam * xv, (lam**k) * yv)
                base = _eval_f(f, x, y, xv, yv)
                if lhs is None or base is None:
                    all_ok = False
                    break
                expected = (lam ** (k - 1)) * base
                if abs(lhs - expected) > _SCALING_TOL * max(1.0, abs(expected)):
                    all_ok = False
                    break
            if not all_ok:
                break
        if all_ok:
            return k
    return None


# ---------------------------------------------------------------------------
# Section 5 — Reductions
# ---------------------------------------------------------------------------
#
# Each reduction returns an ``Equal(...)`` IR node — either explicit
# ``Equal(y, expr)`` or an implicit relation between ``x`` and ``y``.  All
# integration goes through the VM so we stay consistent with the rest of
# the package.


def _reduce_translation_y(
    f: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Solve ``y' = f(x)`` by direct integration: ``y = ∫ f(x) dx + C``."""
    from cas_ode.ode import _is_unevaluated_integrate

    int_f = vm.eval(IRApply(INTEGRATE, (f, x)))
    if _is_unevaluated_integrate(int_f, x):
        return None
    rhs = vm.eval(_add(int_f, C_CONST))
    return IRApply(EQUAL, (y, rhs))


def _reduce_translation_x(
    f: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Solve autonomous ``y' = g(y)`` via the quadrature
    ``x = ∫ 1/g(y) dy + C``.

    This produces the inverse relation between x and y — exactly the
    form a CAS user expects for autonomous ODEs that the separable
    handler couldn't invert (e.g. logistic ``y' = y(1-y)``).
    """
    from cas_ode.ode import _is_unevaluated_integrate

    # Guard against the f = 0 case (trivial: y = C — caught by separable).
    if isinstance(f, IRInteger) and f.value == 0:
        return None

    inv = vm.eval(_div(_ONE, f))
    int_inv = vm.eval(IRApply(INTEGRATE, (inv, y)))
    if _is_unevaluated_integrate(int_inv, y):
        return None
    rhs = vm.eval(_add(int_inv, C_CONST))
    return IRApply(EQUAL, (x, rhs))


_CERT_SAMPLE_X: tuple[float, ...] = (1.5, 2.5, 0.4)
_CERT_SAMPLE_V: tuple[float, ...] = (0.7, 1.3, 2.1)


def _verify_scaling_certificate(
    f_subst: IRNode,
    G_raw: IRNode,
    k: int,
    x: IRSymbol,
    v: IRSymbol,
    tol: float = 1e-6,
) -> bool:
    """Confirm ``f_subst(x, v) / x^(k-1) == G_raw(v)`` at sample points.

    This is the numerical safety net for the scaling reduction.  When
    the algebra is sound, ``f(x, v·x^k)`` already factors as
    ``x^(k-1) · G(v)``; we obtained ``G_raw`` by substituting ``x = 1``
    in ``f_subst``.  At any other ``x``, the identity
    ``f_subst(x, v) = x^(k-1) · G_raw(v)`` must hold.  If it does not,
    something has gone wrong (e.g. the scaling-detection picked the
    wrong ``k``) and we must bail.
    """
    for xv in _CERT_SAMPLE_X:
        for vv in _CERT_SAMPLE_V:
            lhs = _eval_f(f_subst, x, v, xv, vv)
            g = _eval_f(G_raw, x, v, xv, vv)  # G is x-free, xv ignored
            if lhs is None or g is None:
                return False
            expected = (xv ** (k - 1)) * g
            scale = max(1.0, abs(expected))
            if abs(lhs - expected) > tol * scale:
                return False
    return True


def _reduce_scaling(
    f: IRNode,
    k: int,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Reduce ``y' = f(x, y)`` with scaling symmetry exponent ``k``.

    The similarity variable is ``v = y / x^k``.  Setting ``y = v · x^k``
    and ``y' = v'·x^k + k·v·x^(k-1)`` transforms the ODE into

        v' · x^k = f(x, v·x^k) - k·v·x^(k-1).

    Because of the scaling invariance, the right-hand side factors as
    ``x^(k-1) · [F(v) - k·v]`` for some function ``F`` of v alone, giving
    the separable equation ``x · v' = F(v) - k·v``.

    Rather than reconstruct ``F`` symbolically (which requires algebraic
    simplification we don't have at this layer), we **return None** and let
    the homogeneous-type / separable handlers earlier in the dispatch
    chain catch the equation — they almost always do for k = ±1 (the
    homogeneous-type substitution ``v = y/x`` is the k=1 case).

    Algorithm (mirrors ``_try_homogeneous_type`` but uses a *direct*
    structural substitution rather than ratio detection):

    1. Build the IR for ``f_subst = f(x, v · x^k)`` via :func:`_subst_ir`.
    2. Divide by ``x^(k-1)`` to extract ``G(v) = f_subst / x^(k-1)``.
       The VM's simplifier must collapse the ``x`` dependence; if it
       doesn't, we conservatively bail.
    3. The separable ODE ``x·v' = G(v) − k·v`` reduces to
       ``∫ dv / (G(v) − k·v) = log(x) + C``.
    4. Back-substitute ``v → y / x^k`` and return the implicit relation.

    The degenerate case ``G(v) = k·v`` (i.e. ``f = k·y/x``) gives
    ``v = const``, so ``y = C · x^k``.

    Returns ``None`` when integration fails or the scaling certificate
    cannot be verified (preserves the "no false closures" invariant).
    """
    from symbolic_ir import LOG

    from cas_ode.ode import _is_const_wrt, _is_unevaluated_integrate, _subst_ir

    if k == 0:
        # k = 0 means f does not depend on y at all — already handled by
        # the translation-in-y branch upstream.  Skip to avoid double work.
        return None

    # Build the IR for x^k (x^(k-1) is no longer needed — we extract G via
    # the certificate point x = 1).
    v = IRSymbol("_lie_v")
    if k == 1:
        x_to_k: IRNode = x
    elif k > 1:
        x_to_k = _pow(x, IRInteger(k))
    else:
        # k <= -1: y = v / x^|k|.
        x_to_k = _div(_ONE, _pow(x, IRInteger(-k)))

    # Step 1: f(x, v · x^k).
    f_subst = vm.eval(_subst_ir(f, y, _mul(v, x_to_k)))

    # Step 2: extract G(v) = f_subst / x^(k-1).
    #
    # The VM's algebraic simplifier may not collapse the x dependence
    # symbolically — even when the scaling-invariance algebra guarantees
    # it must cancel.  We therefore evaluate at the *certificate point*
    # ``x = 1``: at that point, ``x^(k-1) = 1`` and ``f_subst`` reduces
    # to a pure function of ``v``.  We then verify *numerically* that
    # the resulting G is x-independent (by checking f_subst / x^(k-1) at
    # several other x values agrees with the x = 1 value at sample v).
    G_raw = vm.eval(_subst_ir(f_subst, x, _ONE))

    # The scaling certificate must hold both ways:
    # (a) G_raw must be free of x  (structural — required for integration).
    # (b) f_subst / x^(k-1) at non-unit x must agree with G_raw at sample v
    #     (numerical — confirms our extracted G genuinely represents the
    #     scaling-reduced form).
    if not _is_const_wrt(G_raw, x):
        return None

    # Numerical certificate: probe at three (x, v) sample points.
    if not _verify_scaling_certificate(f_subst, G_raw, k, x, v):
        return None

    # Build the separable denominator and check for the degenerate case.
    denom = vm.eval(_sub(G_raw, _mul(IRInteger(k), v)))
    if isinstance(denom, IRInteger) and denom.value == 0:
        # G(v) = k·v  →  v = constant  →  y = C·x^k.
        return IRApply(EQUAL, (y, _mul(C_CONST, x_to_k)))

    # Step 3: integrate 1 / (G(v) − k·v) with respect to v.
    integrand = vm.eval(_div(_ONE, denom))
    H_v = vm.eval(IRApply(INTEGRATE, (integrand, v)))
    if _is_unevaluated_integrate(H_v, v):
        return None

    # Step 4: back-substitute v → y / x^k and build the RHS log(x) + C.
    H_yxk = vm.eval(_subst_ir(H_v, v, _div(y, x_to_k)))

    log_x = vm.eval(IRApply(INTEGRATE, (_div(_ONE, x), x)))
    if _is_unevaluated_integrate(log_x, x):
        log_x = IRApply(LOG, (x,))

    rhs = vm.eval(_add(log_x, C_CONST))
    return IRApply(EQUAL, (H_yxk, rhs))


# ---------------------------------------------------------------------------
# Section 6 — Public entry point
# ---------------------------------------------------------------------------


def try_lie_symmetry(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    vm: VM,
) -> IRNode | None:
    """Detect a Lie point-symmetry of ``expr = 0`` and reduce.

    Returns an ``Equal(...)`` IR node (closed-form solution, possibly
    implicit) when one of the three handled symmetries fires, or ``None``
    to signal fall-through.

    Dispatch order (within Lie):

    1. Translation in y — cheapest test; produces explicit ``y = ∫ f dx + C``.
    2. Translation in x — autonomous case; produces implicit
       ``x = ∫ 1/g dy + C``.
    3. Scaling — detection only (see :func:`_reduce_scaling`).

    Parameters
    ----------
    expr:
        ODE in zero form: ``y' - f(x, y) = 0``.
    y, x:
        Dependent and independent variable symbols.
    vm:
        Live symbolic VM — used for integration.
    """
    f = _extract_f(expr, y, x, vm)
    if f is None:
        return None

    # ---- 1. Translation in y: f has no explicit y dependence ---------------
    if _is_y_autonomous(f, x, y):
        sol = _reduce_translation_y(f, y, x, vm)
        if sol is not None:
            return sol

    # ---- 2. Translation in x: f has no explicit x dependence ---------------
    if _is_x_autonomous(f, x, y):
        sol = _reduce_translation_x(f, y, x, vm)
        if sol is not None:
            return sol

    # ---- 3. Scaling symmetry: detect-only --------------------------------
    k = _detect_scaling_k(f, x, y)
    if k is not None:
        sol = _reduce_scaling(f, k, y, x, vm)
        if sol is not None:
            return sol

    return None
