"""control.py --- Control flow instruction handlers for WASM.

Handles block, loop, if/else, end, br, br_if, br_table, return, call,
call_indirect, unreachable, and nop.
"""

from __future__ import annotations

from typing import Any

from virtual_machine.generic_vm import GenericVM
from wasm_types import ValueType

from wasm_execution.decoder import build_control_flow_map, decode_function_body, to_vm_instructions
from wasm_execution.host_interface import TrapError
from wasm_execution.types import Label, WasmExecutionContext
from wasm_execution.values import WasmValue, default_value


# ===========================================================================
# Helpers
# ===========================================================================


def _block_arity(block_type: int | None, func_types: list[Any]) -> int:
    """Resolve a block type to its result arity."""
    if block_type is None or block_type == 0x40:
        return 0
    if block_type in (ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64):
        return 1
    if isinstance(block_type, int) and 0 <= block_type < len(func_types):
        return len(func_types[block_type].results)
    return 0


def _execute_branch(
    vm: GenericVM,
    ctx: WasmExecutionContext,
    label_index: int,
) -> None:
    """Execute a branch to the Nth label from the top of the label stack."""
    label_stack_index = len(ctx.label_stack) - 1 - label_index
    if label_stack_index < 0:
        msg = f"branch target {label_index} out of range"
        raise TrapError(msg)

    label = ctx.label_stack[label_stack_index]

    # For loops, arity is 0 in WASM 1.0 MVP; for blocks, it's the result arity
    arity = 0 if label.is_loop else label.arity

    # Save result values from the top of the stack
    results: list[WasmValue] = []
    for _ in range(arity):
        results.insert(0, vm.pop_typed())

    # Unwind the value stack to the label's recorded height
    while len(vm.typed_stack) > label.stack_height:
        vm.pop_typed()

    # Push the result values back
    for v in results:
        vm.push_typed(v)

    # Pop labels down to and including the target label
    ctx.label_stack = ctx.label_stack[:label_stack_index]

    # Branching to a block/if exits that construct, so execution resumes
    # after its ``end``. Branching to a loop re-enters at the loop header.
    vm.jump_to(label.target_pc if label.is_loop else label.target_pc + 1)


def _call_function(
    vm: GenericVM,
    code: Any,
    ctx: WasmExecutionContext,
    func_index: int,
) -> None:
    """Call a function by index (host or module-defined)."""
    func_type = ctx.func_types[func_index]
    if func_type is None:
        msg = f"undefined function {func_index}"
        raise TrapError(msg)

    # Pop arguments from the stack (in reverse order)
    args: list[WasmValue] = []
    for _ in range(len(func_type.params)):
        args.insert(0, vm.pop_typed())

    # Check if this is a host function
    host_func = ctx.host_functions[func_index]
    if host_func is not None:
        results = host_func.call(args)
        for r in results:
            vm.push_typed(r)
        vm.advance_pc()
        return

    # Module-defined function --- set up a new frame
    body = ctx.func_bodies[func_index]
    if body is None:
        msg = f"no body for function {func_index}"
        raise TrapError(msg)

    # Save caller's state
    ctx.saved_frames.append(
        _make_saved_frame(
            ctx,
            vm,
            code,
            return_arity=len(func_type.results),
        )
    )

    # Initialize callee's locals
    ctx.typed_locals = [
        *args,
        *[default_value(t) for t in body.locals],
    ]

    # Clear label stack for new frame
    ctx.label_stack = []

    # Decode callee's function body and build control flow map
    decoded = decode_function_body(body.code)
    ctx.control_flow_map = build_control_flow_map(decoded)

    # Load callee's code into the VM
    vm_instructions = to_vm_instructions(decoded)
    from virtual_machine.vm import CodeObject
    new_code = CodeObject(instructions=vm_instructions, constants=[], names=[])

    # We need to swap code --- store on context for the engine to pick up
    ctx._pending_code = new_code  # type: ignore[attr-defined]
    ctx.returned = False
    vm.jump_to(0)
    vm.halted = True


def _make_saved_frame(
    ctx: WasmExecutionContext,
    vm: GenericVM,
    code: Any,
    return_arity: int,
) -> Any:
    """Create a SavedFrame from the current execution context."""
    from wasm_execution.types import SavedFrame
    return SavedFrame(
        locals=list(ctx.typed_locals),
        label_stack=list(ctx.label_stack),
        stack_height=len(vm.typed_stack),
        control_flow_map=ctx.control_flow_map,
        code=code,
        return_pc=vm.pc + 1,
        return_arity=return_arity,
    )


# ===========================================================================
# Registration
# ===========================================================================


def register_control(vm: GenericVM) -> None:
    """Register all 13 control flow instruction handlers."""

    # 0x00: unreachable
    def handle_unreachable(_vm: GenericVM, _instr: Any, _code: Any, _ctx: WasmExecutionContext) -> str:
        raise TrapError("unreachable instruction executed")
    vm.register_context_opcode(0x00, handle_unreachable)

    # 0x01: nop
    def handle_nop(vm: GenericVM, _instr: Any, _code: Any, _ctx: WasmExecutionContext) -> str:
        vm.advance_pc()
        return "nop"
    vm.register_context_opcode(0x01, handle_nop)

    # 0x02: block
    def handle_block(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        block_type = instr.operand
        arity = _block_arity(block_type, ctx.func_types)
        target = ctx.control_flow_map.get(vm.pc)
        end_pc = target.end_pc if target else vm.pc + 1

        ctx.label_stack.append(Label(
            arity=arity,
            target_pc=end_pc,
            stack_height=len(vm.typed_stack),
            is_loop=False,
        ))
        vm.advance_pc()
        return "block"
    vm.register_context_opcode(0x02, handle_block)

    # 0x03: loop
    def handle_loop(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        block_type = instr.operand
        arity = _block_arity(block_type, ctx.func_types)

        ctx.label_stack.append(Label(
            arity=arity,
            target_pc=vm.pc,  # loops branch BACK to start
            stack_height=len(vm.typed_stack),
            is_loop=True,
        ))
        vm.advance_pc()
        return "loop"
    vm.register_context_opcode(0x03, handle_loop)

    # 0x04: if
    def handle_if(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        block_type = instr.operand
        arity = _block_arity(block_type, ctx.func_types)
        condition = vm.pop_typed().value

        target = ctx.control_flow_map.get(vm.pc)
        end_pc = target.end_pc if target else vm.pc + 1
        else_pc = target.else_pc if target else None

        ctx.label_stack.append(Label(
            arity=arity,
            target_pc=end_pc,
            stack_height=len(vm.typed_stack),
            is_loop=False,
        ))

        if condition != 0:
            vm.advance_pc()
        else:
            vm.jump_to(else_pc + 1 if else_pc is not None else end_pc)
        return "if"
    vm.register_context_opcode(0x04, handle_if)

    # 0x05: else
    def handle_else(vm: GenericVM, _instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        label = ctx.label_stack[-1]
        vm.jump_to(label.target_pc)
        return "else"
    vm.register_context_opcode(0x05, handle_else)

    # 0x0B: end
    def handle_end(vm: GenericVM, _instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        if len(ctx.label_stack) > 0:
            ctx.label_stack.pop()
            vm.advance_pc()
            return "end (block)"
        else:
            ctx.returned = True
            vm.halted = True
            return "end (function)"
    vm.register_context_opcode(0x0B, handle_end)

    # 0x0C: br
    def handle_br(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        label_index = instr.operand
        _execute_branch(vm, ctx, label_index)
        return f"br {label_index}"
    vm.register_context_opcode(0x0C, handle_br)

    # 0x0D: br_if
    def handle_br_if(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        label_index = instr.operand
        condition = vm.pop_typed().value
        if condition != 0:
            _execute_branch(vm, ctx, label_index)
        else:
            vm.advance_pc()
        return f"br_if {label_index}"
    vm.register_context_opcode(0x0D, handle_br_if)

    # 0x0E: br_table
    def handle_br_table(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        operand = instr.operand
        labels = operand["labels"]
        default_label = operand["default_label"]
        index = vm.pop_typed().value

        target_label = labels[index] if 0 <= index < len(labels) else default_label
        _execute_branch(vm, ctx, target_label)
        return f"br_table -> {target_label}"
    vm.register_context_opcode(0x0E, handle_br_table)

    # 0x0F: return
    def handle_return(vm: GenericVM, _instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        ctx.returned = True
        vm.halted = True
        return "return"
    vm.register_context_opcode(0x0F, handle_return)

    # 0x10: call
    def handle_call(vm: GenericVM, instr: Any, code: Any, ctx: WasmExecutionContext) -> str:
        func_index = instr.operand
        _call_function(vm, code, ctx, func_index)
        return f"call {func_index}"
    vm.register_context_opcode(0x10, handle_call)

    # 0x11: call_indirect
    def handle_call_indirect(vm: GenericVM, instr: Any, _code: Any, ctx: WasmExecutionContext) -> str:
        operand = instr.operand
        type_idx = operand["typeidx"]
        table_idx = operand.get("tableidx", 0)
        elem_index = vm.pop_typed().value

        table = ctx.tables[table_idx] if table_idx < len(ctx.tables) else None
        if table is None:
            raise TrapError("undefined table")

        func_index = table.get(elem_index)
        if func_index is None:
            raise TrapError("uninitialized table element")

        expected_type = ctx.func_types[type_idx]
        actual_type = ctx.func_types[func_index]
        if (expected_type.params != actual_type.params or
                expected_type.results != actual_type.results):
            raise TrapError("indirect call type mismatch")

        _call_function(vm, ctx, func_index)
        return f"call_indirect [{elem_index}] -> func {func_index}"
    vm.register_context_opcode(0x11, handle_call_indirect)
