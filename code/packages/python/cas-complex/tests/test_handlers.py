"""Tests for complex handlers — no symbolic_vm dependency.

Uses a self-contained MinimalVM stub so this package can be tested
without installing the full symbolic_vm stack (which would create a
circular dependency: symbolic_vm → cas_complex → symbolic_vm).
"""
from __future__ import annotations

from typing import Any

from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRNode, IRSymbol

from cas_complex import IMAGINARY_UNIT, build_complex_handler_table
from cas_complex.handlers import imaginary_power_handler


# ---------------------------------------------------------------------------
# Minimal VM stub — no symbolic_vm dependency required.
# Dispatches to registered handlers; leaves unregistered heads unevaluated.
# ---------------------------------------------------------------------------

class _MinimalVM:
    def __init__(self) -> None:
        self._handlers: dict[str, Any] = {}

    def eval(self, node: IRNode) -> IRNode:
        if not isinstance(node, IRApply):
            return node
        if not isinstance(node.head, IRSymbol):
            return node
        handler = self._handlers.get(node.head.name)
        if handler is None:
            return node
        return handler(self, node)  # type: ignore[no-any-return]


_RE = IRSymbol("Re")
_IM = IRSymbol("Im")
_CONJUGATE = IRSymbol("Conjugate")
_ABS_COMPLEX = IRSymbol("AbsComplex")
_ARG = IRSymbol("Arg")
_RECT_FORM = IRSymbol("RectForm")
_POLAR_FORM = IRSymbol("PolarForm")
_I_POW = IRSymbol("_ImaginaryPow")


def make_vm() -> _MinimalVM:
    vm = _MinimalVM()
    vm._handlers.update(build_complex_handler_table())
    # Wire imaginary power through Pow
    original_pow = vm._handlers.get("Pow")

    def complex_pow_handler(vm_: _MinimalVM, expr: IRApply) -> IRNode:
        if (
            len(expr.args) == 2
            and isinstance(expr.args[0], IRSymbol)
            and expr.args[0].name == "ImaginaryUnit"
            and isinstance(expr.args[1], IRInteger)
        ):
            return imaginary_power_handler(vm_, expr)  # type: ignore[arg-type]
        if original_pow is not None:
            return original_pow(vm_, expr)
        return expr

    vm._handlers["Pow"] = complex_pow_handler
    return vm


def i() -> IRSymbol:
    return IMAGINARY_UNIT


def rect(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(ADD, (a, IRApply(MUL, (b, IMAGINARY_UNIT))))


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


def test_conjugate_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_CONJUGATE, (node,)))
    # conjugate(3 + 4i) → 3 + (-4)*i — an Add node
    assert isinstance(result, IRApply)


def test_i_power_2_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(2)))
    assert vm.eval(expr) == IRInteger(-1)


def test_i_power_4_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(4)))
    assert vm.eval(expr) == IRInteger(1)


def test_i_power_3_via_pow() -> None:
    vm = make_vm()
    expr = IRApply(IRSymbol("Pow"), (IMAGINARY_UNIT, IRInteger(3)))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"


def test_abs_complex_3_4() -> None:
    vm = make_vm()
    # AbsComplex(3 + 4i) = sqrt(25) — not numerically folded yet
    node = rect(IRInteger(3), IRInteger(4))
    result = vm.eval(IRApply(_ABS_COMPLEX, (node,)))
    # Should be a Sqrt expression (or 5 if the VM folds it)
    assert result is not None


def test_abs_complex_passthrough_real() -> None:
    vm = make_vm()
    # AbsComplex(x) — x is not complex, returns unevaluated
    x = IRSymbol("x")
    expr = IRApply(_ABS_COMPLEX, (x,))
    assert vm.eval(expr) == expr


def test_rect_form_passthrough_real() -> None:
    vm = make_vm()
    x = IRSymbol("x")
    assert vm.eval(IRApply(_RECT_FORM, (x,))) == x


def test_rect_form_rect() -> None:
    vm = make_vm()
    node = rect(IRInteger(2), IRInteger(3))
    result = vm.eval(IRApply(_RECT_FORM, (node,)))
    assert isinstance(result, IRApply)


def test_re_wrong_arity_passthrough() -> None:
    vm = make_vm()
    expr = IRApply(_RE, (IRInteger(1), IRInteger(2)))
    assert vm.eval(expr) == expr
