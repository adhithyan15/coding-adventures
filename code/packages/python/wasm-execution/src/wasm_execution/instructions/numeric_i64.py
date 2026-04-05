"""numeric_i64.py --- 64-bit integer instruction handlers for WASM.

All 32 i64 handlers. Python ints are arbitrary precision, so we mask to
64-bit signed range using ctypes.c_int64 after every operation.
"""

from __future__ import annotations

import ctypes
from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.host_interface import TrapError
from wasm_execution.values import as_i64, i32, i64

INT64_MIN = -(2**63)
MASK64 = 0xFFFFFFFFFFFFFFFF


def _to_unsigned_64(v: int) -> int:
    """Convert signed i64 to unsigned 64-bit interpretation."""
    return v & MASK64


def _ctz64(value: int) -> int:
    """Count trailing zero bits in a 64-bit integer."""
    if value == 0:
        return 64
    v = value & MASK64
    count = 0
    while (v & 1) == 0:
        count += 1
        v >>= 1
    return count


def _popcnt64(value: int) -> int:
    """Count the number of 1-bits in a 64-bit integer."""
    return bin(value & MASK64).count("1")


def _clz64(value: int) -> int:
    """Count leading zeros in a 64-bit integer."""
    v = value & MASK64
    if v == 0:
        return 64
    return 64 - v.bit_length()


def register_numeric_i64(vm: GenericVM) -> None:
    """Register all 32 i64 numeric instruction handlers."""

    # 0x42: i64.const
    def handle_i64_const(vm: GenericVM, instr: Any, _code: Any, _ctx: Any) -> str:
        vm.push_typed(i64(instr.operand))
        vm.advance_pc()
        return "i64.const"
    vm.register_context_opcode(0x42, handle_i64_const)

    # 0x50: i64.eqz
    def handle_i64_eqz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(i32(1 if a == 0 else 0))
        vm.advance_pc()
        return "i64.eqz"
    vm.register_context_opcode(0x50, handle_i64_eqz)

    # 0x51: i64.eq
    def handle_i64_eq(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        vm.push_typed(i32(1 if a == b else 0))
        vm.advance_pc()
        return "i64.eq"
    vm.register_context_opcode(0x51, handle_i64_eq)

    # 0x52: i64.ne
    def handle_i64_ne(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        vm.push_typed(i32(1 if a != b else 0))
        vm.advance_pc()
        return "i64.ne"
    vm.register_context_opcode(0x52, handle_i64_ne)

    # 0x53-0x5A: i64 comparisons
    for opcode, op_name, signed, cmp_fn in [
        (0x53, "i64.lt_s", True, lambda a, b: a < b),
        (0x54, "i64.lt_u", False, lambda a, b: a < b),
        (0x55, "i64.gt_s", True, lambda a, b: a > b),
        (0x56, "i64.gt_u", False, lambda a, b: a > b),
        (0x57, "i64.le_s", True, lambda a, b: a <= b),
        (0x58, "i64.le_u", False, lambda a, b: a <= b),
        (0x59, "i64.ge_s", True, lambda a, b: a >= b),
        (0x5A, "i64.ge_u", False, lambda a, b: a >= b),
    ]:
        def _make_cmp(op_n: str, is_signed: bool, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_raw = as_i64(vm.pop_typed())
                a_raw = as_i64(vm.pop_typed())
                if is_signed:
                    vm.push_typed(i32(1 if fn(a_raw, b_raw) else 0))
                else:
                    vm.push_typed(i32(1 if fn(_to_unsigned_64(a_raw), _to_unsigned_64(b_raw)) else 0))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_cmp(op_name, signed, cmp_fn))

    # 0x79: i64.clz
    def handle_i64_clz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(_clz64(a)))
        vm.advance_pc()
        return "i64.clz"
    vm.register_context_opcode(0x79, handle_i64_clz)

    # 0x7A: i64.ctz
    def handle_i64_ctz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(_ctz64(a)))
        vm.advance_pc()
        return "i64.ctz"
    vm.register_context_opcode(0x7A, handle_i64_ctz)

    # 0x7B: i64.popcnt
    def handle_i64_popcnt(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(_popcnt64(a)))
        vm.advance_pc()
        return "i64.popcnt"
    vm.register_context_opcode(0x7B, handle_i64_popcnt)

    # 0x7C: i64.add
    def handle_i64_add(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(ctypes.c_int64(a + b).value))
        vm.advance_pc()
        return "i64.add"
    vm.register_context_opcode(0x7C, handle_i64_add)

    # 0x7D: i64.sub
    def handle_i64_sub(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(ctypes.c_int64(a - b).value))
        vm.advance_pc()
        return "i64.sub"
    vm.register_context_opcode(0x7D, handle_i64_sub)

    # 0x7E: i64.mul
    def handle_i64_mul(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        vm.push_typed(i64(ctypes.c_int64(a * b).value))
        vm.advance_pc()
        return "i64.mul"
    vm.register_context_opcode(0x7E, handle_i64_mul)

    # 0x7F: i64.div_s
    def handle_i64_div_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        if b == 0:
            raise TrapError("integer divide by zero")
        if a == INT64_MIN and b == -1:
            raise TrapError("integer overflow")
        result = int(a / b)  # truncation toward zero
        vm.push_typed(i64(result))
        vm.advance_pc()
        return "i64.div_s"
    vm.register_context_opcode(0x7F, handle_i64_div_s)

    # 0x80: i64.div_u
    def handle_i64_div_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned_64(as_i64(vm.pop_typed()))
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        if b == 0:
            raise TrapError("integer divide by zero")
        vm.push_typed(i64(a // b))
        vm.advance_pc()
        return "i64.div_u"
    vm.register_context_opcode(0x80, handle_i64_div_u)

    # 0x81: i64.rem_s
    def handle_i64_rem_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        if b == 0:
            raise TrapError("integer divide by zero")
        if a == INT64_MIN and b == -1:
            vm.push_typed(i64(0))
        else:
            result = a - int(a / b) * b
            vm.push_typed(i64(result))
        vm.advance_pc()
        return "i64.rem_s"
    vm.register_context_opcode(0x81, handle_i64_rem_s)

    # 0x82: i64.rem_u
    def handle_i64_rem_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned_64(as_i64(vm.pop_typed()))
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        if b == 0:
            raise TrapError("integer divide by zero")
        vm.push_typed(i64(a % b))
        vm.advance_pc()
        return "i64.rem_u"
    vm.register_context_opcode(0x82, handle_i64_rem_u)

    # 0x83-0x8A: i64 bitwise and shift ops
    for opcode, op_name, op_fn in [
        (0x83, "i64.and", lambda a, b: a & b),
        (0x84, "i64.or", lambda a, b: a | b),
        (0x85, "i64.xor", lambda a, b: a ^ b),
    ]:
        def _make_bitwise(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_val = as_i64(vm.pop_typed())
                a_val = as_i64(vm.pop_typed())
                vm.push_typed(i64(ctypes.c_int64(fn(a_val, b_val)).value))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_bitwise(op_name, op_fn))

    # 0x86: i64.shl
    def handle_i64_shl(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        n = b & 63
        vm.push_typed(i64(ctypes.c_int64(a << n).value))
        vm.advance_pc()
        return "i64.shl"
    vm.register_context_opcode(0x86, handle_i64_shl)

    # 0x87: i64.shr_s
    def handle_i64_shr_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = as_i64(vm.pop_typed())
        n = b & 63
        vm.push_typed(i64(a >> n))
        vm.advance_pc()
        return "i64.shr_s"
    vm.register_context_opcode(0x87, handle_i64_shr_s)

    # 0x88: i64.shr_u
    def handle_i64_shr_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        n = b & 63
        vm.push_typed(i64(ctypes.c_int64(a >> n).value))
        vm.advance_pc()
        return "i64.shr_u"
    vm.register_context_opcode(0x88, handle_i64_shr_u)

    # 0x89: i64.rotl
    def handle_i64_rotl(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        n = b & 63
        result = ((a << n) | (a >> (64 - n))) & MASK64
        vm.push_typed(i64(ctypes.c_int64(result).value))
        vm.advance_pc()
        return "i64.rotl"
    vm.register_context_opcode(0x89, handle_i64_rotl)

    # 0x8A: i64.rotr
    def handle_i64_rotr(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i64(vm.pop_typed())
        a = _to_unsigned_64(as_i64(vm.pop_typed()))
        n = b & 63
        result = ((a >> n) | (a << (64 - n))) & MASK64
        vm.push_typed(i64(ctypes.c_int64(result).value))
        vm.advance_pc()
        return "i64.rotr"
    vm.register_context_opcode(0x8A, handle_i64_rotr)
