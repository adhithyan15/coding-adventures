"""parametric.py --- Parametric instruction handlers (drop and select)."""

from __future__ import annotations

from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.types import WasmExecutionContext


def register_parametric(vm: GenericVM) -> None:
    """Register the 2 parametric instruction handlers."""

    # 0x1A: drop --- discard the top stack value
    def handle_drop(vm: GenericVM, _instr: Any, _code: Any, _ctx: WasmExecutionContext) -> str:
        vm.pop_typed()
        vm.advance_pc()
        return "drop"
    vm.register_context_opcode(0x1A, handle_drop)

    # 0x1B: select --- conditional pick
    # Pop order: condition (i32), val2, val1.
    # Push val1 if condition != 0, else val2.
    def handle_select(vm: GenericVM, _instr: Any, _code: Any, _ctx: WasmExecutionContext) -> str:
        condition = vm.pop_typed()
        val2 = vm.pop_typed()
        val1 = vm.pop_typed()
        vm.push_typed(val1 if condition.value != 0 else val2)
        vm.advance_pc()
        return "select"
    vm.register_context_opcode(0x1B, handle_select)
