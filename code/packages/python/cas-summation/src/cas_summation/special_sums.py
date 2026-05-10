"""Classic convergent infinite series with known closed forms.

These are "table look-up" results for well-known infinite series that
the Risch/Faulhaber machinery cannot derive algorithmically:

    Basel problem (Euler 1734):
        Σ_{k=1}^∞ 1/k²  =  π²/6

    Basel-4 (Euler):
        Σ_{k=1}^∞ 1/k⁴  =  π⁴/90

    Leibniz formula for π:
        Σ_{k=0}^∞ (−1)^k / (2k+1)  =  π/4

    Taylor series for e:
        Σ_{k=0}^∞ 1/k!  =  e

    Taylor series for exp(x):
        Σ_{k=0}^∞ x^k / k!  =  exp(x)

    Each is recognised by structural pattern-matching on the unevaluated IR
    tree of the summand *f* and the lower bound *lo*.

Note on factorial representation
---------------------------------
Historical MACSYMA and most modern CAS systems represent *k!* as
``Gamma(k+1)`` in the IR (since Gamma is the analytic continuation of
factorial to non-integers, and avoids a separate head symbol).  We follow
the same convention: ``1/k!`` is expected as ``DIV(1, GAMMA_FUNC(k+1))``
or equivalently ``MUL(INV, GAMMA_FUNC(k+1))``.  If the summand arrives in
a different form (e.g. a raw ``FACTORIAL`` head) it simply falls through to
the unevaluated case.

Usage::

    from symbolic_ir import IRSymbol, IRInteger, DIV, POW
    from cas_summation.special_sums import try_special_infinite

    k = IRSymbol("k")
    f = DIV(IRInteger(1), POW(k, IRInteger(2)))   # 1/k²
    lo = IRInteger(1)
    result = try_special_infinite(f, k, lo)
    # result = MUL(POW(%pi, 2), IRRational(1, 6))  →  π²/6
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    DIV,
    EXP,
    GAMMA_FUNC,
    MUL,
    NEG,
    POW,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# π and e as IR symbol literals (same convention used throughout the VM).
_PI = IRSymbol("%pi")
_E = IRSymbol("%e")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _frac(p: int, q: int) -> IRNode:
    return IRRational(p, q)


def _is_int(node: IRNode, value: int) -> bool:
    """True iff *node* is an IRInteger equal to *value*."""
    return isinstance(node, IRInteger) and node.value == value


def _is_symbol(node: IRNode, name: str) -> bool:
    return isinstance(node, IRSymbol) and node.name == name


# ---------------------------------------------------------------------------
# Pattern-matching helpers
# ---------------------------------------------------------------------------


def _match_inv_k_pow(f: IRNode, k: IRSymbol, exp: int) -> bool:
    """True iff *f* represents 1/k^exp."""
    # DIV(1, POW(k, exp))
    if (
        isinstance(f, IRApply)
        and f.head == DIV
        and len(f.args) == 2
        and _is_int(f.args[0], 1)
    ):
        inner = f.args[1]
        if isinstance(inner, IRApply) and inner.head == POW and len(inner.args) == 2:
            base, e = inner.args
            return base == k and _is_int(e, exp)
    return False


def _match_leibniz(f: IRNode, k: IRSymbol) -> bool:
    """True iff *f* represents (-1)^k / (2k + 1).

    MACSYMA/IR tree form:
        DIV(POW(-1, k),  ADD(MUL(2, k), 1))
    or
        DIV(POW(NEG(1), k),  ADD(MUL(2, k), 1))
    """
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return False
    numerator, denominator = f.args

    # numerator = (-1)^k  or  NEG(1)^k
    num_ok = False
    if (
        isinstance(numerator, IRApply)
        and numerator.head == POW
        and len(numerator.args) == 2
    ):
        base_n, exp_n = numerator.args
        if exp_n == k and (
            _is_int(base_n, -1)
            or (
                isinstance(base_n, IRApply)
                and base_n.head == NEG
                and len(base_n.args) == 1
                and _is_int(base_n.args[0], 1)
            )
        ):
            num_ok = True
    if not num_ok:
        return False

    # denominator = 2*k + 1  or  1 + 2*k
    denom_ok = False
    if (
        isinstance(denominator, IRApply)
        and denominator.head == ADD
        and len(denominator.args) == 2
    ):
        a, b = denominator.args
        # 2*k + 1
        if (
            isinstance(a, IRApply)
            and a.head == MUL
            and len(a.args) == 2
            and _is_int(a.args[0], 2)
            and a.args[1] == k
            and _is_int(b, 1)
        ):
            denom_ok = True
        # 1 + 2*k
        if (
            _is_int(a, 1)
            and isinstance(b, IRApply)
            and b.head == MUL
            and len(b.args) == 2
            and _is_int(b.args[0], 2)
            and b.args[1] == k
        ):
            denom_ok = True
    return denom_ok


def _match_inv_factorial(f: IRNode, k: IRSymbol) -> bool:
    """True iff *f* represents 1/k! = 1/Gamma(k+1).

    Expected IR form:
        DIV(1,  GAMMA_FUNC(ADD(k, 1)))
    """
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return False
    numerator, denominator = f.args
    if not _is_int(numerator, 1):
        return False
    # denominator = GAMMA_FUNC(k + 1)
    if not (
        isinstance(denominator, IRApply)
        and denominator.head == GAMMA_FUNC
        and len(denominator.args) == 1
    ):
        return False
    arg = denominator.args[0]
    return (
        isinstance(arg, IRApply)
        and arg.head == ADD
        and len(arg.args) == 2
        and arg.args[0] == k
        and _is_int(arg.args[1], 1)
    )


def _match_exp_series(f: IRNode, k: IRSymbol) -> IRNode | None:
    """If *f* represents x^k / k! = x^k / Gamma(k+1), return x; else None.

    Expected IR form:
        DIV(POW(x, k),  GAMMA_FUNC(ADD(k, 1)))
    """
    if not (isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2):
        return None
    numerator, denominator = f.args
    # numerator = x^k
    if not (
        isinstance(numerator, IRApply)
        and numerator.head == POW
        and len(numerator.args) == 2
        and numerator.args[1] == k
    ):
        return None
    x = numerator.args[0]
    if x == k:  # avoid x = k degenerate
        return None
    # denominator = GAMMA_FUNC(k + 1)
    if not (
        isinstance(denominator, IRApply)
        and denominator.head == GAMMA_FUNC
        and len(denominator.args) == 1
    ):
        return None
    arg = denominator.args[0]
    if not (
        isinstance(arg, IRApply)
        and arg.head == ADD
        and len(arg.args) == 2
        and arg.args[0] == k
        and _is_int(arg.args[1], 1)
    ):
        return None
    return x


# ---------------------------------------------------------------------------
# Public function
# ---------------------------------------------------------------------------


def try_special_infinite(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
) -> IRNode | None:
    """Return the closed form for a recognised infinite series, or None.

    The function checks *f* against a table of classic convergent series and
    returns the corresponding IR constant or expression.  If *f* does not
    match any known pattern, ``None`` is returned.

    Parameters
    ----------
    f:
        The summand expression (may contain *k*).
    k:
        The index variable (an ``IRSymbol``).
    lo:
        The lower bound IR node.

    Returns
    -------
    IRNode | None
        Closed form, or ``None``.

    Recognised series
    -----------------
    Σ_{k=1}^∞ 1/k²       →  π²/6
    Σ_{k=1}^∞ 1/k⁴       →  π⁴/90
    Σ_{k=0}^∞ (-1)^k/(2k+1) →  π/4
    Σ_{k=0}^∞ 1/k!       →  %e
    Σ_{k=0}^∞ x^k/k!     →  exp(x)
    """
    # ── Basel: Σ 1/k², k=1..∞ ──────────────────────────────────────────────
    if _is_int(lo, 1) and _match_inv_k_pow(f, k, 2):
        # π²/6
        return _div(_pow(_PI, _int(2)), _int(6))

    # ── Basel-4: Σ 1/k⁴, k=1..∞ ────────────────────────────────────────────
    if _is_int(lo, 1) and _match_inv_k_pow(f, k, 4):
        # π⁴/90
        return _div(_pow(_PI, _int(4)), _int(90))

    # ── Leibniz: Σ (-1)^k/(2k+1), k=0..∞ = π/4 ────────────────────────────
    if _is_int(lo, 0) and _match_leibniz(f, k):
        return _div(_PI, _int(4))

    # ── Taylor for e: Σ 1/k!, k=0..∞ ───────────────────────────────────────
    if _is_int(lo, 0) and _match_inv_factorial(f, k):
        return _E

    # ── Taylor for exp(x): Σ x^k/k!, k=0..∞ ─────────────────────────────
    if _is_int(lo, 0):
        x = _match_exp_series(f, k)
        if x is not None:
            return IRApply(EXP, (x,))

    return None
