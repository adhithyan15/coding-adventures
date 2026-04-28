"""Inverse Fourier transform table.

Convention
----------
The inverse Fourier transform uses the factor-of-2π convention that is
symmetric with the forward table::

    f(t) = (1/2π) ∫_{-∞}^{+∞} F(ω) · e^{+iωt} dω

This is the standard physics/engineering form. The factor (1/2π) means:

    ifourier(1, ω, t)         = δ(t)
    ifourier(2π·δ(ω), ω, t)  = 1
    ifourier(1/(a+iω), ω, t) = exp(-at)·u(t)  for Re(a)>0

Design
------
The inverse table implements **exact pattern matching** against the
outputs of the forward table. This means the round-trip property holds
for all table entries::

    ifourier(fourier(f, t, ω), ω, t) ≈ f

for each ``f`` in the forward table. Patterns that don't match return
the unevaluated ``IFourier(F, ω, t)`` form.

Standard inverse transforms implemented
-----------------------------------------
| F(ω)                          | f(t) = ifourier(F, ω, t)           |
|-------------------------------|-------------------------------------|
| 1                             | δ(t)                                |
| DiracDelta(ω)                 | 1/(2π)                              |
| 2π·δ(ω - a)  (from FT{e^iat})| exp(i·a·t)                          |
| Mul(2π, DiracDelta(ω))        | 1   (inv of FT{1})                  |
| 1/(a + i·ω)                   | exp(-at)·u(t)                       |
| 1/(a + i·ω)²                  | t·exp(-at)·u(t)                     |
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
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

# Shared heads.
_DIRAC_DELTA = IRSymbol("DiracDelta")
_UNIT_STEP = IRSymbol("UnitStep")
_IMAG = IRSymbol("ImaginaryUnit")
_PI = IRSymbol("%pi")


# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _make_add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _make_sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _make_mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _make_div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _make_neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _make_pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _make_exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _make_delta(arg: IRNode) -> IRNode:
    return IRApply(_DIRAC_DELTA, (arg,))


def _make_unit_step(arg: IRNode) -> IRNode:
    return IRApply(_UNIT_STEP, (arg,))


def _is_imag_unit(n: IRNode) -> bool:
    """Return True if n is the ImaginaryUnit symbol."""
    return isinstance(n, IRSymbol) and n.name == "ImaginaryUnit"


def _is_pi(n: IRNode) -> bool:
    """Return True if n is the %pi constant."""
    return isinstance(n, IRSymbol) and n.name == "%pi"


def _is_two_pi(n: IRNode) -> bool:
    """Return True if n represents 2·π, i.e. Mul(2, %pi)."""
    if not (
        isinstance(n, IRApply)
        and isinstance(n.head, IRSymbol)
        and n.head.name == "Mul"
        and len(n.args) == 2
    ):
        return False
    a, b = n.args
    if isinstance(a, IRInteger) and a.value == 2 and _is_pi(b):
        return True
    return isinstance(b, IRInteger) and b.value == 2 and _is_pi(a)


# ---------------------------------------------------------------------------
# Pattern recognizers for the inverse table
# ---------------------------------------------------------------------------


def _imatch_one(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = 1. ifourier(1) = δ(t)."""
    if isinstance(F, IRInteger) and F.value == 1:
        return {}
    return None


def _imatch_dirac_omega(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = DiracDelta(ω). ifourier(δ(ω)) = 1/(2π)."""
    if (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "DiracDelta"
        and len(F.args) == 1
        and isinstance(F.args[0], IRSymbol)
        and F.args[0].name == omega_sym.name
    ):
        return {}
    return None


def _imatch_two_pi_dirac(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = Mul(Mul(2, %pi), DiracDelta(ω)) — inverse of FT{1}.

    ifourier(2π·δ(ω)) = 1.

    This is the exact output structure produced by _tf_constant_one.
    """
    # Shape: Mul(Mul(2, %pi), DiracDelta(ω))
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Mul"
        and len(F.args) == 2
    ):
        return None

    outer_left, outer_right = F.args

    # Outer structure: Mul(<2pi_part>, DiracDelta(ω))
    for two_pi_part, delta_part in [
        (outer_left, outer_right),
        (outer_right, outer_left),
    ]:
        if (
            _is_two_pi(two_pi_part)
            and isinstance(delta_part, IRApply)
            and isinstance(delta_part.head, IRSymbol)
            and delta_part.head.name == "DiracDelta"
            and len(delta_part.args) == 1
            and isinstance(delta_part.args[0], IRSymbol)
            and delta_part.args[0].name == omega_sym.name
        ):
            return {}

    return None


def _imatch_two_pi_dirac_shifted(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = Mul(Mul(2, %pi), DiracDelta(Sub(ω, a))) — inverse of FT{e^{iat}}.

    ifourier(2π·δ(ω - a)) = exp(i·a·t).

    Returns {"a": a_node}.
    """
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Mul"
        and len(F.args) == 2
    ):
        return None

    outer_left, outer_right = F.args

    for two_pi_part, delta_part in [
        (outer_left, outer_right),
        (outer_right, outer_left),
    ]:
        if not _is_two_pi(two_pi_part):
            continue
        if not (
            isinstance(delta_part, IRApply)
            and isinstance(delta_part.head, IRSymbol)
            and delta_part.head.name == "DiracDelta"
            and len(delta_part.args) == 1
        ):
            continue

        inner = delta_part.args[0]

        # Sub(ω, a) — shifted delta
        if (
            isinstance(inner, IRApply)
            and isinstance(inner.head, IRSymbol)
            and inner.head.name == "Sub"
            and len(inner.args) == 2
            and isinstance(inner.args[0], IRSymbol)
            and inner.args[0].name == omega_sym.name
        ):
            return {"a": inner.args[1]}

    return None


def _imatch_causal_exp_denom(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = Div(1, Add(a, Mul(ImaginaryUnit, ω))).

    This is the exact output of _tf_causal_exp.
    ifourier(1/(a + i·ω)) = exp(-a·t)·u(t).

    Returns {"a": a_node}.
    """
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
    ):
        return None

    num, denom = F.args

    if not (isinstance(num, IRInteger) and num.value == 1):
        return None

    # Denom must be Add(a, Mul(ImaginaryUnit, ω))
    if not (
        isinstance(denom, IRApply)
        and isinstance(denom.head, IRSymbol)
        and denom.head.name == "Add"
        and len(denom.args) == 2
    ):
        return None

    da, db = denom.args

    # Look for Mul(ImaginaryUnit, ω) in either position
    def _is_i_omega(node: IRNode) -> bool:
        return (
            isinstance(node, IRApply)
            and isinstance(node.head, IRSymbol)
            and node.head.name == "Mul"
            and len(node.args) == 2
            and _is_imag_unit(node.args[0])
            and isinstance(node.args[1], IRSymbol)
            and node.args[1].name == omega_sym.name
        )

    if _is_i_omega(db):
        return {"a": da}
    if _is_i_omega(da):
        return {"a": db}

    return None


def _imatch_t_exp_denom(
    F: IRNode, omega_sym: IRSymbol
) -> dict[str, Any] | None:
    """Match F = Div(1, Pow(Add(a, Mul(ImaginaryUnit, ω)), 2)).

    This is the exact output of _tf_t_exp.
    ifourier(1/(a + i·ω)²) = t·exp(-a·t)·u(t).

    Returns {"a": a_node}.
    """
    if not (
        isinstance(F, IRApply)
        and isinstance(F.head, IRSymbol)
        and F.head.name == "Div"
        and len(F.args) == 2
    ):
        return None

    num, denom = F.args

    if not (isinstance(num, IRInteger) and num.value == 1):
        return None

    # Denom must be Pow(<add_expr>, 2)
    if not (
        isinstance(denom, IRApply)
        and isinstance(denom.head, IRSymbol)
        and denom.head.name == "Pow"
        and len(denom.args) == 2
        and isinstance(denom.args[1], IRInteger)
        and denom.args[1].value == 2
    ):
        return None

    add_expr = denom.args[0]

    # add_expr must match the causal-exp denominator
    dummy = IRApply(
        IRSymbol("Div"), (IRInteger(1), add_expr)
    )
    result = _imatch_causal_exp_denom(dummy, omega_sym)
    return result


# ---------------------------------------------------------------------------
# Inverse transform builders
# ---------------------------------------------------------------------------


def _itf_one(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(1) = δ(t).

    This is the dual of fourier(δ(t)) = 1 under the 1/(2π) convention.
    """
    return IRApply(_DIRAC_DELTA, (t_sym,))


def _itf_dirac_omega(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(δ(ω)) = 1/(2π).

    Derivation: (1/2π) ∫ δ(ω) e^{iωt} dω = (1/2π) · e^{i·0·t} = 1/(2π).

    Result: Div(1, Mul(2, %pi))
    """
    two_pi = _make_mul(IRInteger(2), _PI)
    return _make_div(IRInteger(1), two_pi)


def _itf_two_pi_dirac(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(2π·δ(ω)) = 1.

    This undoes FT{1} = 2π·δ(ω).
    """
    return IRInteger(1)


def _itf_two_pi_dirac_shifted(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(2π·δ(ω - a)) = exp(i·a·t).

    Undoes FT{e^{iat}} = 2π·δ(ω - a).

    Result: Exp(Mul(Mul(ImaginaryUnit, a), t))
    """
    a = params["a"]
    # i·a·t expressed as Mul(Mul(ImaginaryUnit, a), t)
    i_a = _make_mul(_IMAG, a)
    return _make_exp(_make_mul(i_a, t_sym))


def _itf_causal_exp_denom(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(1/(a + i·ω)) = exp(-a·t)·u(t).

    Undoes FT{exp(-at)·u(t)} = 1/(a + iω).

    Result: Mul(Exp(Neg(Mul(a, t))), UnitStep(t))
    """
    a = params["a"]
    exp_part = _make_exp(_make_neg(_make_mul(a, t_sym)))
    step_part = _make_unit_step(t_sym)
    return _make_mul(exp_part, step_part)


def _itf_t_exp_denom(params: dict[str, Any], t_sym: IRSymbol) -> IRNode:
    """ifourier(1/(a + i·ω)²) = t·exp(-a·t)·u(t).

    Undoes FT{t·exp(-at)·u(t)} = 1/(a + iω)².

    Result: Mul(t, Mul(Exp(Neg(Mul(a, t))), UnitStep(t)))
    """
    a = params["a"]
    exp_part = _make_exp(_make_neg(_make_mul(a, t_sym)))
    step_part = _make_unit_step(t_sym)
    exp_step = _make_mul(exp_part, step_part)
    return _make_mul(t_sym, exp_step)


# ---------------------------------------------------------------------------
# Inverse Fourier table (more specific → less specific)
# ---------------------------------------------------------------------------

IFOURIER_TABLE: list[tuple[Any, Any]] = [
    # --- Exact shape matches from forward table outputs (most specific) ---
    (_imatch_t_exp_denom,          _itf_t_exp_denom),         # 1/(a+iω)²
    (_imatch_causal_exp_denom,     _itf_causal_exp_denom),    # 1/(a+iω)
    (_imatch_two_pi_dirac_shifted, _itf_two_pi_dirac_shifted), # 2π·δ(ω-a)
    (_imatch_two_pi_dirac,         _itf_two_pi_dirac),         # 2π·δ(ω)
    (_imatch_dirac_omega,          _itf_dirac_omega),          # δ(ω)
    # --- Simplest ---
    (_imatch_one,                  _itf_one),                  # 1
]


def ifourier_transform(
    F: IRNode, omega_sym: IRSymbol, t_sym: IRSymbol
) -> IRNode:
    """Compute the symbolic inverse Fourier transform of F w.r.t. omega_sym.

    Convention: f(t) = (1/2π) ∫ F(ω) e^{+iωt} dω

    Algorithm
    ---------
    1. **Table lookup**: try every pattern in IFOURIER_TABLE.
    2. **Fallback**: return IFourier(F, omega_sym, t_sym) unevaluated.

    Parameters
    ----------
    F:
        The frequency-domain expression.
    omega_sym:
        The frequency variable (must be an IRSymbol).
    t_sym:
        The time variable (must be an IRSymbol).

    Returns
    -------
    IRNode
        The inverse Fourier transform f(t), or the unevaluated form.
    """
    from cas_fourier.heads import IFOURIER as _IFOURIER

    for pattern_fn, transform_fn in IFOURIER_TABLE:
        params = pattern_fn(F, omega_sym)
        if params is not None:
            return transform_fn(params, t_sym)

    # Fallback: return unevaluated.
    return IRApply(_IFOURIER, (F, omega_sym, t_sym))
