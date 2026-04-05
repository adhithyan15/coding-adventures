"""numeric_i32.py --- 32-bit integer instruction handlers for WASM.

All 33 i32 handlers. Python ints have arbitrary precision, so we MUST mask
to 32 bits after every arithmetic operation. We use ctypes.c_int32 for
signed wrapping and ``& 0xFFFFFFFF`` for unsigned interpretation.

Pop order: b first (top of stack), then a. The operation is ``a <op> b``.
"""

from __future__ import annotations

import ctypes
import math
from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.host_interface import TrapError
from wasm_execution.values import WasmValue, as_i32, i32

INT32_MIN = -2147483648
MASK32 = 0xFFFFFFFF


def _to_unsigned(v: int) -> int:
    """Convert signed i32 to unsigned 32-bit interpretation."""
    return v & MASK32


def _ctz32(value: int) -> int:
    """Count trailing zero bits in a 32-bit integer."""
    if value == 0:
        return 32
    v = value & MASK32
    count = 0
    while (v & 1) == 0:
        count += 1
        v >>= 1
    return count


def _popcnt32(value: int) -> int:
    """Count the number of 1-bits in a 32-bit integer."""
    v = value & MASK32
    v = v - ((v >> 1) & 0x55555555)
    v = (v & 0x33333333) + ((v >> 2) & 0x33333333)
    v = (v + (v >> 4)) & 0x0F0F0F0F
    return ((v * 0x01010101) >> 24) & 0xFF


def register_numeric_i32(vm: GenericVM) -> None:
    """Register all 33 i32 numeric instruction handlers."""

    # 0x41: i32.const
    def handle_i32_const(vm: GenericVM, instr: Any, _code: Any, _ctx: Any) -> str:
        vm.push_typed(i32(instr.operand))
        vm.advance_pc()
        return "i32.const"
    vm.register_context_opcode(0x41, handle_i32_const)

    # 0x45: i32.eqz
    def handle_i32_eqz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a == 0 else 0))
        vm.advance_pc()
        return "i32.eqz"
    vm.register_context_opcode(0x45, handle_i32_eqz)

    # 0x46: i32.eq
    def handle_i32_eq(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a == b else 0))
        vm.advance_pc()
        return "i32.eq"
    vm.register_context_opcode(0x46, handle_i32_eq)

    # 0x47: i32.ne
    def handle_i32_ne(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a != b else 0))
        vm.advance_pc()
        return "i32.ne"
    vm.register_context_opcode(0x47, handle_i32_ne)

    # 0x48: i32.lt_s
    def handle_i32_lt_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a < b else 0))
        vm.advance_pc()
        return "i32.lt_s"
    vm.register_context_opcode(0x48, handle_i32_lt_s)

    # 0x49: i32.lt_u
    def handle_i32_lt_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        vm.push_typed(i32(1 if a < b else 0))
        vm.advance_pc()
        return "i32.lt_u"
    vm.register_context_opcode(0x49, handle_i32_lt_u)

    # 0x4A: i32.gt_s
    def handle_i32_gt_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a > b else 0))
        vm.advance_pc()
        return "i32.gt_s"
    vm.register_context_opcode(0x4A, handle_i32_gt_s)

    # 0x4B: i32.gt_u
    def handle_i32_gt_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        vm.push_typed(i32(1 if a > b else 0))
        vm.advance_pc()
        return "i32.gt_u"
    vm.register_context_opcode(0x4B, handle_i32_gt_u)

    # 0x4C: i32.le_s
    def handle_i32_le_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a <= b else 0))
        vm.advance_pc()
        return "i32.le_s"
    vm.register_context_opcode(0x4C, handle_i32_le_s)

    # 0x4D: i32.le_u
    def handle_i32_le_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        vm.push_typed(i32(1 if a <= b else 0))
        vm.advance_pc()
        return "i32.le_u"
    vm.register_context_opcode(0x4D, handle_i32_le_u)

    # 0x4E: i32.ge_s
    def handle_i32_ge_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(1 if a >= b else 0))
        vm.advance_pc()
        return "i32.ge_s"
    vm.register_context_opcode(0x4E, handle_i32_ge_s)

    # 0x4F: i32.ge_u
    def handle_i32_ge_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        vm.push_typed(i32(1 if a >= b else 0))
        vm.advance_pc()
        return "i32.ge_u"
    vm.register_context_opcode(0x4F, handle_i32_ge_u)

    # 0x67: i32.clz
    def handle_i32_clz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = _to_unsigned(as_i32(vm.pop_typed()))
        if a == 0:
            vm.push_typed(i32(32))
        else:
            # Count leading zeros: 32 - bit_length gives us leading zeros
            vm.push_typed(i32(32 - a.bit_length()))
        vm.advance_pc()
        return "i32.clz"
    vm.register_context_opcode(0x67, handle_i32_clz)

    # 0x68: i32.ctz
    def handle_i32_ctz(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(_ctz32(a)))
        vm.advance_pc()
        return "i32.ctz"
    vm.register_context_opcode(0x68, handle_i32_ctz)

    # 0x69: i32.popcnt
    def handle_i32_popcnt(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(_popcnt32(a)))
        vm.advance_pc()
        return "i32.popcnt"
    vm.register_context_opcode(0x69, handle_i32_popcnt)

    # 0x6A: i32.add
    def handle_i32_add(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a + b).value))
        vm.advance_pc()
        return "i32.add"
    vm.register_context_opcode(0x6A, handle_i32_add)

    # 0x6B: i32.sub
    def handle_i32_sub(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a - b).value))
        vm.advance_pc()
        return "i32.sub"
    vm.register_context_opcode(0x6B, handle_i32_sub)

    # 0x6C: i32.mul
    def handle_i32_mul(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a * b).value))
        vm.advance_pc()
        return "i32.mul"
    vm.register_context_opcode(0x6C, handle_i32_mul)

    # 0x6D: i32.div_s
    def handle_i32_div_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        if b == 0:
            raise TrapError("integer divide by zero")
        if a == INT32_MIN and b == -1:
            raise TrapError("integer overflow")
        # Python's // does floor division; we need truncation toward zero
        result = int(a / b)
        vm.push_typed(i32(result))
        vm.advance_pc()
        return "i32.div_s"
    vm.register_context_opcode(0x6D, handle_i32_div_s)

    # 0x6E: i32.div_u
    def handle_i32_div_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        if b == 0:
            raise TrapError("integer divide by zero")
        vm.push_typed(i32(a // b))
        vm.advance_pc()
        return "i32.div_u"
    vm.register_context_opcode(0x6E, handle_i32_div_u)

    # 0x6F: i32.rem_s
    def handle_i32_rem_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        if b == 0:
            raise TrapError("integer divide by zero")
        if a == INT32_MIN and b == -1:
            vm.push_typed(i32(0))
        else:
            # WASM remainder has the sign of the dividend (truncated division)
            result = int(math.remainder(a, b)) if False else a - int(a / b) * b
            vm.push_typed(i32(result))
        vm.advance_pc()
        return "i32.rem_s"
    vm.register_context_opcode(0x6F, handle_i32_rem_s)

    # 0x70: i32.rem_u
    def handle_i32_rem_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = _to_unsigned(as_i32(vm.pop_typed()))
        a = _to_unsigned(as_i32(vm.pop_typed()))
        if b == 0:
            raise TrapError("integer divide by zero")
        vm.push_typed(i32(a % b))
        vm.advance_pc()
        return "i32.rem_u"
    vm.register_context_opcode(0x70, handle_i32_rem_u)

    # 0x71: i32.and
    def handle_i32_and(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a & b).value))
        vm.advance_pc()
        return "i32.and"
    vm.register_context_opcode(0x71, handle_i32_and)

    # 0x72: i32.or
    def handle_i32_or(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a | b).value))
        vm.advance_pc()
        return "i32.or"
    vm.register_context_opcode(0x72, handle_i32_or)

    # 0x73: i32.xor
    def handle_i32_xor(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a ^ b).value))
        vm.advance_pc()
        return "i32.xor"
    vm.register_context_opcode(0x73, handle_i32_xor)

    # 0x74: i32.shl
    def handle_i32_shl(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        n = b & 31
        vm.push_typed(i32(ctypes.c_int32(_to_unsigned(a) << n).value))
        vm.advance_pc()
        return "i32.shl"
    vm.register_context_opcode(0x74, handle_i32_shl)

    # 0x75: i32.shr_s (arithmetic shift right, sign-extending)
    def handle_i32_shr_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = as_i32(vm.pop_typed())
        n = b & 31
        vm.push_typed(i32(a >> n))
        vm.advance_pc()
        return "i32.shr_s"
    vm.register_context_opcode(0x75, handle_i32_shr_s)

    # 0x76: i32.shr_u (logical shift right, zero-filling)
    def handle_i32_shr_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = _to_unsigned(as_i32(vm.pop_typed()))
        n = b & 31
        vm.push_typed(i32(ctypes.c_int32(a >> n).value))
        vm.advance_pc()
        return "i32.shr_u"
    vm.register_context_opcode(0x76, handle_i32_shr_u)

    # 0x77: i32.rotl
    def handle_i32_rotl(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = _to_unsigned(as_i32(vm.pop_typed()))
        n = b & 31
        result = ((a << n) | (a >> (32 - n))) & MASK32
        vm.push_typed(i32(ctypes.c_int32(result).value))
        vm.advance_pc()
        return "i32.rotl"
    vm.register_context_opcode(0x77, handle_i32_rotl)

    # 0x78: i32.rotr
    def handle_i32_rotr(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_i32(vm.pop_typed())
        a = _to_unsigned(as_i32(vm.pop_typed()))
        n = b & 31
        result = ((a >> n) | (a << (32 - n))) & MASK32
        vm.push_typed(i32(ctypes.c_int32(result).value))
        vm.advance_pc()
        return "i32.rotr"
    vm.register_context_opcode(0x78, handle_i32_rotr)
