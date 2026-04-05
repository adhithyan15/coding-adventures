"""conversion.py --- Type conversion instruction handlers for WASM.

27 handlers for converting between i32, i64, f32, and f64 types.
"""

from __future__ import annotations

import ctypes
import math
import struct
from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.host_interface import TrapError
from wasm_execution.values import as_f32, as_f64, as_i32, as_i64, f32, f64, i32, i64

MASK32 = 0xFFFFFFFF
MASK64 = 0xFFFFFFFFFFFFFFFF


def register_conversion(vm: GenericVM) -> None:
    """Register all 27 conversion instruction handlers."""

    # 0xA7: i32.wrap_i64 --- truncate i64 to i32
    def handle_i32_wrap_i64(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(i32(ctypes.c_int32(a & MASK32).value))
        vm.advance_pc()
        return "i32.wrap_i64"
    vm.register_context_opcode(0xA7, handle_i32_wrap_i64)

    # 0xA8: i32.trunc_f32_s
    def handle_i32_trunc_f32_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < -2147483648 or t > 2147483647:
            raise TrapError("integer overflow")
        vm.push_typed(i32(int(t)))
        vm.advance_pc()
        return "i32.trunc_f32_s"
    vm.register_context_opcode(0xA8, handle_i32_trunc_f32_s)

    # 0xA9: i32.trunc_f32_u
    def handle_i32_trunc_f32_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < 0 or t > 4294967295:
            raise TrapError("integer overflow")
        vm.push_typed(i32(ctypes.c_int32(int(t)).value))
        vm.advance_pc()
        return "i32.trunc_f32_u"
    vm.register_context_opcode(0xA9, handle_i32_trunc_f32_u)

    # 0xAA: i32.trunc_f64_s
    def handle_i32_trunc_f64_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < -2147483648 or t > 2147483647:
            raise TrapError("integer overflow")
        vm.push_typed(i32(int(t)))
        vm.advance_pc()
        return "i32.trunc_f64_s"
    vm.register_context_opcode(0xAA, handle_i32_trunc_f64_s)

    # 0xAB: i32.trunc_f64_u
    def handle_i32_trunc_f64_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < 0 or t > 4294967295:
            raise TrapError("integer overflow")
        vm.push_typed(i32(ctypes.c_int32(int(t)).value))
        vm.advance_pc()
        return "i32.trunc_f64_u"
    vm.register_context_opcode(0xAB, handle_i32_trunc_f64_u)

    # 0xAC: i64.extend_i32_s
    def handle_i64_extend_i32_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(i64(a))  # already signed
        vm.advance_pc()
        return "i64.extend_i32_s"
    vm.register_context_opcode(0xAC, handle_i64_extend_i32_s)

    # 0xAD: i64.extend_i32_u
    def handle_i64_extend_i32_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(i64(a & MASK32))
        vm.advance_pc()
        return "i64.extend_i32_u"
    vm.register_context_opcode(0xAD, handle_i64_extend_i32_u)

    # 0xAE: i64.trunc_f32_s
    def handle_i64_trunc_f32_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < -(2**63) or t > 2**63 - 1:
            raise TrapError("integer overflow")
        vm.push_typed(i64(int(t)))
        vm.advance_pc()
        return "i64.trunc_f32_s"
    vm.register_context_opcode(0xAE, handle_i64_trunc_f32_s)

    # 0xAF: i64.trunc_f32_u
    def handle_i64_trunc_f32_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < 0 or t > 2**64 - 1:
            raise TrapError("integer overflow")
        vm.push_typed(i64(ctypes.c_int64(int(t)).value))
        vm.advance_pc()
        return "i64.trunc_f32_u"
    vm.register_context_opcode(0xAF, handle_i64_trunc_f32_u)

    # 0xB0: i64.trunc_f64_s
    def handle_i64_trunc_f64_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < -(2**63) or t > 2**63 - 1:
            raise TrapError("integer overflow")
        vm.push_typed(i64(int(t)))
        vm.advance_pc()
        return "i64.trunc_f64_s"
    vm.register_context_opcode(0xB0, handle_i64_trunc_f64_s)

    # 0xB1: i64.trunc_f64_u
    def handle_i64_trunc_f64_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        if math.isnan(a):
            raise TrapError("invalid conversion to integer")
        t = math.trunc(a)
        if t < 0 or t > 2**64 - 1:
            raise TrapError("integer overflow")
        vm.push_typed(i64(ctypes.c_int64(int(t)).value))
        vm.advance_pc()
        return "i64.trunc_f64_u"
    vm.register_context_opcode(0xB1, handle_i64_trunc_f64_u)

    # 0xB2: f32.convert_i32_s
    def handle_f32_convert_i32_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(f32(float(a)))
        vm.advance_pc()
        return "f32.convert_i32_s"
    vm.register_context_opcode(0xB2, handle_f32_convert_i32_s)

    # 0xB3: f32.convert_i32_u
    def handle_f32_convert_i32_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(f32(float(a & MASK32)))
        vm.advance_pc()
        return "f32.convert_i32_u"
    vm.register_context_opcode(0xB3, handle_f32_convert_i32_u)

    # 0xB4: f32.convert_i64_s
    def handle_f32_convert_i64_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(f32(float(a)))
        vm.advance_pc()
        return "f32.convert_i64_s"
    vm.register_context_opcode(0xB4, handle_f32_convert_i64_s)

    # 0xB5: f32.convert_i64_u
    def handle_f32_convert_i64_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(f32(float(a & MASK64)))
        vm.advance_pc()
        return "f32.convert_i64_u"
    vm.register_context_opcode(0xB5, handle_f32_convert_i64_u)

    # 0xB6: f32.demote_f64
    def handle_f32_demote_f64(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        vm.push_typed(f32(a))
        vm.advance_pc()
        return "f32.demote_f64"
    vm.register_context_opcode(0xB6, handle_f32_demote_f64)

    # 0xB7: f64.convert_i32_s
    def handle_f64_convert_i32_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(f64(float(a)))
        vm.advance_pc()
        return "f64.convert_i32_s"
    vm.register_context_opcode(0xB7, handle_f64_convert_i32_s)

    # 0xB8: f64.convert_i32_u
    def handle_f64_convert_i32_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        vm.push_typed(f64(float(a & MASK32)))
        vm.advance_pc()
        return "f64.convert_i32_u"
    vm.register_context_opcode(0xB8, handle_f64_convert_i32_u)

    # 0xB9: f64.convert_i64_s
    def handle_f64_convert_i64_s(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(f64(float(a)))
        vm.advance_pc()
        return "f64.convert_i64_s"
    vm.register_context_opcode(0xB9, handle_f64_convert_i64_s)

    # 0xBA: f64.convert_i64_u
    def handle_f64_convert_i64_u(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        vm.push_typed(f64(float(a & MASK64)))
        vm.advance_pc()
        return "f64.convert_i64_u"
    vm.register_context_opcode(0xBA, handle_f64_convert_i64_u)

    # 0xBB: f64.promote_f32
    def handle_f64_promote_f32(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f64(float(a)))
        vm.advance_pc()
        return "f64.promote_f32"
    vm.register_context_opcode(0xBB, handle_f64_promote_f32)

    # 0xBC: i32.reinterpret_f32 --- reinterpret f32 bits as i32
    def handle_i32_reinterpret_f32(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        bits = struct.unpack("<i", struct.pack("<f", a))[0]
        vm.push_typed(i32(bits))
        vm.advance_pc()
        return "i32.reinterpret_f32"
    vm.register_context_opcode(0xBC, handle_i32_reinterpret_f32)

    # 0xBD: i64.reinterpret_f64 --- reinterpret f64 bits as i64
    def handle_i64_reinterpret_f64(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        bits = struct.unpack("<q", struct.pack("<d", a))[0]
        vm.push_typed(i64(bits))
        vm.advance_pc()
        return "i64.reinterpret_f64"
    vm.register_context_opcode(0xBD, handle_i64_reinterpret_f64)

    # 0xBE: f32.reinterpret_i32 --- reinterpret i32 bits as f32
    def handle_f32_reinterpret_i32(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i32(vm.pop_typed())
        val = struct.unpack("<f", struct.pack("<i", a))[0]
        vm.push_typed(f32(val))
        vm.advance_pc()
        return "f32.reinterpret_i32"
    vm.register_context_opcode(0xBE, handle_f32_reinterpret_i32)

    # 0xBF: f64.reinterpret_i64 --- reinterpret i64 bits as f64
    def handle_f64_reinterpret_i64(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_i64(vm.pop_typed())
        val = struct.unpack("<d", struct.pack("<q", a))[0]
        vm.push_typed(f64(val))
        vm.advance_pc()
        return "f64.reinterpret_i64"
    vm.register_context_opcode(0xBF, handle_f64_reinterpret_i64)
