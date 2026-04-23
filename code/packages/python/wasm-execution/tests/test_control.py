"""test_control.py --- Tests for WASM control flow instruction handlers.

Covers: block/end, loop, if/else, br, br_if, br_table, return, nop,
unreachable, and control flow helper functions.
"""

from __future__ import annotations

from typing import Any

import pytest
from virtual_machine.generic_vm import GenericVM
from virtual_machine.vm import CodeObject, Instruction
from wasm_types import FuncType, ValueType

from wasm_execution.host_interface import TrapError
from wasm_execution.instructions.control import register_control
from wasm_execution.instructions.dispatch import register_all_instructions
from wasm_execution.types import ControlTarget, Label, WasmExecutionContext
from wasm_execution.values import as_i32, i32


def _make_ctx(**overrides: Any) -> WasmExecutionContext:
    """Build a minimal WasmExecutionContext with overrides."""
    defaults = dict(
        memory=None,
        tables=[],
        globals=[],
        global_types=[],
        func_types=[],
        func_bodies=[],
        host_functions=[],
        typed_locals=[],
        label_stack=[],
        control_flow_map={},
        saved_frames=[],
    )
    defaults.update(overrides)
    return WasmExecutionContext(**defaults)


def _run_code(
    instructions: list[Instruction],
    ctx: WasmExecutionContext,
    stack: list[Any] | None = None,
) -> GenericVM:
    """Execute a sequence of instructions with the given context."""
    vm = GenericVM()
    register_all_instructions(vm)
    register_control(vm)

    if stack:
        for val in stack:
            vm.push_typed(val)

    code = CodeObject(instructions=instructions, constants=[], names=[])
    vm.execute_with_context(code, ctx)
    return vm


# ===========================================================================
# Unreachable and nop
# ===========================================================================


class TestUnreachableNop:
    def test_unreachable_traps(self) -> None:
        ctx = _make_ctx()
        with pytest.raises(TrapError, match="unreachable"):
            _run_code([Instruction(0x00, None)], ctx)

    def test_nop_does_nothing(self) -> None:
        ctx = _make_ctx()
        # nop followed by end (as function terminator)
        instrs = [
            Instruction(0x01, None),  # nop
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx)
        assert len(vm.typed_stack) == 0


# ===========================================================================
# Block and end
# ===========================================================================


class TestBlockEnd:
    def test_block_end_void(self) -> None:
        """A void block { nop } end should execute and leave empty stack."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=2, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, 0x40),  # block (void)
            Instruction(0x01, None),  # nop
            Instruction(0x0B, None),  # end (block)
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx)
        assert len(vm.typed_stack) == 0

    def test_block_with_i32_result(self) -> None:
        """A block that produces an i32 value."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=2, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, ValueType.I32),  # block (i32 result)
            Instruction(0x41, 42),              # i32.const 42
            Instruction(0x0B, None),            # end (block)
            Instruction(0x0B, None),            # end (function)
        ]
        vm = _run_code(instrs, ctx)
        assert as_i32(vm.pop_typed()) == 42


# ===========================================================================
# Loop
# ===========================================================================


class TestLoop:
    def test_simple_loop_with_br_if(self) -> None:
        """A loop that counts down from 3 to 0 using br_if."""
        # This is: loop { local.get 0; i32.const 1; i32.sub; local.tee 0; br_if 0 }
        # Local 0 = 3 initially
        ctx = _make_ctx(
            typed_locals=[i32(3)],
            control_flow_map={
                0: ControlTarget(end_pc=5, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x03, 0x40),  # loop (void)
            Instruction(0x20, 0),     # local.get 0
            Instruction(0x41, 1),     # i32.const 1
            Instruction(0x6B, None),  # i32.sub
            Instruction(0x22, 0),     # local.tee 0
            Instruction(0x0D, 0),     # br_if 0 (back to loop start)
            Instruction(0x0B, None),  # end (loop)
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx)
        assert ctx.typed_locals[0].value == 0


# ===========================================================================
# If/else
# ===========================================================================


class TestIfElse:
    def test_if_true(self) -> None:
        """if (true) { i32.const 10 } end => 10 on stack."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=2, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x04, ValueType.I32),  # if (i32 result)
            Instruction(0x41, 10),              # i32.const 10
            Instruction(0x0B, None),            # end
            Instruction(0x0B, None),            # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(1)])
        assert as_i32(vm.pop_typed()) == 10

    def test_if_false_skips_to_end(self) -> None:
        """if (false) { i32.const 10 } end => nothing on stack (void block)."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=2, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x04, 0x40),  # if (void)
            Instruction(0x41, 10),    # i32.const 10 (skipped)
            Instruction(0x0B, None),  # end
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(0)])
        # The i32.const 10 was skipped, stack should be empty
        assert len(vm.typed_stack) == 0

    def test_if_else_true_branch(self) -> None:
        """if (true) { 10 } else { 20 } end => 10."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=4, else_pc=2),
            },
        )
        instrs = [
            Instruction(0x04, ValueType.I32),  # if (i32)
            Instruction(0x41, 10),              # i32.const 10
            Instruction(0x05, None),            # else
            Instruction(0x41, 20),              # i32.const 20
            Instruction(0x0B, None),            # end
            Instruction(0x0B, None),            # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(1)])
        assert as_i32(vm.pop_typed()) == 10

    def test_if_else_false_branch(self) -> None:
        """if (false) { 10 } else { 20 } end => 20."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=4, else_pc=2),
            },
        )
        instrs = [
            Instruction(0x04, ValueType.I32),  # if (i32)
            Instruction(0x41, 10),              # i32.const 10
            Instruction(0x05, None),            # else
            Instruction(0x41, 20),              # i32.const 20
            Instruction(0x0B, None),            # end
            Instruction(0x0B, None),            # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(0)])
        assert as_i32(vm.pop_typed()) == 20


# ===========================================================================
# Branch (br)
# ===========================================================================


class TestBranch:
    def test_br_exits_block(self) -> None:
        """br 0 inside a block jumps to end."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=3, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, 0x40),  # block (void)
            Instruction(0x0C, 0),     # br 0
            Instruction(0x00, None),  # unreachable (should NOT be reached)
            Instruction(0x0B, None),  # end (block)
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx)
        # If unreachable was hit, we'd get TrapError

    def test_br_to_outer_block_resumes_after_end(self) -> None:
        """br 1 from a nested loop exits the block without halting the function."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=4, else_pc=None),
                1: ControlTarget(end_pc=3, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, 0x40),  # block
            Instruction(0x03, 0x40),  # loop
            Instruction(0x0C, 1),     # br 1
            Instruction(0x0B, None),  # end (loop)
            Instruction(0x0B, None),  # end (block)
            Instruction(0x41, 7),     # i32.const 7
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx)
        assert as_i32(vm.pop_typed()) == 7


# ===========================================================================
# br_if
# ===========================================================================


class TestBrIf:
    def test_br_if_true(self) -> None:
        """br_if with non-zero condition branches."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=3, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, 0x40),  # block (void)
            Instruction(0x0D, 0),     # br_if 0
            Instruction(0x00, None),  # unreachable (should NOT be reached)
            Instruction(0x0B, None),  # end (block)
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(1)])

    def test_br_if_false(self) -> None:
        """br_if with zero condition falls through."""
        ctx = _make_ctx(
            control_flow_map={
                0: ControlTarget(end_pc=3, else_pc=None),
            },
        )
        instrs = [
            Instruction(0x02, 0x40),  # block (void)
            Instruction(0x0D, 0),     # br_if 0
            Instruction(0x01, None),  # nop (should be reached)
            Instruction(0x0B, None),  # end (block)
            Instruction(0x0B, None),  # end (function)
        ]
        vm = _run_code(instrs, ctx, stack=[i32(0)])
        # nop was reached, no crash


# ===========================================================================
# Return
# ===========================================================================


class TestReturn:
    def test_return_halts(self) -> None:
        """return sets halted and returned flags."""
        ctx = _make_ctx()
        instrs = [
            Instruction(0x41, 42),    # i32.const 42
            Instruction(0x0F, None),  # return
            Instruction(0x00, None),  # unreachable (should not execute)
        ]
        vm = _run_code(instrs, ctx)
        assert ctx.returned is True
        assert as_i32(vm.pop_typed()) == 42
