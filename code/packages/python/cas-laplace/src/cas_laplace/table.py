"""Forward Laplace transform table.

The Laplace transform is evaluated by matching f(t) against known patterns
and returning the closed-form transform F(s). This is a *table-driven*
approach: every standard textbook includes a table of pairs (f(t), F(s))
and the linearity property:

    L{a·f(t) + b·g(t)} = a·L{f(t)} + b·L{g(t)}

This file implements that table as a list of ``(matcher, transformer)``
pairs, along with the helper functions that decompose IR trees into their
constituent parts.

Design philosophy
-----------------
Each entry in the table is a tuple:

    (pattern_fn, transform_fn)

``pattern_fn(f, t_sym)`` attempts to recognize f as a known pattern. If
recognized, it returns a dictionary of extracted parameters (e.g. ``a``,
``omega``). If not recognized, it returns ``None``.

``transform_fn(params, s_sym)`` takes the extracted parameters and builds
the IR for the Laplace transform F(s).

This split makes it easy to add new entries: write the pattern recognizer
and the closed-form expression separately.

Standard transforms implemented
--------------------------------
| f(t)                  | F(s) = L{f}(s)         |
|-----------------------|------------------------|
| 1                     | 1/s                    |
| t^n                   | n! / s^(n+1)           |
| exp(a·t)              | 1 / (s - a)            |
| sin(ω·t)              | ω / (s² + ω²)          |
| cos(ω·t)              | s / (s² + ω²)          |
| exp(a·t)·sin(ω·t)     | ω / ((s-a)² + ω²)      |
| exp(a·t)·cos(ω·t)     | (s-a) / ((s-a)² + ω²)  |
| t·exp(a·t)            | 1 / (s-a)²             |
| t^n·exp(a·t)          | n! / (s-a)^(n+1)       |
| sinh(a·t)             | a / (s² - a²)          |
| cosh(a·t)             | s / (s² - a²)          |
| t·sin(ω·t)            | 2ωs / (s² + ω²)²       |
| t·cos(ω·t)            | (s² - ω²) / (s² + ω²)² |
| DiracDelta(t)         | 1                      |
| UnitStep(t)           | 1/s                    |
"""

from __future__ import annotations

import math
from typing import Any

from symbolic_ir import (
    ADD,
    DIV,
    MUL,
    POW,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# Helper: check if a node is a rational literal (independent of t)
# ---------------------------------------------------------------------------


def _frac_value(node: IRNode) -> int | None:
    """Return the integer value of an IRInteger node, else None."""
    if isinstance(node, IRInteger):
        return node.value
    return None


def _is_const(node: IRNode, t_sym: IRSymbol) -> bool:
    """Return True if ``node`` does not contain the symbol ``t_sym``.

    A node is constant with respect to t if it is:
    - An IRInteger or IRRational or IRFloat (no symbols at all)
    - An IRSymbol different from t_sym
    - An IRApply where every argument is constant w.r.t. t

    This tells us whether a factor in a product is "pulled out" by
    the linearity rule L{c·f(t)} = c·L{f(t)}.
    """
    if isinstance(node, IRSymbol):
        return node.name != t_sym.name
    if isinstance(node, IRApply):
        return all(_is_const(a, t_sym) for a in node.args)
    # IRInteger, IRRational, IRFloat, IRString — no symbols
    return True


def _extract_coeff_and_fn(
    node: IRNode, t_sym: IRSymbol
) -> tuple[IRNode, IRNode]:
    """Split ``node`` into (constant coefficient, function of t).

    For ``Mul(c, f)`` where ``c`` does not contain t, returns ``(c, f)``.
    For everything else returns ``(IRInteger(1), node)``.

    Examples::

        Mul(3, sin(t))  →  (3, sin(t))
        sin(t)          →  (1, sin(t))
        Mul(t, sin(t))  →  (1, Mul(t, sin(t)))   # t is not const
    """
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Mul"
        and len(node.args) == 2
    ):
        a, b = node.args
        if _is_const(a, t_sym):
            return (a, b)
        if _is_const(b, t_sym):
            return (b, a)
    return (IRInteger(1), node)


def _extract_linear_arg(node: IRNode, t_sym: IRSymbol) -> IRNode:
    """Extract ``a`` from an expression ``a*t`` inside a trig/exp argument.

    For ``Mul(a, t)`` returns ``a``.
    For the bare symbol ``t`` returns ``IRInteger(1)`` (coefficient is 1).
    For anything else returns the whole node (caller should handle that).

    This is used to find ω in ``sin(ω·t)`` or ``a`` in ``exp(a·t)``.
    """
    if isinstance(node, IRSymbol) and node.name == t_sym.name:
        return IRInteger(1)
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Mul"
        and len(node.args) == 2
    ):
        a, b = node.args
        if isinstance(b, IRSymbol) and b.name == t_sym.name and _is_const(a, t_sym):
            return a
        if isinstance(a, IRSymbol) and a.name == t_sym.name and _is_const(b, t_sym):
            return b
    return node


# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _make_pow(base: IRNode, exp_val: int) -> IRNode:
    """Build ``base^exp_val`` as IR, simplifying trivially."""
    if exp_val == 1:
        return base
    return IRApply(POW, (base, IRInteger(exp_val)))


def _make_mul(a: IRNode, b: IRNode) -> IRNode:
    """Build ``a * b`` as IR."""
    return IRApply(MUL, (a, b))


def _make_add(a: IRNode, b: IRNode) -> IRNode:
    """Build ``a + b`` as IR."""
    return IRApply(ADD, (a, b))


def _make_div(num: IRNode, den: IRNode) -> IRNode:
    """Build ``num / den`` as IR."""
    return IRApply(DIV, (num, den))


def _factorial(n: int) -> int:
    """Return n! (factorial of n). n must be non-negative."""
    return math.factorial(n)


# ---------------------------------------------------------------------------
# Pattern recognizers
# ---------------------------------------------------------------------------


def _match_constant_one(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = 1`` (the unit step that's always on).

    Recognizes:
    - ``IRInteger(1)``
    - ``IRRational`` with value 1 (numerator == denominator)
    """
    if isinstance(f, IRInteger) and f.value == 1:
        return {}
    if isinstance(f, IRRational) and f.numer == f.denom:
        return {}
    return None


def _match_power_of_t(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = t^n`` for positive integer n.

    Also matches bare ``t`` (which is ``t^1``).
    """
    # Bare t → t^1
    if isinstance(f, IRSymbol) and f.name == t_sym.name:
        return {"n": 1}
    # Pow(t, n) where n is a positive integer
    if (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Pow"
        and len(f.args) == 2
    ):
        base, exp = f.args
        if (
            isinstance(base, IRSymbol)
            and base.name == t_sym.name
            and isinstance(exp, IRInteger)
            and exp.value >= 1
        ):
            return {"n": exp.value}
    return None


def _match_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = exp(a*t)`` or ``f = exp(t)`` (when a=1).

    Returns ``{"a": a_node}`` where a_node is an IR node for the coefficient.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Exp"
        and len(f.args) == 1
    ):
        return None
    arg = f.args[0]
    # Verify the arg was indeed a*t form (linear in t)
    if isinstance(arg, IRSymbol) and arg.name == t_sym.name:
        return {"a": IRInteger(1)}
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"a": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"a": bb}
    return None


def _match_sin(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = sin(ω*t)`` or ``f = sin(t)`` (ω=1).

    Returns ``{"omega": omega_node}``.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Sin"
        and len(f.args) == 1
    ):
        return None
    arg = f.args[0]
    if isinstance(arg, IRSymbol) and arg.name == t_sym.name:
        return {"omega": IRInteger(1)}
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"omega": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"omega": bb}
    return None


def _match_cos(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = cos(ω*t)`` or ``f = cos(t)`` (ω=1).

    Returns ``{"omega": omega_node}``.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Cos"
        and len(f.args) == 1
    ):
        return None
    arg = f.args[0]
    if isinstance(arg, IRSymbol) and arg.name == t_sym.name:
        return {"omega": IRInteger(1)}
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"omega": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"omega": bb}
    return None


def _match_sinh(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = sinh(a*t)`` or ``f = sinh(t)`` (a=1).

    Returns ``{"a": a_node}``.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Sinh"
        and len(f.args) == 1
    ):
        return None
    arg = f.args[0]
    if isinstance(arg, IRSymbol) and arg.name == t_sym.name:
        return {"a": IRInteger(1)}
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"a": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"a": bb}
    return None


def _match_cosh(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = cosh(a*t)`` or ``f = cosh(t)`` (a=1).

    Returns ``{"a": a_node}``.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Cosh"
        and len(f.args) == 1
    ):
        return None
    arg = f.args[0]
    if isinstance(arg, IRSymbol) and arg.name == t_sym.name:
        return {"a": IRInteger(1)}
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"a": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"a": bb}
    return None


def _match_dirac_delta(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = DiracDelta(t)``.

    The Dirac delta at t=0 has L{δ(t)} = 1.
    """
    if (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "DiracDelta"
        and len(f.args) == 1
        and isinstance(f.args[0], IRSymbol)
        and f.args[0].name == t_sym.name
    ):
        return {}
    return None


def _match_unit_step(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = UnitStep(t)``.

    The unit step (Heaviside function) has L{u(t)} = 1/s.
    """
    if (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "UnitStep"
        and len(f.args) == 1
        and isinstance(f.args[0], IRSymbol)
        and f.args[0].name == t_sym.name
    ):
        return {}
    return None


def _match_exp_sin(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = exp(a*t) * sin(ω*t)`` (in either order in the Mul).

    Returns ``{"a": a_node, "omega": omega_node}``.

    Transform: L{exp(at)·sin(ωt)} = ω / ((s-a)² + ω²)
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    # Try exp * sin and sin * exp
    for exp_node, sin_node in [(left, right), (right, left)]:
        e = _match_exp(exp_node, t_sym)
        s = _match_sin(sin_node, t_sym)
        if e is not None and s is not None:
            return {"a": e["a"], "omega": s["omega"]}
    return None


def _match_exp_cos(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = exp(a*t) * cos(ω*t)`` (in either order in the Mul).

    Returns ``{"a": a_node, "omega": omega_node}``.

    Transform: L{exp(at)·cos(ωt)} = (s-a) / ((s-a)² + ω²)
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    for exp_node, cos_node in [(left, right), (right, left)]:
        e = _match_exp(exp_node, t_sym)
        c = _match_cos(cos_node, t_sym)
        if e is not None and c is not None:
            return {"a": e["a"], "omega": c["omega"]}
    return None


def _match_t_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = t * exp(a*t)`` (in either order in the Mul).

    Returns ``{"a": a_node}``.

    Transform: L{t·exp(at)} = 1 / (s-a)²
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    for t_node, exp_node in [(left, right), (right, left)]:
        if (
            isinstance(t_node, IRSymbol)
            and t_node.name == t_sym.name
        ):
            e = _match_exp(exp_node, t_sym)
            if e is not None:
                return {"a": e["a"]}
    return None


def _match_tn_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = t^n * exp(a*t)`` for integer n ≥ 2 (in either order in the Mul).

    Returns ``{"n": n, "a": a_node}``.

    Transform: L{t^n · exp(at)} = n! / (s-a)^(n+1)
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    for pow_node, exp_node in [(left, right), (right, left)]:
        pow_match = _match_power_of_t(pow_node, t_sym)
        if pow_match is not None and pow_match.get("n", 0) >= 2:
            e = _match_exp(exp_node, t_sym)
            if e is not None:
                return {"n": pow_match["n"], "a": e["a"]}
    return None


def _match_t_sin(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = t * sin(ω*t)`` (in either order in the Mul).

    Returns ``{"omega": omega_node}``.

    Transform: L{t·sin(ωt)} = 2ωs / (s² + ω²)²
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    for t_node, sin_node in [(left, right), (right, left)]:
        if (
            isinstance(t_node, IRSymbol)
            and t_node.name == t_sym.name
        ):
            s = _match_sin(sin_node, t_sym)
            if s is not None:
                return {"omega": s["omega"]}
    return None


def _match_t_cos(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match ``f = t * cos(ω*t)`` (in either order in the Mul).

    Returns ``{"omega": omega_node}``.

    Transform: L{t·cos(ωt)} = (s² - ω²) / (s² + ω²)²
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Mul"
        and len(f.args) == 2
    ):
        return None
    left, right = f.args
    for t_node, cos_node in [(left, right), (right, left)]:
        if (
            isinstance(t_node, IRSymbol)
            and t_node.name == t_sym.name
        ):
            c = _match_cos(cos_node, t_sym)
            if c is not None:
                return {"omega": c["omega"]}
    return None


# ---------------------------------------------------------------------------
# Transform builders (take extracted params, return F(s) IR)
# ---------------------------------------------------------------------------


def _tf_one(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{1} = 1/s."""
    return _make_div(IRInteger(1), s)


def _tf_t_power(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{t^n} = n! / s^(n+1).

    Example: L{t^3} = 6/s^4 because 3! = 6 and the denominator is s^(3+1).
    """
    n: int = params["n"]
    fact = _factorial(n)
    return _make_div(IRInteger(fact), _make_pow(s, n + 1))


def _tf_exp(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{exp(a·t)} = 1/(s-a).

    The denominator is a first-order polynomial in s with the shift equal
    to the growth rate ``a``. For a=0 this is just 1/s (same as L{1}).
    """
    a = params["a"]
    # s - a
    from symbolic_ir import SUB
    denom = IRApply(SUB, (s, a))
    return _make_div(IRInteger(1), denom)


def _tf_sin(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{sin(ωt)} = ω / (s² + ω²).

    The denominator s² + ω² is the characteristic polynomial of a
    pure oscillator. The factor ω in the numerator comes from the
    integration of the imaginary exponential.
    """
    omega = params["omega"]
    # s^2 + omega^2
    denom = _make_add(_make_pow(s, 2), _make_pow(omega, 2))
    return _make_div(omega, denom)


def _tf_cos(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{cos(ωt)} = s / (s² + ω²).

    Like sin but with ``s`` in the numerator rather than ``ω``.
    """
    omega = params["omega"]
    denom = _make_add(_make_pow(s, 2), _make_pow(omega, 2))
    return _make_div(s, denom)


def _tf_exp_sin(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{exp(at)·sin(ωt)} = ω / ((s-a)² + ω²).

    This is the damped sinusoid — a spring-mass-damper system. The shift
    ``a`` moves the pole from the imaginary axis to a complex location.
    """
    from symbolic_ir import SUB

    a = params["a"]
    omega = params["omega"]
    s_minus_a = IRApply(SUB, (s, a))
    # (s-a)^2 + omega^2
    denom = _make_add(_make_pow(s_minus_a, 2), _make_pow(omega, 2))
    return _make_div(omega, denom)


def _tf_exp_cos(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{exp(at)·cos(ωt)} = (s-a) / ((s-a)² + ω²)."""
    from symbolic_ir import SUB

    a = params["a"]
    omega = params["omega"]
    s_minus_a = IRApply(SUB, (s, a))
    denom = _make_add(_make_pow(s_minus_a, 2), _make_pow(omega, 2))
    return _make_div(s_minus_a, denom)


def _tf_t_exp(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{t·exp(at)} = 1/(s-a)².

    The repeated pole at s=a corresponds to the ramp function modulated
    by an exponential.
    """
    from symbolic_ir import SUB

    a = params["a"]
    s_minus_a = IRApply(SUB, (s, a))
    return _make_div(IRInteger(1), _make_pow(s_minus_a, 2))


def _tf_tn_exp(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{t^n · exp(at)} = n! / (s-a)^(n+1)."""
    from symbolic_ir import SUB

    n: int = params["n"]
    a = params["a"]
    fact = _factorial(n)
    s_minus_a = IRApply(SUB, (s, a))
    return _make_div(IRInteger(fact), _make_pow(s_minus_a, n + 1))


def _tf_sinh(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{sinh(at)} = a / (s² - a²).

    Hyperbolic sine arises in systems with real exponentials of both signs:
    sinh(at) = (e^{at} - e^{-at})/2, so the transform is
    1/2 · [1/(s-a) - 1/(s+a)] = a/(s²-a²).
    """
    from symbolic_ir import SUB

    a = params["a"]
    # s^2 - a^2
    denom = IRApply(SUB, (_make_pow(s, 2), _make_pow(a, 2)))
    return _make_div(a, denom)


def _tf_cosh(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{cosh(at)} = s / (s² - a²).

    Similar to sinh but with s in the numerator, analogous to cos vs sin.
    """
    from symbolic_ir import SUB

    a = params["a"]
    denom = IRApply(SUB, (_make_pow(s, 2), _make_pow(a, 2)))
    return _make_div(s, denom)


def _tf_dirac(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{δ(t)} = 1.

    The Dirac delta is the identity for convolution, and its Laplace
    transform is the constant 1 — independent of s.
    """
    return IRInteger(1)


def _tf_unit_step(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{u(t)} = 1/s.

    Same as L{1} for t ≥ 0 (the convention in one-sided Laplace transforms).
    """
    return _make_div(IRInteger(1), s)


def _tf_t_sin(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{t·sin(ωt)} = 2ωs / (s² + ω²)².

    Derivation: differentiate L{sin(ωt)} = ω/(s²+ω²) with respect to (-s):
    -d/ds [ω/(s²+ω²)] = 2ωs/(s²+ω²)².
    """
    omega = params["omega"]
    s2_plus_w2 = _make_add(_make_pow(s, 2), _make_pow(omega, 2))
    # numerator = 2 * omega * s
    num = _make_mul(IRInteger(2), _make_mul(omega, s))
    denom = _make_pow(s2_plus_w2, 2)
    return _make_div(num, denom)


def _tf_t_cos(params: dict[str, Any], s: IRSymbol) -> IRNode:
    """L{t·cos(ωt)} = (s² - ω²) / (s² + ω²)².

    Derivation: -d/ds [s/(s²+ω²)] = (s²-ω²)/(s²+ω²)².
    """
    from symbolic_ir import SUB

    omega = params["omega"]
    s2_plus_w2 = _make_add(_make_pow(s, 2), _make_pow(omega, 2))
    num = IRApply(SUB, (_make_pow(s, 2), _make_pow(omega, 2)))
    denom = _make_pow(s2_plus_w2, 2)
    return _make_div(num, denom)


# ---------------------------------------------------------------------------
# The transform table: ordered from most specific to least specific.
#
# Order matters! More specific patterns (exp*sin, exp*cos, t*exp) must
# come before their component patterns (exp alone, sin alone, t alone).
# Otherwise ``exp(t)*sin(t)`` would match the ``exp`` rule first and
# leave sin(t) as a cofactor.
# ---------------------------------------------------------------------------

TRANSFORM_TABLE: list[
    tuple[
        Any,  # pattern_fn(f, t_sym) -> dict | None
        Any,  # transform_fn(params, s_sym) -> IRNode
    ]
] = [
    # --- Most specific compound patterns first ---
    (_match_exp_sin, _tf_exp_sin),
    (_match_exp_cos, _tf_exp_cos),
    (_match_tn_exp, _tf_tn_exp),   # t^n * exp(at), n>=2 — must precede t*exp
    (_match_t_exp, _tf_t_exp),     # t * exp(at)
    (_match_t_sin, _tf_t_sin),     # t * sin(wt)
    (_match_t_cos, _tf_t_cos),     # t * cos(wt)
    # --- Elementary transforms ---
    (_match_exp, _tf_exp),
    (_match_sin, _tf_sin),
    (_match_cos, _tf_cos),
    (_match_sinh, _tf_sinh),
    (_match_cosh, _tf_cosh),
    # --- Power of t (before constant-1 check, since t^0 = 1 but t is t^1) ---
    (_match_power_of_t, _tf_t_power),
    # --- Special functions ---
    (_match_dirac_delta, _tf_dirac),
    (_match_unit_step, _tf_unit_step),
    # --- Simplest: constant 1 (last, after all other checks) ---
    (_match_constant_one, _tf_one),
]


def table_lookup(
    f: IRNode, t_sym: IRSymbol, s_sym: IRSymbol
) -> IRNode | None:
    """Try every entry in the transform table and return F(s) or None.

    Returns the transformed expression if a pattern matches, or ``None``
    if no pattern matches (caller should return the unevaluated form).
    """
    for pattern_fn, transform_fn in TRANSFORM_TABLE:
        params = pattern_fn(f, t_sym)
        if params is not None:
            return transform_fn(params, s_sym)
    return None
