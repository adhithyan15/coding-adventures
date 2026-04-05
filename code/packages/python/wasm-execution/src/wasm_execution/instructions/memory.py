"""memory.py --- Linear memory load/store instruction handlers for WASM."""

from __future__ import annotations

from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.host_interface import TrapError
from wasm_execution.types import WasmExecutionContext
from wasm_execution.values import WasmValue, as_f32, as_f64, as_i32, as_i64, f32, f64, i32, i64


def _effective_address(base: int, memarg: dict[str, int]) -> int:
    """Calculate effective address from base + memarg offset."""
    return (base & 0xFFFFFFFF) + memarg["offset"]


def register_memory(vm: GenericVM) -> None:
    """Register all memory instruction handlers."""

    # --- i32 loads ---

    # 0x28: i32.load
    def handle_i32_load(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        vm.push_typed(i32(ctx.memory.load_i32(addr)))
        vm.advance_pc()
        return "i32.load"
    vm.register_context_opcode(0x28, handle_i32_load)

    # 0x29: i64.load
    def handle_i64_load(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        vm.push_typed(i64(ctx.memory.load_i64(addr)))
        vm.advance_pc()
        return "i64.load"
    vm.register_context_opcode(0x29, handle_i64_load)

    # 0x2A: f32.load
    def handle_f32_load(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        vm.push_typed(f32(ctx.memory.load_f32(addr)))
        vm.advance_pc()
        return "f32.load"
    vm.register_context_opcode(0x2A, handle_f32_load)

    # 0x2B: f64.load
    def handle_f64_load(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        vm.push_typed(f64(ctx.memory.load_f64(addr)))
        vm.advance_pc()
        return "f64.load"
    vm.register_context_opcode(0x2B, handle_f64_load)

    # 0x2C-0x2F: i32 narrow loads
    for opcode, op_name, load_fn in [
        (0x2C, "i32.load8_s", lambda m, a: m.load_i32_8s(a)),
        (0x2D, "i32.load8_u", lambda m, a: m.load_i32_8u(a)),
        (0x2E, "i32.load16_s", lambda m, a: m.load_i32_16s(a)),
        (0x2F, "i32.load16_u", lambda m, a: m.load_i32_16u(a)),
    ]:
        def _make_i32_load(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
                if ctx.memory is None:
                    raise TrapError("no memory")
                base = as_i32(vm.pop_typed())
                addr = _effective_address(base, instr.operand)
                vm.push_typed(i32(fn(ctx.memory, addr)))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_i32_load(op_name, load_fn))

    # 0x30-0x35: i64 narrow loads
    for opcode, op_name, load_fn in [
        (0x30, "i64.load8_s", lambda m, a: m.load_i64_8s(a)),
        (0x31, "i64.load8_u", lambda m, a: m.load_i64_8u(a)),
        (0x32, "i64.load16_s", lambda m, a: m.load_i64_16s(a)),
        (0x33, "i64.load16_u", lambda m, a: m.load_i64_16u(a)),
        (0x34, "i64.load32_s", lambda m, a: m.load_i64_32s(a)),
        (0x35, "i64.load32_u", lambda m, a: m.load_i64_32u(a)),
    ]:
        def _make_i64_load(op_n: str, fn: Any) -> Any:
            def handler(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
                if ctx.memory is None:
                    raise TrapError("no memory")
                base = as_i32(vm.pop_typed())
                addr = _effective_address(base, instr.operand)
                vm.push_typed(i64(fn(ctx.memory, addr)))
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_i64_load(op_name, load_fn))

    # --- Stores ---

    # 0x36: i32.store
    def handle_i32_store(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        value = as_i32(vm.pop_typed())
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        ctx.memory.store_i32(addr, value)
        vm.advance_pc()
        return "i32.store"
    vm.register_context_opcode(0x36, handle_i32_store)

    # 0x37: i64.store
    def handle_i64_store(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        value = as_i64(vm.pop_typed())
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        ctx.memory.store_i64(addr, value)
        vm.advance_pc()
        return "i64.store"
    vm.register_context_opcode(0x37, handle_i64_store)

    # 0x38: f32.store
    def handle_f32_store(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        value = as_f32(vm.pop_typed())
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        ctx.memory.store_f32(addr, value)
        vm.advance_pc()
        return "f32.store"
    vm.register_context_opcode(0x38, handle_f32_store)

    # 0x39: f64.store
    def handle_f64_store(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        value = as_f64(vm.pop_typed())
        base = as_i32(vm.pop_typed())
        addr = _effective_address(base, instr.operand)
        ctx.memory.store_f64(addr, value)
        vm.advance_pc()
        return "f64.store"
    vm.register_context_opcode(0x39, handle_f64_store)

    # 0x3A-0x3E: narrow stores
    for opcode, op_name, store_fn, extract in [
        (0x3A, "i32.store8", lambda m, a, v: m.store_i32_8(a, v), as_i32),
        (0x3B, "i32.store16", lambda m, a, v: m.store_i32_16(a, v), as_i32),
        (0x3C, "i64.store8", lambda m, a, v: m.store_i64_8(a, v), as_i64),
        (0x3D, "i64.store16", lambda m, a, v: m.store_i64_16(a, v), as_i64),
        (0x3E, "i64.store32", lambda m, a, v: m.store_i64_32(a, v), as_i64),
    ]:
        def _make_store(op_n: str, fn: Any, ext: Any) -> Any:
            def handler(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
                if ctx.memory is None:
                    raise TrapError("no memory")
                value = ext(vm.pop_typed())
                base = as_i32(vm.pop_typed())
                addr = _effective_address(base, instr.operand)
                fn(ctx.memory, addr, value)
                vm.advance_pc()
                return op_n
            return handler
        vm.register_context_opcode(opcode, _make_store(op_name, store_fn, extract))

    # 0x3F: memory.size
    def handle_memory_size(vm: GenericVM, _instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        vm.push_typed(i32(ctx.memory.size()))
        vm.advance_pc()
        return "memory.size"
    vm.register_context_opcode(0x3F, handle_memory_size)

    # 0x40: memory.grow
    def handle_memory_grow(vm: GenericVM, _instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if ctx.memory is None:
            raise TrapError("no memory")
        delta = as_i32(vm.pop_typed())
        result = ctx.memory.grow(delta)
        vm.push_typed(i32(result))
        vm.advance_pc()
        return "memory.grow"
    vm.register_context_opcode(0x40, handle_memory_grow)
