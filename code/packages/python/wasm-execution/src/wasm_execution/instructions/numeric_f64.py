"""numeric_f64.py --- 64-bit float instruction handlers for WASM."""

from __future__ import annotations

import math
from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.values import as_f64, f64, i32


def register_numeric_f64(vm: GenericVM) -> None:
    """Register all f64 numeric instruction handlers."""

    # 0x44: f64.const
    def handle_f64_const(vm: GenericVM, instr: Any, _code: Any, _ctx: Any) -> str:
        vm.push_typed(f64(instr.operand))
        vm.advance_pc()
        return "f64.const"
    vm.register_context_opcode(0x44, handle_f64_const)

    # Comparisons: result is i32
    for opcode, op_name, cmp_fn in [
        (0x61, "f64.eq", lambda a, b: a == b),
        (0x62, "f64.ne", lambda a, b: a != b),
        (0x63, "f64.lt", lambda a, b: a < b),
        (0x64, "f64.gt", lambda a, b: a > b),
        (0x65, "f64.le", lambda a, b: a <= b),
        (0x66, "f64.ge", lambda a, b: a >= b),
    ]:
        def _make_cmp(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_val = as_f64(vm.pop_typed())
                a_val = as_f64(vm.pop_typed())
                vm.push_typed(i32(1 if fn(a_val, b_val) else 0))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_cmp(op_name, cmp_fn))

    # Unary operations
    for opcode, op_name, op_fn in [
        (0x99, "f64.abs", lambda a: abs(a)),
        (0x9A, "f64.neg", lambda a: -a),
        (0x9B, "f64.ceil", lambda a: math.ceil(a) if not math.isnan(a) else a),
        (0x9C, "f64.floor", lambda a: math.floor(a) if not math.isnan(a) else a),
        (0x9D, "f64.trunc", lambda a: math.trunc(a) if not math.isnan(a) and not math.isinf(a) else a),
        (0x9F, "f64.sqrt", lambda a: math.sqrt(a) if a >= 0 or math.isnan(a) else float("nan")),
    ]:
        def _make_unary(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                a_val = as_f64(vm.pop_typed())
                vm.push_typed(f64(fn(a_val)))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_unary(op_name, op_fn))

    # 0x9E: f64.nearest
    def handle_f64_nearest(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        a = as_f64(vm.pop_typed())
        if math.isnan(a) or math.isinf(a) or a == 0.0:
            vm.push_typed(f64(a))
        else:
            vm.push_typed(f64(round(a)))
        vm.advance_pc()
        return "f64.nearest"
    vm.register_context_opcode(0x9E, handle_f64_nearest)

    # Binary arithmetic
    for opcode, op_name, op_fn in [
        (0xA0, "f64.add", lambda a, b: a + b),
        (0xA1, "f64.sub", lambda a, b: a - b),
        (0xA2, "f64.mul", lambda a, b: a * b),
        (0xA3, "f64.div", lambda a, b: a / b if b != 0 else (float("nan") if a == 0 else math.copysign(float("inf"), a * b) if b == 0.0 else a / b)),
    ]:
        def _make_bin(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
                b_val = as_f64(vm.pop_typed())
                a_val = as_f64(vm.pop_typed())
                vm.push_typed(f64(fn(a_val, b_val)))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_bin(op_name, op_fn))

    # 0xA4: f64.min
    def handle_f64_min(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f64(vm.pop_typed())
        a = as_f64(vm.pop_typed())
        if math.isnan(a) or math.isnan(b):
            vm.push_typed(f64(float("nan")))
        else:
            vm.push_typed(f64(min(a, b) if a != b else (a if math.copysign(1.0, a) < 0 else b)))
        vm.advance_pc()
        return "f64.min"
    vm.register_context_opcode(0xA4, handle_f64_min)

    # 0xA5: f64.max
    def handle_f64_max(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f64(vm.pop_typed())
        a = as_f64(vm.pop_typed())
        if math.isnan(a) or math.isnan(b):
            vm.push_typed(f64(float("nan")))
        else:
            vm.push_typed(f64(max(a, b) if a != b else (a if math.copysign(1.0, a) > 0 else b)))
        vm.advance_pc()
        return "f64.max"
    vm.register_context_opcode(0xA5, handle_f64_max)

    # 0xA6: f64.copysign
    def handle_f64_copysign(vm: GenericVM, _instr: Any, _code: Any, _ctx: Any) -> str:
        b = as_f64(vm.pop_typed())
        a = as_f64(vm.pop_typed())
        vm.push_typed(f64(math.copysign(a, b)))
        vm.advance_pc()
        return "f64.copysign"
    vm.register_context_opcode(0xA6, handle_f64_copysign)
