"""Forward Fourier transform table.

Convention
----------
We use the physics/engineering (angular-frequency) convention::

    F(ω) = ∫_{-∞}^{+∞} f(t) · e^{-iωt} dt

This choice means the pair of transforms is asymmetric:

    Forward:  F(ω) = ∫ f(t) e^{-iωt} dt
    Inverse:  f(t) = (1/2π) ∫ F(ω) e^{+iωt} dω

The factor 1/(2π) lives on the inverse, which is why fourier(1) = 2π·δ(ω)
and ifourier(1) = δ(t).

Design
------
Like the Laplace table in ``cas_laplace.table``, each entry is a pair::

    (pattern_fn, transform_fn)

``pattern_fn(f, t_sym)`` → dict of extracted parameters, or None.
``transform_fn(params, omega_sym)`` → IR for F(ω).

Linearity
---------
Before hitting the table, the driver decomposes ``f`` by linearity:

    fourier(c·f, t, ω) = c · fourier(f, t, ω)
    fourier(f + g, t, ω) = fourier(f, t, ω) + fourier(g, t, ω)

Standard transforms implemented
--------------------------------
| f(t)                  | F(ω)                                         |
|-----------------------|----------------------------------------------|
| δ(t)                  | 1                                            |
| 1                     | 2π·δ(ω)                                      |
| exp(-a·t) (causal)    | 1/(a + i·ω)         [a implicit positive]    |
| exp(i·a·t)            | 2π·δ(ω - a)                                 |
| sin(ω₀·t)             | i·π·(δ(ω+ω₀) - δ(ω-ω₀))                    |
| cos(ω₀·t)             | π·(δ(ω-ω₀) + δ(ω+ω₀))                      |
| exp(-a·t²) (Gaussian) | √(π/a) · exp(-ω²/(4a))                      |
| t·exp(-a·t) (causal)  | 1/(a + i·ω)²                                 |
"""

from __future__ import annotations

from typing import Any

from symbolic_ir import (
    ADD,
    DIV,
    EXP,
    MUL,
    NEG,
    POW,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# The Dirac delta head — imported from symbolic_ir where it lives.
_DIRAC_DELTA = IRSymbol("DiracDelta")

# The MACSYMA imaginary unit and pi constants.
# %i is the pre-bound imaginary unit (ImaginaryUnit symbol) in the VM.
# We use ImaginaryUnit to be consistent with cas_complex.
_IMAG = IRSymbol("ImaginaryUnit")
_PI = IRSymbol("%pi")


# ---------------------------------------------------------------------------
# Shared helpers — these are adapted from cas_laplace.table for consistency.
# ---------------------------------------------------------------------------


def _is_const(node: IRNode, t_sym: IRSymbol) -> bool:
    """Return True if ``node`` does not depend on the symbol ``t_sym``.

    A node is constant w.r.t. t when:
    - It is a numeric literal (IRInteger, IRRational, IRFloat).
    - It is an IRSymbol with a different name than t_sym.
    - It is an IRApply where *every* argument is constant w.r.t. t.

    This is the same predicate used by the Laplace table.
    """
    if isinstance(node, IRSymbol):
        return node.name != t_sym.name
    if isinstance(node, IRApply):
        return all(_is_const(a, t_sym) for a in node.args)
    return True  # IRInteger, IRRational, IRFloat, IRString


def _make_add(a: IRNode, b: IRNode) -> IRNode:
    """Build Add(a, b)."""
    return IRApply(ADD, (a, b))


def _make_sub(a: IRNode, b: IRNode) -> IRNode:
    """Build Sub(a, b)."""
    return IRApply(SUB, (a, b))


def _make_mul(a: IRNode, b: IRNode) -> IRNode:
    """Build Mul(a, b)."""
    return IRApply(MUL, (a, b))


def _make_div(a: IRNode, b: IRNode) -> IRNode:
    """Build Div(a, b)."""
    return IRApply(DIV, (a, b))


def _make_neg(a: IRNode) -> IRNode:
    """Build Neg(a)."""
    return IRApply(NEG, (a,))


def _make_pow(base: IRNode, exp: IRNode) -> IRNode:
    """Build Pow(base, exp)."""
    return IRApply(POW, (base, exp))


def _make_exp(arg: IRNode) -> IRNode:
    """Build Exp(arg)."""
    return IRApply(EXP, (arg,))


def _make_sqrt(arg: IRNode) -> IRNode:
    """Build Sqrt(arg)."""
    return IRApply(SQRT, (arg,))


def _make_delta(arg: IRNode) -> IRNode:
    """Build DiracDelta(arg)."""
    return IRApply(_DIRAC_DELTA, (arg,))


def _make_two_pi_delta(arg: IRNode) -> IRNode:
    """Build 2·π·DiracDelta(arg) — the FT of a complex exponential."""
    return _make_mul(
        _make_mul(IRInteger(2), _PI),
        _make_delta(arg),
    )


# ---------------------------------------------------------------------------
# Pattern recognizers
# ---------------------------------------------------------------------------


def _match_dirac_delta(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = DiracDelta(t).

    The Dirac delta satisfies: ∫ δ(t) e^{-iωt} dt = e^{-iω·0} = 1.

    Returns {} — no parameters needed.
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


def _match_constant_one(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = 1 (the constant function).

    FT{1} = 2π·δ(ω). Recognised forms:
    - IRInteger(1)
    - IRRational(n, n) — i.e. value 1 in rational form.
    """
    if isinstance(f, IRInteger) and f.value == 1:
        return {}
    if isinstance(f, IRRational) and f.numer == f.denom:
        return {}
    return None


def _match_causal_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = exp(-a·t) — a causal decaying exponential.

    We match ``Exp(Neg(Mul(a, t)))`` or ``Exp(Neg(t))`` (the a=1 case).
    The sign must be negative so that a > 0 corresponds to a decaying
    (i.e. absolutely integrable) waveform — the causal Laplace convention.

    FT{exp(-a·t)·u(t)} = 1/(a + i·ω)  for Re(a) > 0.

    Returns {"a": a_node}.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Exp"
        and len(f.args) == 1
    ):
        return None

    arg = f.args[0]

    # Case: Exp(Neg(t)) → a = 1
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Neg"
        and len(arg.args) == 1
        and isinstance(arg.args[0], IRSymbol)
        and arg.args[0].name == t_sym.name
    ):
        return {"a": IRInteger(1)}

    # Case: Exp(Neg(Mul(a, t))) or Exp(Neg(Mul(t, a)))
    if not (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Neg"
        and len(arg.args) == 1
    ):
        return None

    inner = arg.args[0]  # the argument of the Neg

    if isinstance(inner, IRSymbol) and inner.name == t_sym.name:
        return {"a": IRInteger(1)}

    if (
        isinstance(inner, IRApply)
        and isinstance(inner.head, IRSymbol)
        and inner.head.name == "Mul"
        and len(inner.args) == 2
    ):
        aa, bb = inner.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"a": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"a": bb}

    return None


def _match_complex_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = exp(i·a·t) — a pure complex exponential.

    We match ``Exp(Mul(ImaginaryUnit, a, t))`` or simplified forms.
    The imaginary unit may be spelled ``ImaginaryUnit``.

    FT{e^{iωat}} = 2π·δ(ω - a).

    Returns {"a": a_node}.

    Recognised patterns:
    - Exp(Mul(ImaginaryUnit, t))              → a = 1
    - Exp(Mul(ImaginaryUnit, Mul(a, t)))      → a = a
    - Exp(Mul(Mul(ImaginaryUnit, a), t))      → a = a
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Exp"
        and len(f.args) == 1
    ):
        return None

    arg = f.args[0]

    def _is_imag_unit(n: IRNode) -> bool:
        return isinstance(n, IRSymbol) and n.name == "ImaginaryUnit"

    # Pattern: Exp(Mul(ImaginaryUnit, t))
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        a_node, b_node = arg.args
        # ImaginaryUnit * t
        b_is_t = isinstance(b_node, IRSymbol) and b_node.name == t_sym.name
        a_is_t = isinstance(a_node, IRSymbol) and a_node.name == t_sym.name
        if _is_imag_unit(a_node) and b_is_t:
            return {"a": IRInteger(1)}
        # t * ImaginaryUnit (reversed)
        if _is_imag_unit(b_node) and a_is_t:
            return {"a": IRInteger(1)}

        # ImaginaryUnit * (a * t) or ImaginaryUnit * (t * a)
        if _is_imag_unit(a_node) and isinstance(b_node, IRApply):
            inner = b_node
            if (
                isinstance(inner.head, IRSymbol)
                and inner.head.name == "Mul"
                and len(inner.args) == 2
            ):
                ia, ib = inner.args
                ib_is_t = isinstance(ib, IRSymbol) and ib.name == t_sym.name
                ia_is_t = isinstance(ia, IRSymbol) and ia.name == t_sym.name
                if ib_is_t and _is_const(ia, t_sym):
                    return {"a": ia}
                if ia_is_t and _is_const(ib, t_sym):
                    return {"a": ib}

        # (ImaginaryUnit * a) * t  — the Mul(Mul(%i,a), t) form
        if _is_imag_unit(b_node) and isinstance(a_node, IRApply):
            inner = a_node
            if (
                isinstance(inner.head, IRSymbol)
                and inner.head.name == "Mul"
                and len(inner.args) == 2
            ):
                ia, ib = inner.args
                ib_is_t = isinstance(ib, IRSymbol) and ib.name == t_sym.name
                ia_is_t = isinstance(ia, IRSymbol) and ia.name == t_sym.name
                if ib_is_t and _is_const(ia, t_sym):
                    return {"a": ia}
                if ia_is_t and _is_const(ib, t_sym):
                    return {"a": ib}

        # Mul(%i*a, t) — first arg is Mul(%i, a)
        if (
            isinstance(a_node, IRApply)
            and isinstance(a_node.head, IRSymbol)
            and a_node.head.name == "Mul"
            and len(a_node.args) == 2
            and _is_imag_unit(a_node.args[0])
            and _is_const(a_node.args[1], t_sym)
            and isinstance(b_node, IRSymbol)
            and b_node.name == t_sym.name
        ):
            return {"a": a_node.args[1]}

    return None


def _match_sin(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = sin(ω₀·t) or f = sin(t).

    FT{sin(ω₀t)} = i·π·(δ(ω + ω₀) - δ(ω - ω₀))

    Returns {"omega0": omega0_node}.
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
        return {"omega0": IRInteger(1)}

    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"omega0": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"omega0": bb}

    return None


def _match_cos(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = cos(ω₀·t) or f = cos(t).

    FT{cos(ω₀t)} = π·(δ(ω - ω₀) + δ(ω + ω₀))

    Returns {"omega0": omega0_node}.
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
        return {"omega0": IRInteger(1)}

    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        aa, bb = arg.args
        if isinstance(bb, IRSymbol) and bb.name == t_sym.name and _is_const(aa, t_sym):
            return {"omega0": aa}
        if isinstance(aa, IRSymbol) and aa.name == t_sym.name and _is_const(bb, t_sym):
            return {"omega0": bb}

    return None


def _match_gaussian(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = exp(-a·t²) — a Gaussian pulse.

    We recognise ``Exp(Neg(Mul(a, Pow(t, 2))))`` and variants.

    FT{exp(-a·t²)} = √(π/a) · exp(-ω²/(4a))

    Returns {"a": a_node}.
    """
    if not (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Exp"
        and len(f.args) == 1
    ):
        return None

    arg = f.args[0]

    # Need Neg(something involving t^2)
    if not (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Neg"
        and len(arg.args) == 1
    ):
        return None

    inner = arg.args[0]  # what's inside the Neg

    def _is_t_squared(node: IRNode) -> bool:
        """Check if node is Pow(t, 2) or t (but that would be t^1, skip)."""
        return (
            isinstance(node, IRApply)
            and isinstance(node.head, IRSymbol)
            and node.head.name == "Pow"
            and len(node.args) == 2
            and isinstance(node.args[0], IRSymbol)
            and node.args[0].name == t_sym.name
            and isinstance(node.args[1], IRInteger)
            and node.args[1].value == 2
        )

    # Case: Exp(Neg(Pow(t, 2))) → a = 1
    if _is_t_squared(inner):
        return {"a": IRInteger(1)}

    # Case: Exp(Neg(Mul(a, Pow(t, 2)))) or Exp(Neg(Mul(Pow(t,2), a)))
    if (
        isinstance(inner, IRApply)
        and isinstance(inner.head, IRSymbol)
        and inner.head.name == "Mul"
        and len(inner.args) == 2
    ):
        aa, bb = inner.args
        if _is_t_squared(bb) and _is_const(aa, t_sym):
            return {"a": aa}
        if _is_t_squared(aa) and _is_const(bb, t_sym):
            return {"a": bb}

    return None


def _match_t_exp(
    f: IRNode, t_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match f = t·exp(-a·t) — ramp times a causal decaying exponential.

    FT{t·exp(-a·t)·u(t)} = 1/(a + i·ω)²  for Re(a) > 0.

    Returns {"a": a_node}.
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
        if isinstance(t_node, IRSymbol) and t_node.name == t_sym.name:
            result = _match_causal_exp(exp_node, t_sym)
            if result is not None:
                return result

    return None


# ---------------------------------------------------------------------------
# Transform builders
# ---------------------------------------------------------------------------


def _tf_dirac(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{δ(t)} = 1.

    The Dirac delta integrates to 1 (by the sifting property):
        ∫ δ(t) e^{-iωt} dt = e^{-iω·0} = 1.
    """
    return IRInteger(1)


def _tf_constant_one(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{1} = 2π·δ(ω).

    Dual of the inverse: ifourier(1) = δ(t).
    The Fourier transform of the constant 1 is a delta function in
    the frequency domain — it has all of its energy at ω = 0.

    Result: Mul(Mul(2, %pi), DiracDelta(ω))
    """
    return _make_two_pi_delta(omega)


def _tf_causal_exp(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{exp(-a·t)·u(t)} = 1/(a + i·ω).

    This is the one-sided Laplace transform evaluated on the imaginary
    axis: s = iω. The denominator ``a + iω`` has a pole at ω = ia
    in the upper half of the complex ω-plane (for a > 0), confirming
    absolute integrability of the causal exponential.

    Result: Div(1, Add(a, Mul(ImaginaryUnit, ω)))
    """
    a = params["a"]
    i_omega = _make_mul(_IMAG, omega)
    denom = _make_add(a, i_omega)
    return _make_div(IRInteger(1), denom)


def _tf_complex_exp(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{e^{iat}} = 2π·δ(ω - a).

    A pure complex exponential at frequency ``a`` transforms to a delta
    function shifted to ω = a. This follows from the shift property of
    the delta: ∫ e^{iat} e^{-iωt} dt = ∫ e^{-i(ω-a)t} dt = 2π·δ(ω-a).

    Result: Mul(Mul(2, %pi), DiracDelta(Sub(ω, a)))
    """
    a = params["a"]
    return _make_two_pi_delta(_make_sub(omega, a))


def _tf_sin(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{sin(ω₀t)} = i·π·(δ(ω + ω₀) - δ(ω - ω₀)).

    Derivation via Euler's formula:
        sin(ω₀t) = (e^{iω₀t} - e^{-iω₀t}) / (2i)

    Taking the FT of each exponential (using the complex-exp formula):
        FT = (1/2i) [2π·δ(ω - ω₀) - 2π·δ(ω + ω₀)]
           = π/i · [δ(ω - ω₀) - δ(ω + ω₀)]
           = iπ · [δ(ω + ω₀) - δ(ω - ω₀)]    (since 1/i = -i)

    Result: Mul(ImaginaryUnit, Mul(%pi, Sub(DiracDelta(Add(ω, ω₀)),
                                            DiracDelta(Sub(ω, ω₀)))))
    """
    omega0 = params["omega0"]
    delta_plus = _make_delta(_make_add(omega, omega0))   # δ(ω + ω₀)
    delta_minus = _make_delta(_make_sub(omega, omega0))  # δ(ω - ω₀)
    diff = _make_sub(delta_plus, delta_minus)
    pi_diff = _make_mul(_PI, diff)
    return _make_mul(_IMAG, pi_diff)


def _tf_cos(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{cos(ω₀t)} = π·(δ(ω - ω₀) + δ(ω + ω₀)).

    Derivation via Euler's formula:
        cos(ω₀t) = (e^{iω₀t} + e^{-iω₀t}) / 2

    Taking the FT of each exponential:
        FT = (1/2) [2π·δ(ω - ω₀) + 2π·δ(ω + ω₀)]
           = π · [δ(ω - ω₀) + δ(ω + ω₀)]

    Result: Mul(%pi, Add(DiracDelta(Sub(ω, ω₀)), DiracDelta(Add(ω, ω₀))))
    """
    omega0 = params["omega0"]
    delta_minus = _make_delta(_make_sub(omega, omega0))  # δ(ω - ω₀)
    delta_plus = _make_delta(_make_add(omega, omega0))   # δ(ω + ω₀)
    return _make_mul(_PI, _make_add(delta_minus, delta_plus))


def _tf_gaussian(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{exp(-a·t²)} = √(π/a) · exp(-ω²/(4a)).

    The Gaussian is a fixed point of the Fourier transform (up to
    scaling). The derivation uses completing the square in the exponent:

        ∫ exp(-at² - iωt) dt  =  exp(-ω²/4a) · ∫ exp(-a(t + iω/2a)²) dt

    The remaining Gaussian integral equals √(π/a), giving the result.

    Result: Mul(Sqrt(Div(%pi, a)), Exp(Neg(Div(Pow(ω, 2), Mul(4, a)))))
    """
    a = params["a"]
    # √(π/a)
    scale = _make_sqrt(_make_div(_PI, a))
    # -ω²/(4a)
    omega_sq = _make_pow(omega, IRInteger(2))
    four_a = _make_mul(IRInteger(4), a)
    exponent = _make_neg(_make_div(omega_sq, four_a))
    return _make_mul(scale, _make_exp(exponent))


def _tf_t_exp(params: dict[str, Any], omega: IRSymbol) -> IRNode:
    """FT{t·exp(-a·t)·u(t)} = 1/(a + i·ω)².

    Derivation: differentiation in the time domain corresponds to
    multiplication by iω in the frequency domain. Conversely,
    multiplication by t corresponds to differentiating -i · F(ω)
    with respect to ω:

        FT{t·f(t)} = i · dF/dω

    Applying to FT{exp(-at)} = 1/(a+iω) and differentiating once:
        d/dω [1/(a+iω)] = -i/(a+iω)²
        FT{t·exp(-at)} = i · (-i/(a+iω)²) = 1/(a+iω)²

    Result: Div(1, Pow(Add(a, Mul(ImaginaryUnit, ω)), 2))
    """
    a = params["a"]
    i_omega = _make_mul(_IMAG, omega)
    denom_base = _make_add(a, i_omega)
    denom = _make_pow(denom_base, IRInteger(2))
    return _make_div(IRInteger(1), denom)


# ---------------------------------------------------------------------------
# Linearity decomposition helpers
# ---------------------------------------------------------------------------


def _extract_scalar_and_fn(
    node: IRNode, t_sym: IRSymbol
) -> tuple[IRNode, IRNode]:
    """Split ``node`` into (constant, function-of-t) using linearity.

    For Mul(c, f) where c is constant w.r.t. t, returns (c, f).
    For anything else returns (1, node).

    This implements the linearity rule:
        FT{c·f(t)} = c · FT{f(t)}    for c independent of t.
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


# ---------------------------------------------------------------------------
# The Fourier transform table
# ---------------------------------------------------------------------------

# More specific patterns must come before less specific ones.
# t·exp(-at) must precede plain exp(-at) so the combined form
# matches before the exp-only pattern strips the t away.
FOURIER_TABLE: list[tuple[Any, Any]] = [
    # --- Most specific compound patterns ---
    (_match_t_exp,         _tf_t_exp),        # t·exp(-at)
    (_match_gaussian,      _tf_gaussian),      # exp(-at²)
    (_match_complex_exp,   _tf_complex_exp),   # exp(i·a·t)
    (_match_causal_exp,    _tf_causal_exp),    # exp(-a·t)
    # --- Trig ---
    (_match_sin,           _tf_sin),           # sin(ω₀t)
    (_match_cos,           _tf_cos),           # cos(ω₀t)
    # --- Special functions ---
    (_match_dirac_delta,   _tf_dirac),         # δ(t)
    # --- Constant last ---
    (_match_constant_one,  _tf_constant_one),  # 1
]


def table_lookup(
    f: IRNode, t_sym: IRSymbol, omega_sym: IRSymbol
) -> IRNode | None:
    """Try every entry in the Fourier table and return F(ω) or None.

    Returns the transformed expression if a pattern matches, or None
    if no pattern matches (caller should wrap in Fourier(f, t, ω) and
    leave unevaluated).
    """
    for pattern_fn, transform_fn in FOURIER_TABLE:
        params = pattern_fn(f, t_sym)
        if params is not None:
            return transform_fn(params, omega_sym)
    return None


# ---------------------------------------------------------------------------
# Public entry point: fourier_transform
# ---------------------------------------------------------------------------


def fourier_transform(
    f: IRNode, t_sym: IRSymbol, omega_sym: IRSymbol
) -> IRNode:
    """Compute the symbolic Fourier transform of f with respect to t_sym.

    Convention: F(ω) = ∫ f(t) e^{-iωt} dt

    Algorithm
    ---------
    1. **Table lookup**: try the forward Fourier table directly.
    2. **Scalar linearity**: strip constant factors out of Mul(c, g) and
       transform g, then re-multiply by c.
    3. **Sum linearity**: decompose Add(f1, f2) and transform each term.
    4. **Fallback**: return Fourier(f, t_sym, omega_sym) unevaluated.

    Parameters
    ----------
    f:
        The time-domain expression.
    t_sym:
        The integration variable (must be an IRSymbol).
    omega_sym:
        The frequency variable (must be an IRSymbol).

    Returns
    -------
    IRNode
        The Fourier transform F(ω), or the unevaluated form.
    """
    from cas_fourier.heads import FOURIER as _FOURIER

    # --- Step 1: direct table lookup ----------------------------------------
    result = table_lookup(f, t_sym, omega_sym)
    if result is not None:
        return result

    # --- Step 2: scalar linearity — Mul(c, g) → c·F(g) ---------------------
    c, g = _extract_scalar_and_fn(f, t_sym)
    if not (isinstance(c, IRInteger) and c.value == 1):
        # c is a non-trivial scalar; transform g alone
        ft_g = fourier_transform(g, t_sym, omega_sym)
        return _make_mul(c, ft_g)

    # --- Step 3: sum linearity — Add(f1, f2) → F(f1) + F(f2) ---------------
    if (
        isinstance(f, IRApply)
        and isinstance(f.head, IRSymbol)
        and f.head.name == "Add"
        and len(f.args) == 2
    ):
        f1, f2 = f.args
        ft1 = fourier_transform(f1, t_sym, omega_sym)
        ft2 = fourier_transform(f2, t_sym, omega_sym)
        return _make_add(ft1, ft2)

    # --- Step 4: return unevaluated -----------------------------------------
    return IRApply(_FOURIER, (f, t_sym, omega_sym))
