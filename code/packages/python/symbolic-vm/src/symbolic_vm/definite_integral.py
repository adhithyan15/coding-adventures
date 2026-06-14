"""Phase 24 — Definite integration via the Fundamental Theorem of Calculus.

The key identity is:

    ∫_a^b f(x) dx  =  F(b) − F(a)

where F is *any* antiderivative of f (constant of integration cancels).

Usage
-----
The public entry point is :func:`evaluate_definite`.  It is called from
the ``Integrate`` handler in ``integrate.py`` when a 4-argument
``Integrate(f, x, a, b)`` node is encountered.

Finite limits
~~~~~~~~~~~~~
Evaluated by structural substitution:

    F_val = vm.eval( subst(limit_value, x, F) )

where ``subst(value, var, expr)`` is the ``cas_substitution`` convention
— first argument is the replacement value, second is the variable, third
is the expression.

Infinite limits
~~~~~~~~~~~~~~~
``IRSymbol("%inf")`` and ``IRSymbol("%minf")`` (MACSYMA compiler output)
are handled by :func:`_eval_at_inf`, which walks the antiderivative tree
and applies a table of one-sided limits for the special functions
introduced in Phase 23.

Internal ``IRSymbol("inf")`` / ``IRSymbol("minf")`` used by
``limit_advanced`` are also accepted.

Divergence
~~~~~~~~~~
If the antiderivative has no finite limit at an infinite endpoint
(e.g. ``∫₀^∞ exp(x) dx``), :func:`_eval_at_inf` returns ``None`` and the
whole definite integral is left unevaluated as ``Integrate(f, x, a, b)``.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    ATAN,
    CHI,
    CI,
    COTH,
    CSCH,
    DIV,
    ERF,
    ERFC,
    ERFI,
    EXP,
    FRESNEL_C,
    FRESNEL_S,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SECH,
    SHI,
    SI,
    SUB,
    TANH,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# Infinity recognition helpers
# ---------------------------------------------------------------------------

# The MACSYMA compiler (macsyma-compiler) produces %inf / %minf.
# The internal limit engine (cas_limit_series) uses inf / minf.
# We accept both.

_INF_NAMES: frozenset[str] = frozenset({"%inf", "inf"})
_MINF_NAMES: frozenset[str] = frozenset({"%minf", "minf"})


def _is_inf(node: IRNode) -> bool:
    """True if *node* represents +∞."""
    return isinstance(node, IRSymbol) and node.name in _INF_NAMES


def _is_minf(node: IRNode) -> bool:
    """True if *node* represents −∞."""
    return isinstance(node, IRSymbol) and node.name in _MINF_NAMES


def _is_finite_limit(node: IRNode) -> bool:
    """True if *node* is a concrete finite constant.

    Integers, rationals, floats, and symbolic constants (like ``%pi``) are all finite.
    Only ``%inf`` and ``%minf`` are excluded.
    """
    if isinstance(node, (IRInteger, IRRational, IRFloat)):
        return True
    # Treat symbolic constants (e.g. %pi, %e) as finite.  They are not
    # %inf / %minf, so fall through to True.
    if isinstance(node, IRSymbol):
        return node.name not in _INF_NAMES and node.name not in _MINF_NAMES
    # Composed expressions like -1, 2*%pi, etc. are also finite.
    return True


# ---------------------------------------------------------------------------
# Constant-checking helper
# ---------------------------------------------------------------------------

def _is_const_wrt(expr: IRNode, x: IRSymbol) -> bool:
    """Return True if *expr* does not depend on the symbol *x*.

    Walks the expression tree.  Heads (like ``Erf``) are always
    ``IRSymbol`` nodes that won't match *x*, so we only need to recurse
    into ``IRApply.args``.
    """
    if expr == x:
        return False
    if isinstance(expr, (IRInteger, IRRational, IRFloat, IRSymbol)):
        # IRSymbol: either x (already caught) or another name (constant).
        return True
    if isinstance(expr, IRApply):
        return all(_is_const_wrt(a, x) for a in expr.args)
    return True


# ---------------------------------------------------------------------------
# Rational-number IR helper (avoids circular import with polynomial_bridge)
# ---------------------------------------------------------------------------

def _frac_ir(c: Fraction) -> IRNode:
    """Lift a ``Fraction`` to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _ir_rational_val(node: IRNode) -> Fraction | None:
    """Extract numeric value from an IRInteger or IRRational, else None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


# ---------------------------------------------------------------------------
# Argument-sign helper
# ---------------------------------------------------------------------------

def _const_sign(node: IRNode, x: IRSymbol) -> int | None:
    """Return the sign (+1 or −1) of a constant expression, or None if unknown.

    Only called when ``_is_const_wrt(node, x)`` is True.  Handles
    IRInteger, IRRational, IRFloat, and composed constant expressions
    like ``Sqrt(positive)``, ``Div(pos, pos)``, etc.
    """
    from symbolic_ir import SQRT  # noqa: PLC0415

    if isinstance(node, IRInteger):
        if node.value > 0:
            return +1
        if node.value < 0:
            return -1
        return None  # zero
    if isinstance(node, IRRational):
        s = (1 if node.numer > 0 else -1) * (1 if node.denom > 0 else -1)
        return s if node.numer != 0 else None
    if isinstance(node, IRFloat):
        if node.value > 0:
            return +1
        if node.value < 0:
            return -1
        return None
    if isinstance(node, IRApply):
        # Sqrt of a positive expression is always positive.
        if node.head == SQRT:
            return +1
        # Div(pos, pos) = pos; Div(neg, pos) = neg; etc.
        if node.head == DIV and len(node.args) == 2:
            s0 = _const_sign(node.args[0], x)
            s1 = _const_sign(node.args[1], x)
            if s0 is not None and s1 is not None:
                return s0 * s1
        # NEG(c) reverses sign.
        if node.head == NEG and len(node.args) == 1:
            s = _const_sign(node.args[0], x)
            return -s if s is not None else None
        # MUL(c1, c2): multiply signs.
        if node.head == MUL and len(node.args) == 2:
            s0 = _const_sign(node.args[0], x)
            s1 = _const_sign(node.args[1], x)
            if s0 is not None and s1 is not None:
                return s0 * s1
    return None  # unknown sign


def _arg_sgn_at_inf(arg: IRNode, x: IRSymbol, x_sign: int) -> int | None:
    """Determine the sign of *arg* as x → x_sign · ∞.

    Returns +1 if the argument → +∞, −1 if the argument → −∞, or
    ``None`` if the argument stays finite (does not depend on *x*).

    Handles the patterns that arise in the antiderivatives produced by
    Phases 1–23:

    * Linear:   ``x``, ``NEG(x)``, ``MUL(c, x)``, ``ADD(MUL(c,x), b)``
    * Quadratic:``POW(x, 2)``, ``NEG(POW(x, 2))``, ``MUL(c, POW(x, n))``
    * Rational: ``DIV(u, c)`` with *c* a non-zero constant
    * Constant: any expression with no *x* → ``None``
    """
    if arg == x:
        # The argument *is* x — same sign as x's direction.
        return x_sign

    if _is_const_wrt(arg, x):
        # The argument is a constant; it stays finite at ±∞.
        return None

    if isinstance(arg, IRApply):
        # ---- NEG(u) -------------------------------------------------------
        if arg.head == NEG and len(arg.args) == 1:
            inner = _arg_sgn_at_inf(arg.args[0], x, x_sign)
            if inner is not None:
                return -inner
            return None

        # ---- MUL(c, u)  or  MUL(u, c) ------------------------------------
        # Use _const_sign so we handle Sqrt, Div constants, not just rationals.
        if arg.head == MUL and len(arg.args) == 2:
            a0, a1 = arg.args
            if _is_const_wrt(a0, x):
                c_sgn = _const_sign(a0, x)
                u_sign = _arg_sgn_at_inf(a1, x, x_sign)
                if c_sgn is not None and u_sign is not None:
                    return c_sgn * u_sign
            if _is_const_wrt(a1, x):
                c_sgn = _const_sign(a1, x)
                u_sign = _arg_sgn_at_inf(a0, x, x_sign)
                if c_sgn is not None and u_sign is not None:
                    return c_sgn * u_sign

        # ---- DIV(u, c) — u depends on x, c is a non-zero constant --------
        if arg.head == DIV and len(arg.args) == 2:
            a0, a1 = arg.args
            if _is_const_wrt(a1, x) and not _is_const_wrt(a0, x):
                # u / const:  same asymptotic sign as u × sign(const)
                c_sgn = _const_sign(a1, x)
                u_sign = _arg_sgn_at_inf(a0, x, x_sign)
                if c_sgn is not None and u_sign is not None:
                    return c_sgn * u_sign
            if _is_const_wrt(a0, x) and not _is_const_wrt(a1, x):
                # const / u → 0 (finite) as u → ±∞ — stays finite
                return None

        # ---- ADD(u, c)  or  ADD(c, u) ------------------------------------
        if arg.head == ADD and len(arg.args) == 2:
            a0, a1 = arg.args
            if _is_const_wrt(a0, x):
                return _arg_sgn_at_inf(a1, x, x_sign)
            if _is_const_wrt(a1, x):
                return _arg_sgn_at_inf(a0, x, x_sign)

        # ---- SUB(u, c)  or  SUB(c, u) ------------------------------------
        if arg.head == SUB and len(arg.args) == 2:
            a0, a1 = arg.args
            if _is_const_wrt(a1, x):
                # u − const → same sign as u
                return _arg_sgn_at_inf(a0, x, x_sign)
            if _is_const_wrt(a0, x):
                # const − u → negated sign of u
                u_sign = _arg_sgn_at_inf(a1, x, x_sign)
                return -u_sign if u_sign is not None else None

        # ---- POW(x, n) ---------------------------------------------------
        if arg.head == POW and len(arg.args) == 2 and arg.args[0] == x:
            exp_node = arg.args[1]
            exp_val = _ir_rational_val(exp_node)
            if exp_val is not None:
                if exp_val % 2 == 0:
                    # Even power → always positive infinity.
                    return +1
                # Odd power: sign follows x's sign.
                return x_sign

    # Unknown pattern — can't determine.
    return None


# ---------------------------------------------------------------------------
# Limit at ±∞ for a full antiderivative expression
# ---------------------------------------------------------------------------

# Convenience: the IR nodes for π / 2 and 1 / 2.
_PI = IRSymbol("%pi")
_PI_OVER_2 = IRApply(DIV, (_PI, IRInteger(2)))
_HALF = IRRational(1, 2)
_ONE = IRInteger(1)
_TWO = IRInteger(2)
_ZERO = IRInteger(0)
_NEG_ONE = IRApply(NEG, (_ONE,))
_NEG_PI_OVER_2 = IRApply(NEG, (_PI_OVER_2,))
_NEG_HALF = IRApply(NEG, (_HALF,))


def _eval_at_inf(expr: IRNode, x: IRSymbol, sign: int) -> IRNode | None:
    """Compute lim_{x → sign·∞} expr.

    Returns an ``IRNode`` representing the limit, or ``None`` if the limit
    does not exist (diverges) or cannot be determined.

    ``sign = +1``  means x → +∞.
    ``sign = −1``  means x → −∞.

    The function handles the IR shapes produced by Phases 1–23.  Unknown
    patterns cause it to return ``None`` rather than guessing.
    """
    # -- Constant (no x) -- return as-is ------------------------------------
    if _is_const_wrt(expr, x):
        return expr

    # -- x itself diverges --------------------------------------------------
    if expr == x:
        return None  # x → ±∞

    if not isinstance(expr, IRApply):
        # IRFloat, IRInteger, IRRational, IRSymbol(!= x): all constants.
        return expr

    head = expr.head
    args = expr.args

    # -----------------------------------------------------------------------
    # Arithmetic combinators
    # -----------------------------------------------------------------------

    if head == ADD and len(args) == 2:
        la = _eval_at_inf(args[0], x, sign)
        lb = _eval_at_inf(args[1], x, sign)
        if la is None or lb is None:
            return None
        return IRApply(ADD, (la, lb))

    if head == SUB and len(args) == 2:
        la = _eval_at_inf(args[0], x, sign)
        lb = _eval_at_inf(args[1], x, sign)
        if la is None or lb is None:
            return None
        return IRApply(SUB, (la, lb))

    if head == NEG and len(args) == 1:
        la = _eval_at_inf(args[0], x, sign)
        if la is None:
            return None
        return IRApply(NEG, (la,))

    if head == MUL and len(args) == 2:
        a0, a1 = args
        la = _eval_at_inf(a0, x, sign)
        lb = _eval_at_inf(a1, x, sign)
        if la is None or lb is None:
            return None
        return IRApply(MUL, (la, lb))

    if head == DIV and len(args) == 2:
        la = _eval_at_inf(args[0], x, sign)
        lb = _eval_at_inf(args[1], x, sign)
        if la is None or lb is None:
            return None
        # Guard against 0 denominator — if denominator is the zero
        # constant, leave it unevaluated (the VM handles it).
        return IRApply(DIV, (la, lb))

    # -----------------------------------------------------------------------
    # Special functions: all take exactly one argument
    # -----------------------------------------------------------------------

    if len(args) != 1:
        # Multi-arg heads (e.g. BETA_FUNC) are not handled.
        return None

    u = args[0]
    u_sign = _arg_sgn_at_inf(u, x, sign)

    # -- erf(u): erf(+∞) = 1,  erf(−∞) = −1 --------------------------------
    if head == ERF:
        if u_sign is None:
            return None  # constant argument — leave as erf(const)
        return _ONE if u_sign > 0 else _NEG_ONE

    # -- erfc(u): erfc(+∞) = 0,  erfc(−∞) = 2 ------------------------------
    if head == ERFC:
        if u_sign is None:
            return None
        return _ZERO if u_sign > 0 else _TWO

    # -- erfi(u): diverges at both ±∞ ----------------------------------------
    if head == ERFI:
        return None  # ∫ exp(x²) dx diverges on [0, ∞)

    # -- Si(u): Si(+∞) = π/2,  Si(−∞) = −π/2 --------------------------------
    if head == SI:
        if u_sign is None:
            return None
        return _PI_OVER_2 if u_sign > 0 else _NEG_PI_OVER_2

    # -- Ci(u): Ci(+∞) = 0.  Ci(−∞) is oscillating (undefined in R). --------
    if head == CI:
        if u_sign is None:
            return None
        return _ZERO if u_sign > 0 else None

    # -- Shi(u), Chi(u): both diverge ----------------------------------------
    if head in {SHI, CHI}:
        return None

    # -- atan(u): atan(+∞) = π/2,  atan(−∞) = −π/2 --------------------------
    if head == ATAN:
        if u_sign is None:
            return None
        return _PI_OVER_2 if u_sign > 0 else _NEG_PI_OVER_2

    # -- tanh(u): tanh(+∞) = 1,  tanh(−∞) = −1 ------------------------------
    if head == TANH:
        if u_sign is None:
            return None
        return _ONE if u_sign > 0 else _NEG_ONE

    # -- coth(u): coth(+∞) = 1,  coth(−∞) = −1 ------------------------------
    if head == COTH:
        if u_sign is None:
            return None
        return _ONE if u_sign > 0 else _NEG_ONE

    # -- sech(u): sech(±∞) = 0 -----------------------------------------------
    if head == SECH:
        if u_sign is None:
            return None
        return _ZERO

    # -- csch(u): csch(±∞) = 0 -----------------------------------------------
    if head == CSCH:
        if u_sign is None:
            return None
        return _ZERO

    # -- FresnelS(u): FS(+∞) = 1/2,  FS(−∞) = −1/2 --------------------------
    if head == FRESNEL_S:
        if u_sign is None:
            return None
        return _HALF if u_sign > 0 else _NEG_HALF

    # -- FresnelC(u): FC(+∞) = 1/2,  FC(−∞) = −1/2 --------------------------
    if head == FRESNEL_C:
        if u_sign is None:
            return None
        return _HALF if u_sign > 0 else _NEG_HALF

    # -- exp(u): exp(−∞) = 0,  exp(+∞) diverges ------------------------------
    if head == EXP:
        if u_sign is None:
            # Constant argument — leave as exp(const).
            return None
        if u_sign < 0:
            return _ZERO
        return None  # exp(+∞) diverges

    # Unknown head — leave unevaluated.
    return None


# ---------------------------------------------------------------------------
# Limit at zero from the right — handles improper integrals like ∫₀¹ log(x)dx
# ---------------------------------------------------------------------------

def _eval_at_zero_plus(expr: IRNode, x: IRSymbol) -> IRNode | None:
    """Compute lim_{x → 0⁺} expr using symbolic limit rules.

    Needed because direct substitution of x = 0 fails when the antiderivative
    contains ``log(x)`` or ``log(linear of x)`` — the VM raises
    ``ValueError: math domain error`` for ``log(0)``.  However, many improper
    integrals with a logarithmic antiderivative are *convergent*:

        lim_{x→0+}  x · log(x)  = 0          (faster-than-log decay of x)
        lim_{x→0+}  xⁿ · log(x) = 0   (n > 0)

    so their antiderivatives *do* have finite left-boundary values.

    Handles the shapes produced by Phase 1's log-IBP result
    ``∫ log(ax+b) dx = ((ax+b)/a)·log(ax+b) − x + const``  when evaluated
    at a zero of ``ax+b`` (e.g. ``x = 0`` for ``a=1, b=0``).

    Returns an ``IRNode`` (the limit) or ``None`` (diverges / unknown).
    """
    # Constants don't depend on x — return unchanged.
    if _is_const_wrt(expr, x):
        return expr

    # x → 0
    if expr == x:
        return _ZERO

    if not isinstance(expr, IRApply):
        return expr

    head, args = expr.head, expr.args

    if head == ADD and len(args) == 2:
        la = _eval_at_zero_plus(args[0], x)
        lb = _eval_at_zero_plus(args[1], x)
        if la is None or lb is None:
            return None
        return IRApply(ADD, (la, lb))

    if head == SUB and len(args) == 2:
        la = _eval_at_zero_plus(args[0], x)
        lb = _eval_at_zero_plus(args[1], x)
        if la is None or lb is None:
            return None
        return IRApply(SUB, (la, lb))

    if head == NEG and len(args) == 1:
        la = _eval_at_zero_plus(args[0], x)
        if la is None:
            return None
        return IRApply(NEG, (la,))

    if head == DIV and len(args) == 2:
        la = _eval_at_zero_plus(args[0], x)
        lb = _eval_at_zero_plus(args[1], x)
        if la is None or lb is None:
            return None
        return IRApply(DIV, (la, lb))

    if head == MUL and len(args) == 2:
        a0, a1 = args

        # Special case: u · log(v) where u → 0 faster than log(v) → −∞.
        # Covers x · log(x), xⁿ · log(x), x · log(ax+b), etc.
        # L'Hôpital: lim_{x→0+} xⁿ · log(x) = 0 for any n > 0.
        def _is_log_of_x(node: IRNode) -> bool:
            if not isinstance(node, IRApply) or node.head != LOG:
                return False
            # The argument of log must vanish at x=0 (e.g. is x or ax+b with b=0).
            arg = node.args[0]
            if arg == x:
                return True
            # Linear: MUL(c, x) with constant c
            return (
                isinstance(arg, IRApply)
                and arg.head == MUL
                and len(arg.args) == 2
                and _is_const_wrt(arg.args[0], x)
                and arg.args[1] == x
            )

        def _limit_zero_plus(node: IRNode) -> IRNode | None:
            """Evaluate lim_{x→0+} node; None if diverges."""
            return _eval_at_zero_plus(node, x)

        if _is_log_of_x(a1):
            # u · log(x): check if u → 0 at zero (i.e. u ∝ x^n)
            u_lim = _limit_zero_plus(a0)
            if u_lim is not None and _is_zero_node(u_lim):
                return _ZERO
        if _is_log_of_x(a0):
            u_lim = _limit_zero_plus(a1)
            if u_lim is not None and _is_zero_node(u_lim):
                return _ZERO

        # General MUL — recurse into both factors.
        la = _eval_at_zero_plus(a0, x)
        lb = _eval_at_zero_plus(a1, x)
        if la is None or lb is None:
            return None
        return IRApply(MUL, (la, lb))

    # LOG(x) at x=0 diverges — return None.
    # LOG(c) for constant c ≠ 0 is fine — already handled by const check.
    if head == LOG and len(args) == 1:
        # Is the argument a zero-of-x form?
        arg = args[0]
        if _is_const_wrt(arg, x):
            return expr  # log(constant) stays
        # Otherwise the log argument vanishes → diverges
        return None

    # POW(x, n) → 0 for positive n.
    if head == POW and len(args) == 2 and args[0] == x:
        n_val = _ir_rational_val(args[1])
        if n_val is not None and n_val > 0:
            return _ZERO

    # For other heads (trig, exp, etc.) — try direct substitution
    # of x=0 into arguments, then rebuild.
    new_args = []
    for a in args:
        a_lim = _eval_at_zero_plus(a, x)
        if a_lim is None:
            return None
        new_args.append(a_lim)
    return IRApply(head, tuple(new_args))


def _is_zero_node(node: IRNode) -> bool:
    """True if *node* is the literal zero (IRInteger(0))."""
    return isinstance(node, IRInteger) and node.value == 0


# ---------------------------------------------------------------------------
# Evaluate F at a single limit (finite or infinite)
# ---------------------------------------------------------------------------

def _at_limit(
    F: IRNode,
    x: IRSymbol,
    limit: IRNode,
    vm: object,  # type: VM, avoid circular import
) -> IRNode | None:
    """Evaluate the antiderivative *F* at *limit*.

    For finite limits, substitutes and calls ``vm.eval``.  If direct
    substitution raises an exception (e.g. ``ValueError`` for ``log(0)``),
    falls back to :func:`_eval_at_zero_plus` when the limit is zero.
    For infinite limits, uses :func:`_eval_at_inf`.
    Returns ``None`` if the evaluation fails (e.g. diverges).
    """
    from cas_substitution import subst  # noqa: PLC0415 — lazy import

    if _is_inf(limit):
        result = _eval_at_inf(F, x, sign=+1)
        if result is None:
            return None
        return vm.eval(result)

    if _is_minf(limit):
        result = _eval_at_inf(F, x, sign=-1)
        if result is None:
            return None
        return vm.eval(result)

    # Finite limit — structural substitution then evaluate.
    # subst(value, var, expr) replaces var with value in expr.
    substituted = subst(limit, x, F)
    try:
        return vm.eval(substituted)
    except (ValueError, ZeroDivisionError, ArithmeticError):
        # Direct substitution failed (e.g. log(0) at x=0).
        # Try the zero-from-the-right symbolic limit as a fallback.
        if _is_zero_node(limit):
            sym_lim = _eval_at_zero_plus(F, x)
            if sym_lim is not None:
                try:
                    return vm.eval(sym_lim)
                except (ValueError, ZeroDivisionError, ArithmeticError):
                    pass
        return None


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def evaluate_definite(
    f: IRNode,
    x: IRSymbol,
    a: IRNode,
    b: IRNode,
    antiderivative_F: IRNode,
    vm: object,
) -> IRNode:
    """Apply the Fundamental Theorem of Calculus: F(b) − F(a).

    Parameters
    ----------
    f:
        The integrand (used to construct the unevaluated fallback).
    x:
        The integration variable.
    a, b:
        The lower and upper limits of integration.
    antiderivative_F:
        An antiderivative of *f* with respect to *x*, already computed by
        the indefinite-integration machinery.  Must NOT be an unevaluated
        ``Integrate(f, x)`` node — callers must check this first.
    vm:
        The running VM instance (needed to evaluate sub-expressions).

    Returns
    -------
    IRNode
        ``vm.eval(F(b) − F(a))`` if both limits can be evaluated, or the
        original 4-argument ``Integrate(f, x, a, b)`` node if evaluation
        fails at either endpoint.
    """
    F_b = _at_limit(antiderivative_F, x, b, vm)
    if F_b is None:
        return IRApply(INTEGRATE, (f, x, a, b))

    F_a = _at_limit(antiderivative_F, x, a, vm)
    if F_a is None:
        return IRApply(INTEGRATE, (f, x, a, b))

    return vm.eval(IRApply(SUB, (F_b, F_a)))
