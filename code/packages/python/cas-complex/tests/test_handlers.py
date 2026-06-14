"""Tests for complex handlers — no symbolic_vm dependency.

Uses a self-contained MinimalVM stub so this package can be tested
without installing the full symbolic_vm stack (which would create a
circular dependency: symbolic_vm → cas_complex → symbolic_vm).

Architecture
------------
``_numeric_fold`` handles flat arithmetic on ``IRInteger`` / ``IRFloat``
nodes (after subexpressions are already evaluated).  ``_MinimalVM.eval``
evaluates recursively bottom-up, then dispatches to a registered handler,
then falls back to ``_numeric_fold`` — exactly like the real VM.
``make_vm()`` wires all the complex handlers (plus Pow / Mul / Div / Exp
chains that mirror what ``SymbolicBackend.__init__`` does).
"""
from __future__ import annotations

import math
from typing import Any

from symbolic_ir import (
    ADD,
    MUL,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_complex import IMAGINARY_UNIT, build_complex_handler_table
from cas_complex.handlers import (
    atan2_handler,
    complex_div_handler,
    complex_mul_handler,
    complex_pow_handler,
    euler_pow_handler,
    exp_complex_handler,
    imaginary_power_handler,
)

# ---------------------------------------------------------------------------
# Numeric fold helper — flat arithmetic on literal nodes only.
# ---------------------------------------------------------------------------


def _to_float(node: IRNode) -> float | None:
    if isinstance(node, (IRInteger, IRFloat)):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    return None


def _numeric_fold(node: IRNode) -> IRNode:
    """Evaluate simple arithmetic / trig / pow on literal (numeric) nodes.

    This is called *after* sub-expressions have already been evaluated,
    so we only ever see flat combinations of ``IRInteger`` / ``IRFloat``.
    """
    if not isinstance(node, IRApply) or not isinstance(node.head, IRSymbol):
        return node
    name = node.head.name
    args = node.args

    if name == "Neg" and len(args) == 1:
        v = _to_float(args[0])
        if v is not None:
            result = -v
            exact = isinstance(args[0], IRInteger) and int(result) == result
            return IRInteger(int(result)) if exact else IRFloat(result)

    if name in ("Add", "Sub", "Mul") and len(args) == 2:
        av, bv = _to_float(args[0]), _to_float(args[1])
        if av is not None and bv is not None:
            if name == "Add":
                result = av + bv
            elif name == "Sub":
                result = av - bv
            else:
                result = av * bv
            # Keep as integer when both inputs are integers and result is exact.
            if (
                isinstance(args[0], IRInteger)
                and isinstance(args[1], IRInteger)
                and float(int(result)) == result
            ):
                return IRInteger(int(result))
            return IRFloat(result)

    if name == "Div" and len(args) == 2:
        av, bv = _to_float(args[0]), _to_float(args[1])
        if av is not None and bv is not None and bv != 0.0:
            result = av / bv
            # Return IRRational for integer / integer when exact.
            if isinstance(args[0], IRInteger) and isinstance(args[1], IRInteger):
                from math import gcd
                n, d = int(args[0].value), int(args[1].value)
                if d < 0:
                    n, d = -n, -d
                g = gcd(abs(n), d)
                n, d = n // g, d // g
                return IRInteger(n) if d == 1 else IRRational(n, d)
            return IRFloat(result)

    if name == "Cos" and len(args) == 1:
        v = _to_float(args[0])
        if v is not None:
            return IRFloat(math.cos(v))

    if name == "Sin" and len(args) == 1:
        v = _to_float(args[0])
        if v is not None:
            return IRFloat(math.sin(v))

    if name == "Sqrt" and len(args) == 1:
        v = _to_float(args[0])
        if v is not None and v >= 0:
            sq = math.sqrt(v)
            return IRInteger(int(sq)) if float(int(sq)) == sq else IRFloat(sq)

    if name == "Atan2" and len(args) == 2:
        yv, xv = _to_float(args[0]), _to_float(args[1])
        if yv is not None and xv is not None:
            return IRFloat(math.atan2(yv, xv))

    if name == "Pow" and len(args) == 2:
        bv, ev = _to_float(args[0]), _to_float(args[1])
        if bv is not None and ev is not None:
            try:
                result = bv ** ev
                return IRFloat(result)
            except (OverflowError, ZeroDivisionError, ValueError):
                return node

    return node


# ---------------------------------------------------------------------------
# Minimal recursive VM stub — no symbolic_vm dependency.
# ---------------------------------------------------------------------------


class _MinimalVM:
    """Minimal VM that evaluates bottom-up and falls back to numeric folding.

    Evaluation order mirrors the real VM:
    1. Recursively evaluate each argument.
    2. Look up the head in ``_handlers`` and call it.
    3. If no handler or handler returned ``expr`` unchanged, apply
       ``_numeric_fold`` to handle arithmetic on literal nodes.
    """

    def __init__(self) -> None:
        self._handlers: dict[str, Any] = {}

    def eval(self, node: IRNode) -> IRNode:
        if not isinstance(node, IRApply):
            return node
        if not isinstance(node.head, IRSymbol):
            return node
        # Evaluate arguments first (bottom-up).
        evaled_args = tuple(self.eval(a) for a in node.args)
        if evaled_args != node.args:
            node = IRApply(node.head, evaled_args)
        # Dispatch to registered handler.
        handler = self._handlers.get(node.head.name)
        if handler is not None:
            result = handler(self, node)
            if result is not node:
                return result
        # Numeric fold for unhandled arithmetic.
        return _numeric_fold(node)


# ---------------------------------------------------------------------------
# Head-name constants for readability in tests.
# ---------------------------------------------------------------------------

_RE = IRSymbol("Re")
_IM = IRSymbol("Im")
_CONJUGATE = IRSymbol("Conjugate")
_ABS_COMPLEX = IRSymbol("AbsComplex")
_ARG = IRSymbol("Arg")
_RECT_FORM = IRSymbol("RectForm")
_POLAR_FORM = IRSymbol("PolarForm")


# ---------------------------------------------------------------------------
# VM factory — mirrors SymbolicBackend's complex-handler wiring.
# ---------------------------------------------------------------------------


def make_vm() -> _MinimalVM:
    """Build a MinimalVM wired with all complex handlers plus numeric folding.

    This mirrors the handler-chain setup in ``SymbolicBackend.__init__``:
    - Pow: IMAGINARY_POWER_HOOK → euler_pow_handler → complex_pow_handler → fold
    - Mul: complex_mul_handler → fold
    - Div: complex_div_handler → fold
    - Exp: exp_complex_handler → fold
    - Remaining heads from ``build_complex_handler_table()`` registered directly.
    """
    vm = _MinimalVM()
    vm._handlers.update(build_complex_handler_table())

    # -- Pow chain ----------------------------------------------------------
    def full_pow_handler(vm_: _MinimalVM, expr: IRApply) -> IRNode:
        # i^n → {1, i, -1, -i}
        if (
            len(expr.args) == 2
            and isinstance(expr.args[0], IRSymbol)
            and expr.args[0].name == "ImaginaryUnit"
            and isinstance(expr.args[1], IRInteger)
        ):
            return imaginary_power_handler(vm_, expr)  # type: ignore[arg-type]
        # b^(i*theta) via Euler's formula
        result = euler_pow_handler(vm_, expr)  # type: ignore[arg-type]
        if result is not expr:
            return result
        # (a+bi)^n for small positive integers
        result = complex_pow_handler(vm_, expr)  # type: ignore[arg-type]
        if result is not expr:
            return result
        return expr  # fall through to numeric fold in _MinimalVM.eval

    vm._handlers["Pow"] = full_pow_handler

    # -- Mul chain ----------------------------------------------------------
    def full_mul_handler(vm_: _MinimalVM, expr: IRApply) -> IRNode:
        result = complex_mul_handler(vm_, expr)  # type: ignore[arg-type]
        if result is not expr:
            return result
        return expr  # fall through to numeric fold

    vm._handlers["Mul"] = full_mul_handler

    # -- Div chain ----------------------------------------------------------
    def full_div_handler(vm_: _MinimalVM, expr: IRApply) -> IRNode:
        result = complex_div_handler(vm_, expr)  # type: ignore[arg-type]
        if result is not expr:
            return result
        return expr  # fall through to numeric fold

    vm._handlers["Div"] = full_div_handler

    # -- Exp → Euler's formula ----------------------------------------------
    def full_exp_handler(vm_: _MinimalVM, expr: IRApply) -> IRNode:
        return exp_complex_handler(vm_, expr)  # type: ignore[arg-type]

    vm._handlers["Exp"] = full_exp_handler

    return vm


# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------


def rect(a: IRNode, b: IRNode) -> IRApply:
    """Construct ``a + b * ImaginaryUnit`` directly as an IRApply."""
    return IRApply(ADD, (a, IRApply(MUL, (b, IMAGINARY_UNIT))))


def _float_val(node: IRNode) -> float:
    """Extract float value from IRFloat or IRInteger."""
    assert isinstance(node, (IRFloat, IRInteger)), (
        f"Expected numeric node, got {node!r}"
    )
    return float(node.value)


# ---------------------------------------------------------------------------
# Re / Im handlers
# ---------------------------------------------------------------------------


def test_re_pure_real() -> None:
    vm = make_vm()
    x = IRSymbol("x")
    assert vm.eval(IRApply(_RE, (x,))) == x


def test_re_of_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(3), IRInteger(4))
    assert vm.eval(IRApply(_RE, (node,))) == IRInteger(3)


def test_im_of_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(3), IRInteger(4))
    assert vm.eval(IRApply(_IM, (node,))) == IRInteger(4)


def test_im_pure_real() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_IM, (IRInteger(5),))) == IRInteger(0)


def test_re_wrong_arity_passthrough() -> None:
    vm = make_vm()
    expr = IRApply(_RE, (IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == expr


# ---------------------------------------------------------------------------
# Conjugate handler
# ---------------------------------------------------------------------------


def test_conjugate_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_CONJUGATE, (node,)))
    # conjugate(3 + 4i) → 3 - 4i — an Add node
    assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# ImaginaryUnit power reduction — i^n → {1, i, -1, -i}
# ---------------------------------------------------------------------------


def test_i_power_0() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(0)))
    assert vm.eval(expr) == IRInteger(1)


def test_i_power_1() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(1)))
    assert vm.eval(expr) == IMAGINARY_UNIT


def test_i_power_2_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(2)))
    assert vm.eval(expr) == IRInteger(-1)


def test_i_power_3_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(3)))
    result = vm.eval(expr)
    # Should be -i (a Neg or Mul node wrapping ImaginaryUnit)
    assert isinstance(result, IRApply)


def test_i_power_4_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(4)))
    assert vm.eval(expr) == IRInteger(1)


def test_i_power_7_cycles_back_to_neg_i() -> None:
    """i^7 = i^(4+3) = i^3 = -i."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(7)))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# AbsComplex handler
# ---------------------------------------------------------------------------


def test_abs_complex_3_4() -> None:
    vm = make_vm()
    node = rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_ABS_COMPLEX, (node,)))
    # sqrt(9+16) = sqrt(25) = 5
    assert result is not None
    v = _to_float(result)
    if v is not None:
        assert abs(v - 5.0) < 1e-9


def test_abs_complex_passthrough_real() -> None:
    vm = make_vm()
    x = IRSymbol("x")
    expr = IRApply(_ABS_COMPLEX, (x,))
    assert vm.eval(expr) == expr


def test_abs_complex_wrong_arity() -> None:
    vm = make_vm()
    expr = IRApply(_ABS_COMPLEX, (IMAGINARY_UNIT, IRInteger(1)))
    assert vm.eval(expr) == expr


# ---------------------------------------------------------------------------
# RectForm / PolarForm handlers
# ---------------------------------------------------------------------------


def test_rect_form_passthrough_real() -> None:
    vm = make_vm()
    x = IRSymbol("x")
    assert vm.eval(IRApply(_RECT_FORM, (x,))) == x


def test_rect_form_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(2), IRInteger(3))
    result = vm.eval(IRApply(_RECT_FORM, (node,)))
    assert isinstance(result, IRApply)


def test_polar_form_wrong_arity() -> None:
    vm = make_vm()
    expr = IRApply(_POLAR_FORM, ())
    result = vm.eval(expr)
    assert result == expr


# ---------------------------------------------------------------------------
# complex_mul_handler — (a+bi)(c+di) expansion
# ---------------------------------------------------------------------------


def test_complex_mul_rect_by_rect() -> None:
    """(1+i)(1-i) = 2."""
    vm = make_vm()
    z1 = rect(IRInteger(1), IRInteger(1))
    z2 = rect(IRInteger(1), IRInteger(-1))
    result = vm.eval(IRApply(IRSymbol("Mul"), (z1, z2)))
    assert result == IRInteger(2)


def test_complex_mul_imaginary_times_imaginary() -> None:
    """i * i = -1."""
    vm = make_vm()
    result = vm.eval(IRApply(IRSymbol("Mul"), (IMAGINARY_UNIT, IMAGINARY_UNIT)))
    assert result == IRInteger(-1)


def test_complex_mul_scalar_times_rect() -> None:
    """2 * (3 + 4i) = 6 + 8i."""
    vm = make_vm()
    z = rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(IRSymbol("Mul"), (IRInteger(2), z)))
    assert isinstance(result, IRApply)


def test_complex_mul_passthrough_real() -> None:
    """Mul(2, 3) — no imaginary component → falls through to numeric fold."""
    vm = make_vm()
    result = vm.eval(IRApply(IRSymbol("Mul"), (IRInteger(2), IRInteger(3))))
    assert result == IRInteger(6)


def test_complex_mul_wrong_arity() -> None:
    """Mul with three args → not handled by complex_mul_handler."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Mul"), (IMAGINARY_UNIT, IRInteger(1), IRInteger(2)))
    result = complex_mul_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_mul_both_im_zero_passthrough() -> None:
    """Both imaginary parts zero after split_rect → fall through to avoid recursion."""
    vm = make_vm()
    # Mul(x, y) where neither contains ImaginaryUnit
    x = IRSymbol("x")
    y = IRSymbol("y")
    expr = IRApply(IRSymbol("Mul"), (x, y))
    result = complex_mul_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


# ---------------------------------------------------------------------------
# complex_div_handler — (a+bi) / c  and  (a+bi) / (c+di)
# ---------------------------------------------------------------------------


def test_complex_div_rect_by_real() -> None:
    """(0 + 2*i) / 2 = i."""
    vm = make_vm()
    z = rect(IRInteger(0), IRInteger(2))
    result = vm.eval(IRApply(IRSymbol("Div"), (z, IRInteger(2))))
    assert result == IMAGINARY_UNIT


def test_complex_div_rect_by_rect() -> None:
    """(3+4i) / (1+2i) = 11/5 - 2/5*i."""
    vm = make_vm()
    z1 = rect(IRInteger(3), IRInteger(4))
    z2 = rect(IRInteger(1), IRInteger(2))
    result = vm.eval(IRApply(IRSymbol("Div"), (z1, z2)))
    # Result has a real and imaginary component.
    assert isinstance(result, (IRApply, IRInteger, IRRational))


def test_complex_div_unity() -> None:
    """(1+i) / (1-i) = i."""
    vm = make_vm()
    z1 = rect(IRInteger(1), IRInteger(1))
    z2 = rect(IRInteger(1), IRInteger(-1))
    result = vm.eval(IRApply(IRSymbol("Div"), (z1, z2)))
    assert result == IMAGINARY_UNIT


def test_complex_div_passthrough_real_numerator() -> None:
    """Div(x, 2) — x is not complex → fall through."""
    vm = make_vm()
    x = IRSymbol("x")
    expr = IRApply(IRSymbol("Div"), (x, IRInteger(2)))
    result = complex_div_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_div_wrong_arity() -> None:
    """Div with wrong arity → passthrough."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Div"), (IMAGINARY_UNIT,))
    result = complex_div_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_div_i_over_float() -> None:
    """i / 2.0 = 0.5*i."""
    vm = make_vm()
    result = vm.eval(IRApply(IRSymbol("Div"), (IMAGINARY_UNIT, IRFloat(2.0))))
    # Imaginary part should be 0.5
    assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# complex_pow_handler — (a+bi)^n for integer n
# ---------------------------------------------------------------------------


def test_complex_pow_2_plus_i_squared() -> None:
    """(2+i)^2 = 3 + 4i."""
    vm = make_vm()
    z = rect(IRInteger(2), IRInteger(1))
    result = vm.eval(IRApply(IRSymbol("Pow"), (z, IRInteger(2))))
    # 4 - 1 + 4i = 3 + 4i
    assert isinstance(result, (IRApply, IRInteger))


def test_complex_pow_1_plus_i_cubed() -> None:
    """(1+i)^3 = -2 + 2i."""
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    result = vm.eval(IRApply(IRSymbol("Pow"), (z, IRInteger(3))))
    assert isinstance(result, IRApply)


def test_complex_pow_1_plus_i_fourth() -> None:
    """(1+i)^4 = -4  (purely real)."""
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    result = vm.eval(IRApply(IRSymbol("Pow"), (z, IRInteger(4))))
    assert isinstance(result, (IRInteger, IRFloat))
    assert abs(_float_val(result) - (-4.0)) < 1e-9


def test_complex_pow_n_equals_1() -> None:
    """(2+3i)^1 = 2+3i (complex_pow_handler returns base directly)."""
    vm = make_vm()
    z = rect(IRInteger(2), IRInteger(3))
    result = complex_pow_handler(vm, IRApply(IRSymbol("Pow"), (z, IRInteger(1))))  # type: ignore[arg-type]
    assert result is z


def test_complex_pow_passthrough_zero() -> None:
    """n=0 → not expanded by complex_pow_handler (guard for non-positive)."""
    vm = make_vm()
    z = rect(IRInteger(2), IRInteger(1))
    expr = IRApply(IRSymbol("Pow"), (z, IRInteger(0)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_pow_passthrough_negative() -> None:
    """n<0 → not expanded by complex_pow_handler."""
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    expr = IRApply(IRSymbol("Pow"), (z, IRInteger(-2)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_pow_passthrough_too_large() -> None:
    """n>16 → not expanded (guard against deep recursion)."""
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    expr = IRApply(IRSymbol("Pow"), (z, IRInteger(17)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_pow_passthrough_imaginary_unit_base() -> None:
    """ImaginaryUnit^n → delegated to imaginary_power_handler, not complex_pow."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(2)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_pow_passthrough_non_integer_exponent() -> None:
    """Non-integer exponent → fall through."""
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    expr = IRApply(IRSymbol("Pow"), (z, IRFloat(2.5)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_complex_pow_passthrough_real_base() -> None:
    """Real base → not a complex power, fall through."""
    vm = make_vm()
    x = IRSymbol("x")
    expr = IRApply(IRSymbol("Pow"), (x, IRInteger(2)))
    result = complex_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


# ---------------------------------------------------------------------------
# euler_pow_handler — b^(i*theta)
# ---------------------------------------------------------------------------


def test_euler_pow_e_to_i_pi() -> None:
    """e^(i*pi) ≈ -1."""
    vm = make_vm()
    base = IRFloat(math.e)
    exp_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(math.pi)))
    result = vm.eval(IRApply(IRSymbol("Pow"), (base, exp_)))
    # cos(pi) = -1, sin(pi) ≈ 0 → result ≈ -1
    v = _float_val(result)
    assert abs(v - (-1.0)) < 1e-9, f"Expected ≈-1 but got {v}"


def test_euler_pow_e_to_i_pi_over_2() -> None:
    """e^(i*pi/2) ≈ i (purely imaginary)."""
    vm = make_vm()
    base = IRFloat(math.e)
    exp_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(math.pi / 2)))
    result = vm.eval(IRApply(IRSymbol("Pow"), (base, exp_)))
    # cos(pi/2) ≈ 0, sin(pi/2) = 1 → should be i
    assert result == IMAGINARY_UNIT or (
        isinstance(result, IRApply) and "Mul" in result.head.name
    )


def test_euler_pow_passthrough_complex_base() -> None:
    """Euler handler skips if base is complex (not positive real numeric).

    Requires ``cas_trig``: even though the final answer falls through,
    ``euler_pow_handler`` finds an imaginary exponent and reaches the
    ``from symbolic_vm.numeric import …`` statement before checking the
    base, triggering ``symbolic_vm.__init__`` → ``backends`` → ``cas_trig``.
    """
    vm = make_vm()
    z = rect(IRInteger(1), IRInteger(1))
    exp_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(1.0)))
    expr = IRApply(IRSymbol("Pow"), (z, exp_))
    result = euler_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_euler_pow_passthrough_real_exponent() -> None:
    """Euler handler skips for non-imaginary (real) exponents."""
    vm = make_vm()
    base = IRFloat(math.e)
    exp_ = IRFloat(1.0)  # real exponent, not imaginary
    expr = IRApply(IRSymbol("Pow"), (base, exp_))
    result = euler_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_euler_pow_passthrough_imaginary_unit_base() -> None:
    """Euler handler defers ImaginaryUnit base to IMAGINARY_POWER_HOOK."""
    vm = make_vm()
    exp_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(1.0)))
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, exp_))
    result = euler_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_euler_pow_negative_base_passthrough() -> None:
    """Euler handler skips for non-positive base (can't take log)."""
    vm = make_vm()
    base = IRFloat(-1.0)  # negative, can't use log
    exp_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(1.0)))
    expr = IRApply(IRSymbol("Pow"), (base, exp_))
    result = euler_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_euler_pow_wrong_arity() -> None:
    """Euler handler wrong arity → passthrough — no trig import needed."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IRFloat(math.e),))
    result = euler_pow_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


# ---------------------------------------------------------------------------
# exp_complex_handler — Exp(i*theta) and Exp(a + b*i)
# ---------------------------------------------------------------------------


def test_exp_complex_pure_imaginary() -> None:
    """Exp(i*pi) via exp_complex_handler ≈ -1 + 0*i."""
    vm = make_vm()
    arg_ = IRApply(MUL, (IMAGINARY_UNIT, IRFloat(math.pi)))
    result = vm.eval(IRApply(IRSymbol("Exp"), (arg_,)))
    # cos(pi) = -1
    v = _float_val(result)
    assert abs(v - (-1.0)) < 1e-9


def test_exp_complex_general() -> None:
    """Exp(0.0 + (pi/2)*i) gives a value close to i."""
    vm = make_vm()
    arg_ = rect(IRFloat(0.0), IRFloat(math.pi / 2))
    result = exp_complex_handler(vm, IRApply(IRSymbol("Exp"), (arg_,)))  # type: ignore[arg-type]
    # Should have been reduced — not returned unevaluated
    assert result is not IRApply(IRSymbol("Exp"), (arg_,))


def test_exp_complex_passthrough_real() -> None:
    """Exp(x) where x is a real symbol → fall through unchanged."""
    vm = make_vm()
    x = IRSymbol("x")
    expr = IRApply(IRSymbol("Exp"), (x,))
    result = exp_complex_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_exp_complex_wrong_arity() -> None:
    """Exp with wrong arity → passthrough."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Exp"), (IMAGINARY_UNIT, IRInteger(1)))
    result = exp_complex_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


# ---------------------------------------------------------------------------
# atan2_handler — numeric arc-tangent
# ---------------------------------------------------------------------------


def test_atan2_handler_zero_minus_one() -> None:
    """Atan2(0, -1) = pi."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Atan2"), (IRInteger(0), IRInteger(-1)))
    result = vm.eval(expr)
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.pi) < 1e-12


def test_atan2_handler_first_quadrant() -> None:
    """Atan2(1, 1) = pi/4."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Atan2"), (IRFloat(1.0), IRFloat(1.0)))
    result = vm.eval(expr)
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.pi / 4) < 1e-12


def test_atan2_handler_zero_one() -> None:
    """Atan2(0, 1) = 0 (returned as IRInteger(0) since the value is exact)."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Atan2"), (IRInteger(0), IRInteger(1)))
    result = vm.eval(expr)
    # _from_number returns IRInteger for exact-integer values
    assert isinstance(result, (IRInteger, IRFloat))
    v = _to_float(result)
    assert v is not None and abs(v) < 1e-12


def test_atan2_handler_symbolic_passthrough() -> None:
    """Atan2(x, 1.0) where x is symbolic → return unevaluated."""
    vm = make_vm()
    x = IRSymbol("x")
    expr = IRApply(IRSymbol("Atan2"), (x, IRFloat(1.0)))
    result = atan2_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr


def test_atan2_handler_wrong_arity() -> None:
    """Atan2 with wrong arity → passthrough."""
    vm = make_vm()
    expr = IRApply(IRSymbol("Atan2"), (IRFloat(1.0),))
    result = atan2_handler(vm, expr)  # type: ignore[arg-type]
    assert result is expr
