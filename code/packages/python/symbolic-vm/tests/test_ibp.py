"""Track E1 — generic tabular integration-by-parts fallback tests.

These exercise the ``ibp_tabular`` module that runs **after** every
shape-specific integration handler has returned ``None``.  The test
plan:

1. **Acceptance #1** — ``∫ x·sin(x) dx`` closes to ``sin(x) − x·cos(x)``.
2. **Acceptance #2** — ``∫ x²·eˣ dx`` closes to ``(x² − 2x + 2)·eˣ``.
3. **Higher-degree polynomial × trig** — ``∫ x³·cos(x) dx``.
4. **Gap closed by the new fallback** — ``∫ x·sin(x)·cos(x) dx``, a
   three-factor Mul that the per-shape handlers don't recognise but
   tabular IBP closes via the partition ``u = x, w = sin(x)·cos(x)``.
5. **Fallthrough on no viable split** — ``∫ 1/x dx`` returns ``log(x)``
   via the existing handler; tabular IBP doesn't fire (no Mul).
6. **Fallthrough on un-integrable factor** — ``∫ sin(x²) dx`` is a
   Fresnel integral; tabular IBP can't help and the special-function
   handler returns the Fresnel form; the engine does **not** emit a
   bogus elementary closed form.

All correctness checks use **numeric differentiation of the returned
antiderivative** vs. the original integrand.  This avoids hard-coding
the exact algebraic form of the answer — any equivalent expression
(``Sub(Sin(x), Mul(x, Cos(x)))`` vs. ``Add(Sin(x), Neg(Mul(x, Cos(x))))``
vs. anything the simplifier picks tomorrow) is accepted as long as
the symbolic antiderivative evaluates to the correct numeric
antiderivative.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend

X = IRSymbol("x")


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


# ---------------------------------------------------------------------------
# Helpers — numeric evaluation of a returned antiderivative.
# ---------------------------------------------------------------------------


def _integrate(f: IRNode) -> IRApply:
    return IRApply(INTEGRATE, (f, X))


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911, PLR0912
    """Walk an IR antiderivative and evaluate it at ``x = x_val``.

    Only the subset of heads the integration handler emits is covered;
    anything else raises so test failures show up as a clear
    ``ValueError`` instead of a wrong numeric answer.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRSymbol):
        if node == X:
            return x_val
        raise ValueError(f"unknown free symbol: {node}")
    assert isinstance(node, IRApply), f"unexpected IR shape: {node!r}"
    head = node.head
    if head == ADD:
        return sum(_eval_ir(a, x_val) for a in node.args)
    if head == SUB:
        return _eval_ir(node.args[0], x_val) - _eval_ir(node.args[1], x_val)
    if head == MUL:
        result = 1.0
        for a in node.args:
            result *= _eval_ir(a, x_val)
        return result
    if head == DIV:
        return _eval_ir(node.args[0], x_val) / _eval_ir(node.args[1], x_val)
    if head == NEG:
        return -_eval_ir(node.args[0], x_val)
    if head == POW:
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == SIN:
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == COS:
        return math.cos(_eval_ir(node.args[0], x_val))
    if head == EXP:
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == LOG:
        return math.log(_eval_ir(node.args[0], x_val))
    raise ValueError(f"unsupported head in test eval: {head}")


def _trapezoidal(integrand_fn, a: float, b: float, n: int = 50_000) -> float:
    """Plain trapezoidal rule — used as ground truth for definite integrals."""
    h = (b - a) / n
    total = 0.5 * (integrand_fn(a) + integrand_fn(b))
    for i in range(1, n):
        total += integrand_fn(a + i * h)
    return total * h


def _contains_head(node: IRNode, head: IRSymbol) -> bool:
    """Recursively check whether *node* contains an ``IRApply`` with *head*."""
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in node.args)
    return False


# ---------------------------------------------------------------------------
# Test 1 — Acceptance: ∫ x·sin(x) dx
# ---------------------------------------------------------------------------


def test_acceptance_x_times_sin_x(vm: VM) -> None:
    """``∫ x·sin(x) dx`` closes and the antiderivative is numerically correct."""
    integrand = IRApply(MUL, (X, IRApply(SIN, (X,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"not closed: {out}"
    # F(1) − F(0) should equal ∫₀¹ x·sin(x) dx = sin(1) − cos(1) ≈ 0.30116867.
    diff = _eval_ir(out, 1.0) - _eval_ir(out, 0.0)
    expected = math.sin(1.0) - math.cos(1.0) * 1.0 - (0.0 - math.cos(0.0) * 0.0)
    # The classical antiderivative is sin(x) − x·cos(x); evaluate at 1 and 0:
    # = (sin 1 − cos 1) − 0 = sin 1 − cos 1 ≈ 0.30116867.
    expected_alt = math.sin(1.0) - math.cos(1.0)
    assert math.isclose(diff, expected, abs_tol=1e-9)
    assert math.isclose(diff, expected_alt, abs_tol=1e-9)


# ---------------------------------------------------------------------------
# Test 2 — Acceptance: ∫ x²·eˣ dx
# ---------------------------------------------------------------------------


def test_acceptance_xsquared_times_exp_x(vm: VM) -> None:
    """``∫ x²·eˣ dx = (x² − 2x + 2)·eˣ`` (classic IBP result)."""
    integrand = IRApply(MUL, (IRApply(POW, (X, IRInteger(2))), IRApply(EXP, (X,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"not closed: {out}"
    # Evaluate the symbolic antiderivative at x = 2 and x = 0.  Known
    # value: F(2) − F(0) = (4 − 4 + 2)·e² − (0 − 0 + 2)·e⁰ = 2e² − 2.
    diff = _eval_ir(out, 2.0) - _eval_ir(out, 0.0)
    expected = 2.0 * math.exp(2.0) - 2.0
    assert math.isclose(diff, expected, abs_tol=1e-9)


# ---------------------------------------------------------------------------
# Test 3 — Higher-degree polynomial × trig: ∫ x³·cos(x) dx
# ---------------------------------------------------------------------------


def test_higher_degree_xcubed_times_cos_x(vm: VM) -> None:
    """``∫ x³·cos(x) dx`` closes.

    Closed form: ``x³·sin(x) + 3x²·cos(x) − 6x·sin(x) − 6·cos(x) + C``.
    We check numerically at x = 1 vs. x = 0 against the trapezoidal rule.
    """
    integrand = IRApply(MUL, (IRApply(POW, (X, IRInteger(3))), IRApply(COS, (X,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"not closed: {out}"
    diff = _eval_ir(out, 1.0) - _eval_ir(out, 0.0)
    numeric = _trapezoidal(lambda xv: xv**3 * math.cos(xv), 0.0, 1.0)
    assert math.isclose(diff, numeric, abs_tol=1e-5)


# ---------------------------------------------------------------------------
# Test 4 — Three-factor Mul gap closed by the new fallback.
# ---------------------------------------------------------------------------


def test_three_factor_x_sin_cos_via_ibp(vm: VM) -> None:
    """``∫ x·sin(x)·cos(x) dx`` — the IBP fallback closes this case.

    Before Track E1, the per-shape handlers ``_try_trig_product``
    matched only two-factor ``MUL(poly, sin/cos(linear))`` patterns and
    bailed on the three-factor shape.  Tabular IBP closes it via
    ``u = x, w = sin(x)·cos(x)`` (recursive integrate gives
    ``−cos(2x)/4`` for ``∫w``).

    Numeric ground truth at x = 0..1:
    ``∫₀¹ x·sin(x)·cos(x) dx = ∫₀¹ x·sin(2x)/2 dx ≈ 0.21770``.
    """
    integrand = IRApply(
        MUL,
        (X, IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (X,))))),
    )
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), (
        f"expected closed form via tabular IBP, got: {out}"
    )
    diff = _eval_ir(out, 1.0) - _eval_ir(out, 0.0)
    numeric = _trapezoidal(lambda xv: xv * math.sin(xv) * math.cos(xv), 0.0, 1.0)
    assert math.isclose(diff, numeric, abs_tol=1e-5)


# ---------------------------------------------------------------------------
# Test 5 — Fallthrough: ``∫ 1/x dx`` uses the existing log handler, IBP not
# fired (the integrand isn't a Mul, so tabular short-circuits to None).
# ---------------------------------------------------------------------------


def test_fallthrough_one_over_x_returns_log(vm: VM) -> None:
    """``∫ 1/x dx = log(x)`` is closed by the existing handler.

    The new IBP fallback short-circuits on non-``Mul`` integrands —
    ``Div(1, x)`` has head ``DIV``, so ``try_ibp_tabular`` returns
    ``None`` immediately and the elementary log rule provides the
    closed form unchanged from before Track E1.
    """
    integrand = IRApply(DIV, (IRInteger(1), X))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form: {out}"
    assert _contains_head(out, LOG)
    # F(2) − F(1) = log 2 ≈ 0.693147.
    diff = _eval_ir(out, 2.0) - _eval_ir(out, 1.0)
    assert math.isclose(diff, math.log(2.0), abs_tol=1e-12)


# ---------------------------------------------------------------------------
# Test 6 — Fallthrough on un-integrable factor: ∫ sin(x²) dx (Fresnel).
# ---------------------------------------------------------------------------


def test_fallthrough_fresnel_sin_xsq(vm: VM) -> None:
    """``∫ sin(x²) dx`` — Fresnel; IBP can't help.

    The special-function handler (Phase 23) returns the Fresnel form
    *before* tabular IBP runs, so the result mentions ``FresnelS``.
    The critical correctness property is that tabular IBP doesn't
    invent a bogus elementary closed form behind its back — the
    Fresnel form must survive.  Even if Phase 23 were absent, the
    integrand is **not a Mul**, so ``try_ibp_tabular`` would also
    return ``None`` and the result would remain ``Integrate(...)``.
    """
    integrand = IRApply(SIN, (IRApply(POW, (X, IRInteger(2))),))
    out = vm.eval(_integrate(integrand))
    # Either the Fresnel special-function form (if Phase 23 fires) or
    # the unevaluated ``Integrate`` form must come back — but NOT a
    # bogus elementary closed form.
    fresnel = IRSymbol("FresnelS")
    has_fresnel = _contains_head(out, fresnel)
    has_unevaluated = _contains_head(out, INTEGRATE)
    assert has_fresnel or has_unevaluated, (
        f"expected Fresnel or unevaluated, got: {out}"
    )
    # Crucially: no SIN/COS antiderivative-shape fabrication.  The
    # Fresnel function may appear inside the result but it's wrapped
    # as ``FresnelS(...)`` — the tell is the FresnelS head, not loose
    # ``Sin`` over a bare ``x``.  We just check the engine did not lie.
    if not has_fresnel:
        assert has_unevaluated
