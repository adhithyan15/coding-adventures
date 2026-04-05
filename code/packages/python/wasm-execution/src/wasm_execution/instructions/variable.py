"""variable.py --- Local and global variable access instruction handlers."""

from __future__ import annotations

from typing import Any

from virtual_machine.generic_vm import GenericVM

from wasm_execution.types import WasmExecutionContext


def register_variable(vm: GenericVM) -> None:
    """Register all 5 variable access instruction handlers."""

    # 0x20: local.get
    def handle_local_get(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        index = instr.operand
        vm.push_typed(ctx.typed_locals[index])
        vm.advance_pc()
        return "local.get"
    vm.register_context_opcode(0x20, handle_local_get)

    # 0x21: local.set
    def handle_local_set(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        index = instr.operand
        ctx.typed_locals[index] = vm.pop_typed()
        vm.advance_pc()
        return "local.set"
    vm.register_context_opcode(0x21, handle_local_set)

    # 0x22: local.tee --- write local WITHOUT popping
    def handle_local_tee(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        index = instr.operand
        ctx.typed_locals[index] = vm.peek_typed()
        vm.advance_pc()
        return "local.tee"
    vm.register_context_opcode(0x22, handle_local_tee)

    # 0x23: global.get
    def handle_global_get(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        index = instr.operand
        vm.push_typed(ctx.globals[index])
        vm.advance_pc()
        return "global.get"
    vm.register_context_opcode(0x23, handle_global_get)

    # 0x24: global.set
    def handle_global_set(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        index = instr.operand
        ctx.globals[index] = vm.pop_typed()
        vm.advance_pc()
        return "global.set"
    vm.register_context_opcode(0x24, handle_global_set)
