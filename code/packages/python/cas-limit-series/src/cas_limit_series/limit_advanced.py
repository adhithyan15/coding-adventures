"""Enhanced limit computation: L'Hôpital, infinity, and indeterminate forms.

Phase 20 extends the trivial direct-substitution ``limit_direct`` with
a full limit evaluator supporting:

1. **Direct substitution** — always tried first.  If the expression is
   continuous at the point the substituted value is returned immediately.

2. **L'Hôpital's rule** — for ``0/0`` and ``∞/∞`` indeterminate ratios.
   Differentiates the numerator and denominator repeatedly (up to depth 8)
   and retries.

3. **Limits at ±∞** — ``IRSymbol("inf")`` / ``IRSymbol("minf")`` as the
   limit point.  The numeric evaluator maps ``inf → math.inf`` and uses
   Python's IEEE 754 arithmetic so that ``exp(−∞) = 0``, ``1/∞ = 0``, etc.

4. **All standard indeterminate forms**:

   ============  ========================================================
   Form          Reduction
   ============  ========================================================
   ``0/0``       L'Hôpital on the ``DIV`` node
   ``∞/∞``       L'Hôpital on the ``DIV`` node
   ``0·∞``       Rewrite ``MUL(a,b)`` → ``DIV(b, DIV(1,a))`` then
                 L'Hôpital
   ``1^∞``       ``EXP(MUL(e, LOG(b)))`` then recurse on the exponent
   ``0^0``       Same exp-log transform
   ``∞^0``       Same exp-log transform
   ============  ========================================================

5. **One-sided limits** — ``direction="plus"`` (from the right) or
   ``direction="minus"`` (from the left).  A tiny numeric perturbation
   (±1×10⁻³⁰⁰) is applied to the evaluation point to classify the form;
   the symbolic substitution still uses the exact ``point`` value.

Architecture note
-----------------
This module does **not** depend on ``symbolic-vm``.  Differentiation and
VM simplification are injected as callables at call time:

- ``diff_fn(expr, var)`` — returns the simplified derivative of ``expr``
  with respect to ``var``.  Typically
  ``lambda e, v: vm.eval(_symbolic_diff(e, v))``.
- ``eval_fn(node)`` — runs the node through the VM evaluator to collapse
  arithmetic.  Typically ``vm.eval``.

If ``diff_fn`` is ``None``, L'Hôpital falls through and the unevaluated
``Limit(…)`` node is returned instead.

Literate reading order
-----------------------
1. ``_num_eval``          — numeric float evaluator for form detection
2. ``_classify_at``       — classify expression value at a point
3. ``_lhopital_step``     — one round of L'Hôpital differentiation
4. ``_zero_inf_rewrite``  — 0·∞ → 0/0 rewrite
5. ``_pow_exp_log``       — 1^∞ / 0^0 / ∞^0 → exp-log rewrite
6. ``_handle_form``       — dispatcher for all indeterminate forms
7. ``limit_advanced``     — public entry point
"""

from __future__ import annotations

import math
from collections.abc import Callable

from cas_substitution import subst
from symbolic_ir import (
    ADD,
    ATAN,
    COS,
    COSH,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_limit_series.heads import LIMIT

# ---------------------------------------------------------------------------
# Type alias for injected callables
# ---------------------------------------------------------------------------

DiffFn = Callable[[IRNode, IRSymbol], IRNode]
EvalFn = Callable[[IRNode], IRNode]

# ---------------------------------------------------------------------------
# Special-symbol constants
# ---------------------------------------------------------------------------

#: Positive infinity symbol (same as MACSYMA's ``inf``).
INF_SYM = IRSymbol("inf")
#: Negative infinity symbol (same as MACSYMA's ``minf``).
MINF_SYM = IRSymbol("minf")
#: ``%pi``
_PI_SYM = IRSymbol("%pi")
#: ``%e``
_E_SYM = IRSymbol("%e")

#: Small positive perturbation used for one-sided directional detection.
#: 1e-300 is sub-normal but well-defined; 1/1e-300 = 1e300 which is large
#: but representable.  We compare against _INF_THRESHOLD to catch it.
_EPS = 1e-300

#: Values with absolute magnitude above this threshold are treated as ±∞.
#: 1e100 safely captures 1/ε and exp of large arguments while staying far
#: below ``sys.float_info.max`` (~1.8e308).
_INF_THRESHOLD: float = 1e100

# Maximum L'Hôpital recursion depth.
_MAX_DEPTH = 8


# ---------------------------------------------------------------------------
# Section 1 — Numeric evaluator
# ---------------------------------------------------------------------------
#
# ``_num_eval`` converts an IR node to a Python ``float``.  It handles
# ±∞ via ``math.inf`` and returns ``float('nan')`` for truly indeterminate
# sub-expressions (``0/0`` in particular).  All symbolic variables that are
# not recognised constants map to NaN as well.
#
# This is used exclusively for *classifying* a limit form — never for
# computing the final symbolic answer.


def _ev(node: IRNode) -> float:  # noqa: PLR0911, PLR0912 (many branches intentional)
    """Recursively evaluate an IR node to a float.

    ``IRSymbol("inf") → math.inf``, ``IRSymbol("minf") → -math.inf``.
    ``0/0 → nan``.  Unknown heads / symbols → nan.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        name = node.name
        if name == "inf":
            return math.inf
        if name == "minf":
            return -math.inf
        if name == "%pi":
            return math.pi
        if name == "%e":
            return math.e
        return float("nan")
    if not isinstance(node, IRApply):
        return float("nan")

    h = node.head
    args = node.args

    # Arithmetic
    if h == ADD:
        return sum(_ev(a) for a in args)
    if h == SUB:
        return _ev(args[0]) - _ev(args[1])
    if h == MUL:
        result = 1.0
        for a in args:
            result *= _ev(a)
        return result
    if h == DIV:
        numer = _ev(args[0])
        denom = _ev(args[1])
        if denom == 0.0:
            # 0/0 → NaN (indeterminate); nonzero/0 → ±∞
            if numer == 0.0:
                return float("nan")
            return math.copysign(math.inf, numer)
        return numer / denom
    if h == NEG:
        return -_ev(args[0])
    if h == POW:
        base = _ev(args[0])
        exp_val = _ev(args[1])
        # Indeterminate power forms — return NaN so _handle_form dispatches
        # to the exp-log rewrite (Section 5) rather than collapsing prematurely
        # with float arithmetic.
        #   1^∞  : e.g. (1+1/x)^x  → e
        #   0^0  : e.g. x^x at 0+  → 1
        #   ∞^0  : e.g. exp(x)^(1/x) at ∞ → e
        if abs(base - 1.0) < 1e-10 and math.isinf(exp_val):
            return float("nan")  # 1^∞
        if base == 0.0 and exp_val == 0.0:
            return float("nan")  # 0^0
        if math.isinf(base) and exp_val == 0.0:
            return float("nan")  # ∞^0
        # Guard against complex result from negative base with fractional exp.
        if base < 0 and not float(exp_val).is_integer():
            return float("nan")
        try:
            return float(base**exp_val)
        except (ValueError, ZeroDivisionError, OverflowError):
            return float("nan")
    if h == SQRT:
        val = _ev(args[0])
        if val < 0:
            return float("nan")
        return math.sqrt(val)

    # Exponential / logarithm
    if h == EXP:
        val = _ev(args[0])
        if val == math.inf:
            return math.inf
        if val == -math.inf:
            return 0.0
        try:
            return math.exp(val)
        except OverflowError:
            return math.inf
    if h == LOG:
        val = _ev(args[0])
        if val == 0.0:
            return -math.inf  # log(0+) = -∞
        if val < 0:
            return float("nan")
        if val == math.inf:
            return math.inf
        return math.log(val)

    # Trig — undefined at ±∞ (oscillates)
    if h == SIN:
        val = _ev(args[0])
        if math.isinf(val):
            return float("nan")
        return math.sin(val)
    if h == COS:
        val = _ev(args[0])
        if math.isinf(val):
            return float("nan")
        return math.cos(val)
    if h == TAN:
        val = _ev(args[0])
        if math.isinf(val):
            return float("nan")
        return math.tan(val)
    if h == ATAN:
        val = _ev(args[0])
        if val == math.inf:
            return math.pi / 2
        if val == -math.inf:
            return -math.pi / 2
        return math.atan(val)

    # Hyperbolic
    if h == SINH:
        val = _ev(args[0])
        try:
            return math.sinh(val)
        except OverflowError:
            return math.copysign(math.inf, val)
    if h == COSH:
        val = _ev(args[0])
        try:
            return math.cosh(val)
        except OverflowError:
            return math.inf
    if h == TANH:
        val = _ev(args[0])
        return math.tanh(val)

    return float("nan")  # unknown head → indeterminate for our purposes


def _num_eval(node: IRNode) -> float:
    """Safely evaluate *node* to float; return ``nan`` on any exception."""
    try:
        return _ev(node)
    except Exception:  # noqa: BLE001
        return float("nan")


# ---------------------------------------------------------------------------
# Section 2 — Form classification at a numeric test point
# ---------------------------------------------------------------------------


def _eval_at_float(expr: IRNode, var: IRSymbol, pt: float) -> float:
    """Substitute a Python float *pt* for *var* in *expr* and evaluate.

    This is the core classification helper.  We build a temporary
    ``IRFloat`` node for the test point so that ``_num_eval`` can
    evaluate the substituted expression without constructing exact
    rational IR.
    """
    test_node: IRNode = IRFloat(pt) if isinstance(pt, float) else IRInteger(int(pt))
    substituted = subst(test_node, var, expr)
    return _num_eval(substituted)


def _point_to_float(point: IRNode) -> float:
    """Convert an IR point to a float for numeric classification."""
    return _num_eval(point)


# ---------------------------------------------------------------------------
# Section 3 — L'Hôpital step
# ---------------------------------------------------------------------------


def _lhopital_step(
    numer: IRNode,
    denom: IRNode,
    var: IRSymbol,
    point: IRNode,
    direction: str | None,
    diff_fn: DiffFn,
    eval_fn: EvalFn | None,
    depth: int,
) -> IRNode:
    """Apply one step of L'Hôpital and recurse.

    Differentiates *numer* and *denom* with respect to *var*, builds
    the new ratio, and calls :func:`limit_advanced` on the result.

    Parameters
    ----------
    numer, denom:
        The current numerator and denominator IR nodes.
    var:
        The differentiation / limit variable.
    point:
        The limit point.
    direction:
        One-sided direction or ``None``.
    diff_fn:
        Injected differentiation callable.
    eval_fn:
        Optional VM evaluator.
    depth:
        Current recursion depth (incremented before recursive call).

    Returns
    -------
    The limit of ``d(numer)/d(var) / d(denom)/d(var)`` at ``point``.
    """
    d_numer = diff_fn(numer, var)
    d_denom = diff_fn(denom, var)
    if eval_fn is not None:
        d_numer = eval_fn(d_numer)
        d_denom = eval_fn(d_denom)
    new_ratio = IRApply(DIV, (d_numer, d_denom))
    if eval_fn is not None:
        new_ratio = eval_fn(new_ratio)
    return limit_advanced(
        new_ratio, var, point, direction,
        diff_fn=diff_fn, eval_fn=eval_fn, _depth=depth + 1,
    )


# ---------------------------------------------------------------------------
# Section 4 — 0·∞ rewrite
# ---------------------------------------------------------------------------


def _zero_inf_rewrite(
    a: IRNode,
    b: IRNode,
    var: IRSymbol,
    point: IRNode,
    exact_pt: float,
    diff_fn: DiffFn | None,
    eval_fn: EvalFn | None,
    depth: int,
) -> IRNode | None:
    """Rewrite a ``0·∞`` product as a quotient for L'Hôpital.

    Given factors *a* and *b* where one approaches 0 and the other ±∞,
    returns ``limit(b / (1/a))`` — putting the zero factor in the
    denominator inverted, giving a 0/0 or ∞/∞ form.

    *exact_pt* is the numeric value of the limit point (may be ±inf).

    Returns ``None`` if neither factor pattern matches.
    """
    a_val = _eval_at_float(a, var, exact_pt)
    b_val = _eval_at_float(b, var, exact_pt)

    def _is_inf(v: float) -> bool:
        return math.isinf(v) or abs(v) > _INF_THRESHOLD

    # a → 0, b → ±∞: rewrite as b / (1/a)
    if a_val == 0.0 and _is_inf(b_val):
        one_over_a = IRApply(DIV, (IRInteger(1), a))
        new_expr = IRApply(DIV, (b, one_over_a))
        if eval_fn is not None:
            new_expr = eval_fn(new_expr)
        return limit_advanced(
            new_expr, var, point, None,
            diff_fn=diff_fn, eval_fn=eval_fn, _depth=depth + 1,
        )
    # b → 0, a → ±∞: rewrite as a / (1/b)
    if b_val == 0.0 and _is_inf(a_val):
        one_over_b = IRApply(DIV, (IRInteger(1), b))
        new_expr = IRApply(DIV, (a, one_over_b))
        if eval_fn is not None:
            new_expr = eval_fn(new_expr)
        return limit_advanced(
            new_expr, var, point, None,
            diff_fn=diff_fn, eval_fn=eval_fn, _depth=depth + 1,
        )
    return None


# ---------------------------------------------------------------------------
# Section 5 — exp-log rewrite for 1^∞, 0^0, ∞^0
# ---------------------------------------------------------------------------


def _pow_exp_log(
    base: IRNode,
    exp_node: IRNode,
    var: IRSymbol,
    point: IRNode,
    exact_pt: float,
    diff_fn: DiffFn | None,
    eval_fn: EvalFn | None,
    depth: int,
) -> IRNode | None:
    """Rewrite a power ``base^exp`` for indeterminate exponential forms.

    Transforms:

    - ``1^∞`` → ``exp(∞ · log(1))`` → ``exp(lim(∞ · log(base)))``
    - ``0^0`` → ``exp(0 · log(0))`` → ``exp(lim(0 · log(base)))``
    - ``∞^0`` → ``exp(0 · log(∞))`` → ``exp(lim(0 · log(base)))``

    In all cases the inner limit is ``∞ · 0`` (or ``0 · ∞``), which the
    main dispatcher handles via the 0·∞ rewrite.

    *exact_pt* is the numeric value of the limit point (may be ±inf).

    Returns ``None`` if the form is not one of the three above.
    """
    b_val = _eval_at_float(base, var, exact_pt)
    e_val = _eval_at_float(exp_node, var, exact_pt)

    def _is_inf(v: float) -> bool:
        return math.isinf(v) or abs(v) > _INF_THRESHOLD

    is_1_inf = (abs(b_val - 1.0) < 1e-10) and _is_inf(e_val)
    is_0_0 = (b_val == 0.0 and e_val == 0.0)
    is_inf_0 = _is_inf(b_val) and e_val == 0.0

    if not (is_1_inf or is_0_0 or is_inf_0):
        return None

    # Transform base^exp → exp(exp * log(base))
    log_base = IRApply(LOG, (base,))
    product = IRApply(MUL, (exp_node, log_base))
    if eval_fn is not None:
        product = eval_fn(product)

    # Compute limit of the exponent, then wrap in EXP.
    # Direction is not forwarded — the inner product limit is two-sided (the
    # exp-log transform removes the power structure that caused the one-sided
    # form, and the product 0·∞ is evaluated without directional bias).
    exponent_limit = limit_advanced(
        product, var, point, None,
        diff_fn=diff_fn, eval_fn=eval_fn, _depth=depth + 1,
    )
    result = IRApply(EXP, (exponent_limit,))
    if eval_fn is not None:
        result = eval_fn(result)
    return result


# ---------------------------------------------------------------------------
# Section 6 — Indeterminate form dispatcher
# ---------------------------------------------------------------------------


def _handle_form(
    expr: IRNode,
    var: IRSymbol,
    point: IRNode,
    direction: str | None,
    diff_fn: DiffFn | None,
    eval_fn: EvalFn | None,
    depth: int,
) -> IRNode:
    """Dispatch indeterminate forms to their respective reductions.

    Called when direct substitution produces ``NaN`` (an indeterminate
    result).  Examines the **structure** of *expr* to determine the form
    and apply the correct transformation.

    Handles:

    - ``DIV(N, D)`` where both → 0 or both → ±∞: L'Hôpital.
    - ``MUL(a, b)`` where one → 0 and other → ±∞: 0·∞ rewrite.
    - ``POW(b, e)`` for 1^∞, 0^0, ∞^0: exp-log rewrite.

    Falls through to unevaluated ``Limit(…)`` for other forms
    (∞−∞, truly oscillating, etc.).
    """
    pt_f = _point_to_float(point)
    # Use the EXACT limit point for form classification — not the perturbed
    # test point.  The perturbation in ``limit_advanced`` is only for
    # detecting the sign of a directional ±∞.  Here we need the values that
    # the sub-expressions actually approach, which is at the exact limit point.
    # For finite points this is the exact coordinate; for ±∞ points it is inf.
    exact_pt = pt_f  # may be math.inf / -math.inf for limits at ±∞

    # --- DIV: 0/0 or ∞/∞ → L'Hôpital ---
    if isinstance(expr, IRApply) and expr.head == DIV:
        numer, denom = expr.args
        n_val = _eval_at_float(numer, var, exact_pt)
        d_val = _eval_at_float(denom, var, exact_pt)
        is_zero_zero = n_val == 0.0 and d_val == 0.0
        is_inf_inf = (math.isinf(n_val) or abs(n_val) > _INF_THRESHOLD) and (
            math.isinf(d_val) or abs(d_val) > _INF_THRESHOLD
        )
        if (is_zero_zero or is_inf_inf) and diff_fn is not None:
            return _lhopital_step(
                numer, denom, var, point, direction, diff_fn, eval_fn, depth
            )

    # --- MUL(a, b): 0·∞ form ---
    if isinstance(expr, IRApply) and expr.head == MUL and len(expr.args) == 2:
        a, b = expr.args
        result = _zero_inf_rewrite(
            a, b, var, point, exact_pt, diff_fn, eval_fn, depth
        )
        if result is not None:
            return result

    # --- POW(b, e): 1^∞, 0^0, ∞^0 ---
    if isinstance(expr, IRApply) and expr.head == POW:
        base, exp_node = expr.args
        result = _pow_exp_log(
            base, exp_node, var, point, exact_pt, diff_fn, eval_fn, depth
        )
        if result is not None:
            return result

    # --- SUB: try rewriting a - b as (a - b) / 1 and checking ---
    # (limited ∞-∞ handling: pass through for now)

    # Fallthrough — return unevaluated
    return _build_unevaluated(expr, var, point, direction)


# ---------------------------------------------------------------------------
# Section 7 — Public entry point
# ---------------------------------------------------------------------------


def _build_unevaluated(
    expr: IRNode,
    var: IRSymbol,
    point: IRNode,
    direction: str | None,
) -> IRApply:
    """Build an unevaluated ``Limit(…)`` IR node."""
    if direction is None:
        return IRApply(LIMIT, (expr, var, point))
    dir_sym = IRSymbol("plus") if direction == "plus" else IRSymbol("minus")
    return IRApply(LIMIT, (expr, var, point, dir_sym))


def limit_advanced(
    expr: IRNode,
    var: IRSymbol,
    point: IRNode,
    direction: str | None = None,
    *,
    diff_fn: DiffFn | None = None,
    eval_fn: EvalFn | None = None,
    _depth: int = 0,
) -> IRNode:
    """Compute ``lim_{var → point} expr``, handling indeterminate forms.

    Parameters
    ----------
    expr:
        The expression whose limit is sought.
    var:
        The limit variable (must be an ``IRSymbol``).
    point:
        The limit point.  Use ``IRSymbol("inf")`` / ``IRSymbol("minf")``
        for limits at ±∞.
    direction:
        ``None`` — two-sided (default).
        ``"plus"`` — from the right (``x → a+``).
        ``"minus"`` — from the left (``x → a-``).
    diff_fn:
        Callable ``(expr, var) → derivative``.  **Required** for
        L'Hôpital's rule and the 0·∞ / exponential reductions.  If
        ``None``, any indeterminate form falls through to unevaluated.
    eval_fn:
        Optional VM evaluator.  When provided, intermediate results are
        simplified before further processing.  Pass ``vm.eval`` from the
        handler.
    _depth:
        Internal recursion counter (do not pass externally).

    Returns
    -------
    An evaluated IR node, or ``IRApply(LIMIT, …)`` if the limit cannot
    be determined.

    Examples
    --------
    ::

        # lim sin(x)/x as x→0 = 1  (via L'Hôpital)
        limit_advanced(DIV(SIN(x), x), x, 0, diff_fn=d, eval_fn=e)

        # lim (1 + 1/x)^x as x→∞ = %e  (via exp-log + L'Hôpital)
        limit_advanced(POW(1 + 1/x, x), x, inf, diff_fn=d, eval_fn=e)

        # lim x*log(x) as x→0+  =  0  (via 0·∞ rewrite + L'Hôpital)
        limit_advanced(MUL(x, LOG(x)), x, 0, "plus", diff_fn=d, eval_fn=e)
    """
    if _depth > _MAX_DEPTH:
        return _build_unevaluated(expr, var, point, direction)

    # --- Step 1: Compute the numeric test point ---
    # For one-sided limits, perturb by ε for form classification only.
    # The symbolic substitution always uses the exact *point*.
    pt_f = _point_to_float(point)
    if math.isnan(pt_f):
        # Cannot determine the limit point numerically — return unevaluated.
        return _build_unevaluated(expr, var, point, direction)

    eps = _EPS if direction != "minus" else -_EPS
    test_pt = pt_f + eps if not math.isinf(pt_f) else pt_f

    # --- Step 2: Evaluate the expression at the perturbed test point ---
    # This tells us the directional behaviour: ±∞ or finite-or-indeterminate.
    val = _eval_at_float(expr, var, test_pt)

    # --- Step 3: ±∞ at the directional test point → return sentinel ---
    # The signed result captures one-sided direction correctly (e.g.
    # 1/x → +∞ from the right, −∞ from the left).
    # We treat |val| > _INF_THRESHOLD as effectively ±∞ since 1/_EPS = 1e300
    # is representable but clearly means the expression diverges.
    if math.isinf(val) or abs(val) > _INF_THRESHOLD:
        return INF_SYM if val > 0 else MINF_SYM

    # --- Step 4: NaN at perturbed point → indeterminate form ---
    if math.isnan(val):
        return _handle_form(
            expr, var, point, direction, diff_fn, eval_fn, _depth
        )

    # --- Step 5: Perturbed value is finite → try exact substitution ---
    # Substitute the exact limit point symbolically.  If the simplified
    # result is also finite (not NaN), the function is continuous here and
    # we can return the substituted expression directly.  If the exact
    # substitution collapses to NaN (removable singularity, e.g. sin(x)/x
    # at x=0), we fall through to L'Hôpital / rewrite.
    subst_result = subst(point, var, expr)
    if eval_fn is not None:
        subst_result = eval_fn(subst_result)
    exact_val = _num_eval(subst_result)

    if not math.isnan(exact_val):
        if math.isinf(exact_val):
            return INF_SYM if exact_val > 0 else MINF_SYM
        return subst_result

    # --- Step 6: Exact substitution is NaN — removable singularity ---
    return _handle_form(
        expr, var, point, direction, diff_fn, eval_fn, _depth
    )
