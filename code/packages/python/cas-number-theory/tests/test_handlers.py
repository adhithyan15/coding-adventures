"""Tests for cas_number_theory.handlers via SymbolicBackend."""
from __future__ import annotations
import pytest
from symbolic_ir import IRApply, IRInteger, IRSymbol
from symbolic_vm import VM, SymbolicBackend
from cas_number_theory.handlers import build_number_theory_handler_table

_LIST = IRSymbol("List")
_IS_PRIME = IRSymbol("IsPrime")
_NEXT_PRIME = IRSymbol("NextPrime")
_PREV_PRIME = IRSymbol("PrevPrime")
_FACTOR_INTEGER = IRSymbol("FactorInteger")
_DIVISORS = IRSymbol("Divisors")
_TOTIENT = IRSymbol("Totient")
_MOEBIUS_MU = IRSymbol("MoebiusMu")
_JACOBI = IRSymbol("JacobiSymbol")
_CRT = IRSymbol("ChineseRemainder")
_INT_LEN = IRSymbol("IntegerLength")
_TRUE = IRSymbol("True")
_FALSE = IRSymbol("False")


def make_vm() -> VM:
    backend = SymbolicBackend()
    # Install number theory handlers
    backend._handlers.update(build_number_theory_handler_table())
    return VM(backend)


def ilist(*args: object) -> IRApply:
    return IRApply(_LIST, tuple(args))  # type: ignore[arg-type]


def test_is_prime_2() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_IS_PRIME, (IRInteger(2),))) == _TRUE


def test_is_prime_4() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_IS_PRIME, (IRInteger(4),))) == _FALSE


def test_is_prime_1() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_IS_PRIME, (IRInteger(1),))) == _FALSE


def test_next_prime_10() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_NEXT_PRIME, (IRInteger(10),))) == IRInteger(11)


def test_prev_prime_10() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_PREV_PRIME, (IRInteger(10),))) == IRInteger(7)


def test_prev_prime_2_unevaluated() -> None:
    vm = make_vm()
    expr = IRApply(_PREV_PRIME, (IRInteger(2),))
    assert vm.eval(expr) == expr


def test_factor_integer_12() -> None:
    vm = make_vm()
    result = vm.eval(IRApply(_FACTOR_INTEGER, (IRInteger(12),)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    # [[2, 2], [3, 1]]
    assert len(result.args) == 2


def test_factor_integer_1() -> None:
    vm = make_vm()
    result = vm.eval(IRApply(_FACTOR_INTEGER, (IRInteger(1),)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 0


def test_factor_integer_negative_unevaluated() -> None:
    vm = make_vm()
    expr = IRApply(_FACTOR_INTEGER, (IRInteger(-5),))
    assert vm.eval(expr) == expr


def test_divisors_12() -> None:
    vm = make_vm()
    result = vm.eval(IRApply(_DIVISORS, (IRInteger(12),)))
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    values = [arg.value for arg in result.args]  # type: ignore[attr-defined]
    assert values == [1, 2, 3, 4, 6, 12]


def test_totient_12() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_TOTIENT, (IRInteger(12),))) == IRInteger(4)


def test_moebius_6() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_MOEBIUS_MU, (IRInteger(6),))) == IRInteger(1)


def test_moebius_4() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_MOEBIUS_MU, (IRInteger(4),))) == IRInteger(0)


def test_jacobi_symbol() -> None:
    vm = make_vm()
    # (2/3) = -1
    result = vm.eval(IRApply(_JACOBI, (IRInteger(2), IRInteger(3))))
    assert result == IRInteger(-1)


def test_chinese_remainder() -> None:
    vm = make_vm()
    remainders = ilist(IRInteger(2), IRInteger(3))
    moduli = ilist(IRInteger(3), IRInteger(5))
    result = vm.eval(IRApply(_CRT, (remainders, moduli)))
    assert result == IRInteger(8)


def test_chinese_remainder_non_coprime_unevaluated() -> None:
    vm = make_vm()
    remainders = ilist(IRInteger(0), IRInteger(0))
    moduli = ilist(IRInteger(4), IRInteger(6))
    expr = IRApply(_CRT, (remainders, moduli))
    assert vm.eval(expr) == expr


def test_integer_length_decimal() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_INT_LEN, (IRInteger(100),))) == IRInteger(3)


def test_integer_length_binary() -> None:
    vm = make_vm()
    assert vm.eval(IRApply(_INT_LEN, (IRInteger(8), IRInteger(2)))) == IRInteger(4)


def test_is_prime_symbolic_passthrough() -> None:
    vm = make_vm()
    expr = IRApply(_IS_PRIME, (IRSymbol("x"),))
    assert vm.eval(expr) == expr
