"""Tests for the GenericVM — the pluggable bytecode interpreter.

These tests verify that the GenericVM correctly:
1. Dispatches to registered opcode handlers
2. Provides universal stack/variable/PC primitives
3. Enforces recursion limits
4. Supports freeze mode
5. Handles errors for unregistered opcodes
"""

from __future__ import annotations

import pytest

from virtual_machine import (
    CallFrame,
    CodeObject,
    GenericVM,
    Instruction,
    InvalidOpcodeError,
    MaxRecursionError,
    StackUnderflowError,
    VMError,
)


# =========================================================================
# Helpers: minimal opcode handlers for testing
# =========================================================================


def handler_load_const(vm, instr, code):
    """Test handler: push constant from pool."""
    vm.push(code.constants[instr.operand])
    vm.advance_pc()


def handler_add(vm, instr, code):
    """Test handler: pop two, push sum."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a + b)
    vm.advance_pc()


def handler_store_name(vm, instr, code):
    """Test handler: pop and store in named variable."""
    name = code.names[instr.operand]
    vm.variables[name] = vm.pop()
    vm.advance_pc()


def handler_load_name(vm, instr, code):
    """Test handler: push named variable."""
    name = code.names[instr.operand]
    vm.push(vm.variables[name])
    vm.advance_pc()


def handler_print(vm, instr, code):
    """Test handler: pop and print."""
    value = vm.pop()
    output = str(value)
    vm.output.append(output)
    vm.advance_pc()
    return output


def handler_halt(vm, instr, code):
    """Test handler: stop execution."""
    vm.halted = True


def handler_jump(vm, instr, code):
    """Test handler: unconditional jump."""
    vm.jump_to(instr.operand)


def handler_jump_if_false(vm, instr, code):
    """Test handler: conditional jump."""
    value = vm.pop()
    if not value:
        vm.jump_to(instr.operand)
    else:
        vm.advance_pc()


def make_vm():
    """Create a GenericVM with basic opcodes registered."""
    vm = GenericVM()
    vm.register_opcode(0x01, handler_load_const)
    vm.register_opcode(0x10, handler_store_name)
    vm.register_opcode(0x11, handler_load_name)
    vm.register_opcode(0x20, handler_add)
    vm.register_opcode(0x40, handler_jump)
    vm.register_opcode(0x41, handler_jump_if_false)
    vm.register_opcode(0x60, handler_print)
    vm.register_opcode(0xFF, handler_halt)
    return vm


# =========================================================================
# Test: Basic Execution
# =========================================================================


class TestBasicExecution:
    """Test basic VM execution with pluggable handlers."""

    def test_load_const_and_halt(self):
        """Load a constant and halt."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # LOAD_CONST 0
                Instruction(0xFF),     # HALT
            ],
            constants=[42],
        )
        vm.execute(code)
        assert vm.stack == [42]

    def test_add_two_constants(self):
        """Add two constants: 3 + 4 = 7."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # LOAD_CONST 0 (3)
                Instruction(0x01, 1),  # LOAD_CONST 1 (4)
                Instruction(0x20),     # ADD
                Instruction(0xFF),     # HALT
            ],
            constants=[3, 4],
        )
        vm.execute(code)
        assert vm.stack == [7]

    def test_store_and_load_name(self):
        """Store a value in a variable, then load it back."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # LOAD_CONST 0 (42)
                Instruction(0x10, 0),  # STORE_NAME 0 ("x")
                Instruction(0x11, 0),  # LOAD_NAME 0 ("x")
                Instruction(0xFF),     # HALT
            ],
            constants=[42],
            names=["x"],
        )
        vm.execute(code)
        assert vm.variables["x"] == 42
        assert vm.stack == [42]

    def test_print_output(self):
        """Print captures output."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # LOAD_CONST 0 ("hello")
                Instruction(0x60),     # PRINT
                Instruction(0xFF),     # HALT
            ],
            constants=["hello"],
        )
        vm.execute(code)
        assert vm.output == ["hello"]

    def test_compile_and_execute_x_eq_1_plus_2(self):
        """Full test: x = 1 + 2 should result in x = 3."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # LOAD_CONST 0 (1)
                Instruction(0x01, 1),  # LOAD_CONST 1 (2)
                Instruction(0x20),     # ADD
                Instruction(0x10, 0),  # STORE_NAME 0 ("x")
                Instruction(0xFF),     # HALT
            ],
            constants=[1, 2],
            names=["x"],
        )
        vm.execute(code)
        assert vm.variables["x"] == 3


# =========================================================================
# Test: Control Flow
# =========================================================================


class TestControlFlow:
    """Test jump instructions."""

    def test_unconditional_jump(self):
        """JUMP skips over instructions."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # 0: LOAD_CONST 0 (1)
                Instruction(0x40, 3),  # 1: JUMP 3 (skip LOAD_CONST 99)
                Instruction(0x01, 1),  # 2: LOAD_CONST 1 (99) — skipped
                Instruction(0xFF),     # 3: HALT
            ],
            constants=[1, 99],
        )
        vm.execute(code)
        assert vm.stack == [1]  # 99 was skipped

    def test_conditional_jump_taken(self):
        """JUMP_IF_FALSE jumps when value is falsy."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # 0: LOAD_CONST 0 (0 = falsy)
                Instruction(0x41, 4),  # 1: JUMP_IF_FALSE 4
                Instruction(0x01, 1),  # 2: LOAD_CONST 1 (42) — skipped
                Instruction(0x40, 4),  # 3: JUMP 4
                Instruction(0xFF),     # 4: HALT
            ],
            constants=[0, 42],
        )
        vm.execute(code)
        assert vm.stack == []  # 42 was skipped

    def test_conditional_jump_not_taken(self):
        """JUMP_IF_FALSE falls through when value is truthy."""
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),  # 0: LOAD_CONST 0 (1 = truthy)
                Instruction(0x41, 4),  # 1: JUMP_IF_FALSE 4
                Instruction(0x01, 1),  # 2: LOAD_CONST 1 (42)
                Instruction(0x40, 4),  # 3: JUMP 4
                Instruction(0xFF),     # 4: HALT
            ],
            constants=[1, 42],
        )
        vm.execute(code)
        assert vm.stack == [42]


# =========================================================================
# Test: Stack Operations
# =========================================================================


class TestStackOperations:
    """Test push, pop, peek helpers."""

    def test_push_and_pop(self):
        vm = GenericVM()
        vm.push(42)
        vm.push("hello")
        assert vm.pop() == "hello"
        assert vm.pop() == 42

    def test_peek(self):
        vm = GenericVM()
        vm.push(42)
        assert vm.peek() == 42
        assert len(vm.stack) == 1  # Not removed

    def test_pop_empty_raises(self):
        vm = GenericVM()
        with pytest.raises(StackUnderflowError):
            vm.pop()

    def test_peek_empty_raises(self):
        vm = GenericVM()
        with pytest.raises(StackUnderflowError):
            vm.peek()


# =========================================================================
# Test: Call Stack
# =========================================================================


class TestCallStack:
    """Test call frame push/pop."""

    def test_push_and_pop_frame(self):
        vm = GenericVM()
        frame = CallFrame(return_address=5, saved_variables={"x": 1}, saved_locals=[])
        vm.push_frame(frame)
        assert len(vm.call_stack) == 1
        popped = vm.pop_frame()
        assert popped.return_address == 5
        assert len(vm.call_stack) == 0

    def test_pop_empty_call_stack_raises(self):
        vm = GenericVM()
        with pytest.raises(VMError):
            vm.pop_frame()

    def test_max_recursion_depth(self):
        vm = GenericVM()
        vm.set_max_recursion_depth(2)
        frame = CallFrame(return_address=0, saved_variables={}, saved_locals=[])
        vm.push_frame(frame)
        vm.push_frame(frame)
        with pytest.raises(MaxRecursionError):
            vm.push_frame(frame)

    def test_max_recursion_depth_zero(self):
        """Zero depth means no calls allowed."""
        vm = GenericVM()
        vm.set_max_recursion_depth(0)
        frame = CallFrame(return_address=0, saved_variables={}, saved_locals=[])
        with pytest.raises(MaxRecursionError):
            vm.push_frame(frame)


# =========================================================================
# Test: Unregistered Opcode
# =========================================================================


class TestErrors:
    """Test error handling."""

    def test_unregistered_opcode_raises(self):
        vm = GenericVM()
        vm.register_opcode(0xFF, handler_halt)
        code = CodeObject(
            instructions=[
                Instruction(0x99),  # Unknown opcode
            ],
        )
        with pytest.raises(InvalidOpcodeError, match="0x99"):
            vm.execute(code)


# =========================================================================
# Test: Reset
# =========================================================================


class TestReset:
    """Test VM reset preserves handlers."""

    def test_reset_clears_state(self):
        vm = make_vm()
        vm.push(42)
        vm.variables["x"] = 1
        vm.output.append("test")
        vm.reset()
        assert vm.stack == []
        assert vm.variables == {}
        assert vm.output == []

    def test_reset_preserves_handlers(self):
        """After reset, handlers are still registered."""
        vm = make_vm()
        vm.reset()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),
                Instruction(0xFF),
            ],
            constants=[42],
        )
        vm.execute(code)
        assert vm.stack == [42]


# =========================================================================
# Test: Tracing
# =========================================================================


class TestTracing:
    """Test that execution produces traces."""

    def test_traces_recorded(self):
        vm = make_vm()
        code = CodeObject(
            instructions=[
                Instruction(0x01, 0),
                Instruction(0xFF),
            ],
            constants=[42],
        )
        traces = vm.execute(code)
        assert len(traces) == 2  # LOAD_CONST + HALT
        assert traces[0].pc == 0
        assert traces[0].stack_before == []
        assert traces[0].stack_after == [42]


# =========================================================================
# Test: Builtins
# =========================================================================


class TestBuiltins:
    """Test built-in function registration."""

    def test_register_and_get_builtin(self):
        vm = GenericVM()
        vm.register_builtin("len", lambda args: len(args[0]))
        builtin = vm.get_builtin("len")
        assert builtin is not None
        assert builtin.name == "len"
        assert builtin.implementation(["hello"]) == 5

    def test_get_nonexistent_builtin(self):
        vm = GenericVM()
        assert vm.get_builtin("nonexistent") is None


# =========================================================================
# Test: Freeze Mode
# =========================================================================


class TestFreezeMode:
    """Test frozen mode."""

    def test_freeze_unfroze(self):
        vm = GenericVM()
        assert not vm.is_frozen
        vm.set_frozen(True)
        assert vm.is_frozen
        vm.set_frozen(False)
        assert not vm.is_frozen


class TestGlobalsInjection:
    """Test pre-seeding globals into the VM."""

    def test_inject_globals_merges_and_overwrites(self):
        vm = GenericVM()
        vm.variables["existing"] = 1
        vm.variables["ctx_os"] = "linux"

        vm.inject_globals({"ctx_os": "darwin", "answer": 42})

        assert vm.variables == {
            "existing": 1,
            "ctx_os": "darwin",
            "answer": 42,
        }
