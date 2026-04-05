"""numeric_f32.py --- 32-bit float instruction handlers for WASM."""

from __future__ import annotations

import math
import struct
from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.host_interface import TrapError
from wasm_execution.values import as_f32, f32, i32


def _f32_wrap(v: float) -> float:
    """Round a float to f32 precision."""
    return struct.unpack("<f", struct.pack("<f", v))[0]


def register_numeric_f32(vm: GenericVM) -> None:
    """Register all f32 numeric instruction handlers."""

    # 0x43: f32.const
    def handle_f32_const(vm: GenericVM, instr: Any, _code: Any, _ctx: Any) -> str:
        vm.push_typed(f32(instr.operand))
        vm.advance_pc()
        return "f32.const"
    vm.register_context_opcode(0x43, handle_f32_const)

    # Comparisons: result is i32
    for opcode, op_name, cmp_fn in [
        (0x5B, "f32.eq", lambda a, b: a == b),
        (0x5C, "f32.ne", lambda a, b: a != b),
        (0x5D, "f32.lt", lambda a, b: a < b),
        (0x5E, "f32.gt", lambda a, b: a > b),
        (0x5F, "f32.le", lambda a, b: a <= b),
        (0x60, "f32.ge", lambda a, b: a >= b),
    ]:
        def _make_cmp(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_val = as_f32(vm.pop_typed())
                a_val = as_f32(vm.pop_typed())
                vm.push_typed(i32(1 if fn(a_val, b_val) else 0))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_cmp(op_name, cmp_fn))

    # Unary operations
    # 0x8B: f32.abs
    def handle_f32_abs(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(abs(a)))
        vm.advance_pc()
        return "f32.abs"
    vm.register_context_opcode(0x8B, handle_f32_abs)

    # 0x8C: f32.neg
    def handle_f32_neg(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(-a))
        vm.advance_pc()
        return "f32.neg"
    vm.register_context_opcode(0x8C, handle_f32_neg)

    # 0x8D: f32.ceil
    def handle_f32_ceil(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(math.ceil(a) if not math.isnan(a) else a))
        vm.advance_pc()
        return "f32.ceil"
    vm.register_context_opcode(0x8D, handle_f32_ceil)

    # 0x8E: f32.floor
    def handle_f32_floor(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(math.floor(a) if not math.isnan(a) else a))
        vm.advance_pc()
        return "f32.floor"
    vm.register_context_opcode(0x8E, handle_f32_floor)

    # 0x8F: f32.trunc
    def handle_f32_trunc(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(math.trunc(a) if not math.isnan(a) and not math.isinf(a) else a))
        vm.advance_pc()
        return "f32.trunc"
    vm.register_context_opcode(0x8F, handle_f32_trunc)

    # 0x90: f32.nearest
    def handle_f32_nearest(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        if math.isnan(a) or math.isinf(a) or a == 0.0:
            vm.push_typed(f32(a))
        else:
            # Round to nearest even (Python's default)
            vm.push_typed(f32(round(a)))
        vm.advance_pc()
        return "f32.nearest"
    vm.register_context_opcode(0x90, handle_f32_nearest)

    # 0x91: f32.sqrt
    def handle_f32_sqrt(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(math.sqrt(a) if a >= 0 or math.isnan(a) else float("nan")))
        vm.advance_pc()
        return "f32.sqrt"
    vm.register_context_opcode(0x91, handle_f32_sqrt)

    # Binary arithmetic: f32 result
    for opcode, op_name, op_fn in [
        (0x92, "f32.add", lambda a, b: a + b),
        (0x93, "f32.sub", lambda a, b: a - b),
        (0x94, "f32.mul", lambda a, b: a * b),
        (0x95, "f32.div", lambda a, b: a / b if b != 0 else (float("nan") if a == 0 else math.copysign(float("inf"), a * b) if b == 0.0 else a / b)),
    ]:
        def _make_bin(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_val = as_f32(vm.pop_typed())
                a_val = as_f32(vm.pop_typed())
                vm.push_typed(f32(_f32_wrap(fn(a_val, b_val))))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_bin(op_name, op_fn))

    # 0x96: f32.min
    def handle_f32_min(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f32(vm.pop_typed())
        a = as_f32(vm.pop_typed())
        if math.isnan(a) or math.isnan(b):
            vm.push_typed(f32(float("nan")))
        else:
            vm.push_typed(f32(min(a, b) if a != b else (a if math.copysign(1.0, a) < 0 else b)))
        vm.advance_pc()
        return "f32.min"
    vm.register_context_opcode(0x96, handle_f32_min)

    # 0x97: f32.max
    def handle_f32_max(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f32(vm.pop_typed())
        a = as_f32(vm.pop_typed())
        if math.isnan(a) or math.isnan(b):
            vm.push_typed(f32(float("nan")))
        else:
            vm.push_typed(f32(max(a, b) if a != b else (a if math.copysign(1.0, a) > 0 else b)))
        vm.advance_pc()
        return "f32.max"
    vm.register_context_opcode(0x97, handle_f32_max)

    # 0x98: f32.copysign
    def handle_f32_copysign(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f32(vm.pop_typed())
        a = as_f32(vm.pop_typed())
        vm.push_typed(f32(math.copysign(a, b)))
        vm.advance_pc()
        return "f32.copysign"
    vm.register_context_opcode(0x98, handle_f32_copysign)
