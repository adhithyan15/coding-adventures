"""Comprehensive tests for the Virtual Machine — Layer 5 of the computing stack.

These tests verify every opcode, every error path, and several end-to-end
programs. They're organized by category to make it easy to find tests for
a specific feature.

We follow the Arrange-Act-Assert pattern throughout:
    1. Arrange: Build a CodeObject with the instructions we want to test.
    2. Act: Execute it on a fresh VirtualMachine.
    3. Assert: Check the stack, variables, output, and traces.
"""

import pytest

from virtual_machine import (
    CallFrame,
    CodeObject,
    DivisionByZeroError,
    Instruction,
    InvalidOpcodeError,
    InvalidOperandError,
    OpCode,
    StackUnderflowError,
    UndefinedNameError,
    VirtualMachine,
    VMError,
    VMTrace,
    assemble_code,
)


# =========================================================================
# Helpers
# =========================================================================


def run(
    instructions: list[Instruction],
    constants: list[int | float | str] | None = None,
    names: list[str] | None = None,
) -> tuple[VirtualMachine, list[VMTrace]]:
    """Build a CodeObject, execute it, and return the VM + traces.

    This is a convenience wrapper so every test doesn't have to repeat
    the boilerplate of creating a VM and calling execute().
    """
    code = assemble_code(instructions, constants, names)
    vm = VirtualMachine()
    traces = vm.execute(code)
    return vm, traces


# =========================================================================
# Stack Operations
# =========================================================================


class TestLoadConst:
    """Tests for LOAD_CONST — pushing constants onto the stack."""

    def test_push_integer(self) -> None:
        """LOAD_CONST should push an integer from the constants pool."""
        vm, traces = run(
            [Instruction(OpCode.LOAD_CONST, 0), Instruction(OpCode.HALT)],
            constants=[42],
        )
        assert vm.stack == [42]
        assert len(traces) == 2  # LOAD_CONST + HALT

    def test_push_string(self) -> None:
        """LOAD_CONST should push a string from the constants pool."""
        vm, _ = run(
            [Instruction(OpCode.LOAD_CONST, 0), Instruction(OpCode.HALT)],
            constants=["hello"],
        )
        assert vm.stack == ["hello"]

    def test_push_multiple(self) -> None:
        """Multiple LOAD_CONST instructions stack values in order."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.LOAD_CONST, 2),
                Instruction(OpCode.HALT),
            ],
            constants=[10, 20, 30],
        )
        assert vm.stack == [10, 20, 30]

    def test_invalid_index(self) -> None:
        """LOAD_CONST with an out-of-range index should raise an error."""
        with pytest.raises(InvalidOperandError, match="out of range"):
            run(
                [Instruction(OpCode.LOAD_CONST, 5), Instruction(OpCode.HALT)],
                constants=[42],
            )

    def test_missing_operand(self) -> None:
        """LOAD_CONST without an operand should raise an error."""
        with pytest.raises(InvalidOperandError, match="requires an operand"):
            run([Instruction(OpCode.LOAD_CONST), Instruction(OpCode.HALT)])


class TestPop:
    """Tests for POP — discarding the top of the stack."""

    def test_pop_removes_top(self) -> None:
        """POP should remove exactly one value from the top."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.POP),
                Instruction(OpCode.HALT),
            ],
            constants=[10, 20],
        )
        assert vm.stack == [10]

    def test_pop_empty_stack(self) -> None:
        """POP on an empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.POP), Instruction(OpCode.HALT)])


class TestDup:
    """Tests for DUP — duplicating the top of the stack."""

    def test_dup_copies_top(self) -> None:
        """DUP should push a copy of the top value."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.DUP),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
        )
        assert vm.stack == [42, 42]

    def test_dup_empty_stack(self) -> None:
        """DUP on an empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError, match="DUP requires"):
            run([Instruction(OpCode.DUP), Instruction(OpCode.HALT)])


# =========================================================================
# Arithmetic Operations
# =========================================================================


class TestAdd:
    """Tests for ADD — popping two values and pushing their sum."""

    def test_add_integers(self) -> None:
        """ADD should pop two integers and push their sum."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.ADD),
                Instruction(OpCode.HALT),
            ],
            constants=[3, 4],
        )
        assert vm.stack == [7]

    def test_add_strings(self) -> None:
        """ADD should concatenate strings (like Python's + on strings)."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.ADD),
                Instruction(OpCode.HALT),
            ],
            constants=["hello ", "world"],
        )
        assert vm.stack == ["hello world"]

    def test_add_underflow(self) -> None:
        """ADD with fewer than two values should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run(
                [
                    Instruction(OpCode.LOAD_CONST, 0),
                    Instruction(OpCode.ADD),
                    Instruction(OpCode.HALT),
                ],
                constants=[42],
            )


class TestSub:
    """Tests for SUB — popping two values and pushing their difference."""

    def test_sub_integers(self) -> None:
        """SUB should compute a - b where a is pushed first."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),  # push 10
                Instruction(OpCode.LOAD_CONST, 1),  # push 3
                Instruction(OpCode.SUB),             # 10 - 3 = 7
                Instruction(OpCode.HALT),
            ],
            constants=[10, 3],
        )
        assert vm.stack == [7]

    def test_sub_negative_result(self) -> None:
        """SUB should handle negative results correctly."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.SUB),
                Instruction(OpCode.HALT),
            ],
            constants=[3, 10],
        )
        assert vm.stack == [-7]


class TestMul:
    """Tests for MUL — popping two values and pushing their product."""

    def test_mul_integers(self) -> None:
        """MUL should pop two integers and push their product."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.MUL),
                Instruction(OpCode.HALT),
            ],
            constants=[6, 7],
        )
        assert vm.stack == [42]

    def test_mul_by_zero(self) -> None:
        """MUL by zero should produce zero."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.MUL),
                Instruction(OpCode.HALT),
            ],
            constants=[999, 0],
        )
        assert vm.stack == [0]


class TestDiv:
    """Tests for DIV — integer division."""

    def test_div_integers(self) -> None:
        """DIV should perform integer division (a // b)."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.DIV),
                Instruction(OpCode.HALT),
            ],
            constants=[10, 3],
        )
        assert vm.stack == [3]  # 10 // 3 = 3

    def test_div_by_zero(self) -> None:
        """DIV by zero should raise DivisionByZeroError."""
        with pytest.raises(DivisionByZeroError, match="Division by zero"):
            run(
                [
                    Instruction(OpCode.LOAD_CONST, 0),
                    Instruction(OpCode.LOAD_CONST, 1),
                    Instruction(OpCode.DIV),
                    Instruction(OpCode.HALT),
                ],
                constants=[10, 0],
            )

    def test_div_exact(self) -> None:
        """DIV with an exact result should produce the correct quotient."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.DIV),
                Instruction(OpCode.HALT),
            ],
            constants=[42, 6],
        )
        assert vm.stack == [7]


# =========================================================================
# Variable Operations
# =========================================================================


class TestNamedVariables:
    """Tests for STORE_NAME and LOAD_NAME — named variable operations."""

    def test_store_and_load(self) -> None:
        """STORE_NAME followed by LOAD_NAME should round-trip a value."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.STORE_NAME, 0),
                Instruction(OpCode.LOAD_NAME, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        assert vm.stack == [42]
        assert vm.variables == {"x": 42}

    def test_load_undefined(self) -> None:
        """LOAD_NAME for an undefined variable should raise UndefinedNameError."""
        with pytest.raises(UndefinedNameError, match="'x' is not defined"):
            run(
                [Instruction(OpCode.LOAD_NAME, 0), Instruction(OpCode.HALT)],
                names=["x"],
            )

    def test_store_overwrites(self) -> None:
        """STORE_NAME should overwrite the previous value of a variable."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),  # push 10
                Instruction(OpCode.STORE_NAME, 0),   # x = 10
                Instruction(OpCode.LOAD_CONST, 1),   # push 20
                Instruction(OpCode.STORE_NAME, 0),   # x = 20
                Instruction(OpCode.LOAD_NAME, 0),    # push x (should be 20)
                Instruction(OpCode.HALT),
            ],
            constants=[10, 20],
            names=["x"],
        )
        assert vm.stack == [20]
        assert vm.variables["x"] == 20

    def test_store_name_invalid_index(self) -> None:
        """STORE_NAME with out-of-range index should raise InvalidOperandError."""
        with pytest.raises(InvalidOperandError, match="out of range"):
            run(
                [
                    Instruction(OpCode.LOAD_CONST, 0),
                    Instruction(OpCode.STORE_NAME, 5),
                    Instruction(OpCode.HALT),
                ],
                constants=[42],
                names=["x"],
            )

    def test_load_name_invalid_index(self) -> None:
        """LOAD_NAME with out-of-range index should raise InvalidOperandError."""
        with pytest.raises(InvalidOperandError, match="out of range"):
            run(
                [Instruction(OpCode.LOAD_NAME, 5), Instruction(OpCode.HALT)],
                names=["x"],
            )


class TestLocalVariables:
    """Tests for STORE_LOCAL and LOAD_LOCAL — indexed local variable slots."""

    def test_store_and_load_local(self) -> None:
        """STORE_LOCAL + LOAD_LOCAL should round-trip a value by slot index."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.STORE_LOCAL, 0),
                Instruction(OpCode.LOAD_LOCAL, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[99],
        )
        assert vm.stack == [99]

    def test_multiple_local_slots(self) -> None:
        """Multiple local slots should be independent."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),   # push 10
                Instruction(OpCode.STORE_LOCAL, 0),   # slot 0 = 10
                Instruction(OpCode.LOAD_CONST, 1),   # push 20
                Instruction(OpCode.STORE_LOCAL, 1),   # slot 1 = 20
                Instruction(OpCode.LOAD_LOCAL, 0),    # push slot 0 (10)
                Instruction(OpCode.LOAD_LOCAL, 1),    # push slot 1 (20)
                Instruction(OpCode.HALT),
            ],
            constants=[10, 20],
        )
        assert vm.stack == [10, 20]

    def test_load_local_uninitialized(self) -> None:
        """LOAD_LOCAL from an uninitialized slot should raise an error."""
        with pytest.raises(InvalidOperandError, match="not been initialized"):
            run(
                [Instruction(OpCode.LOAD_LOCAL, 0), Instruction(OpCode.HALT)],
            )

    def test_store_local_auto_grows(self) -> None:
        """STORE_LOCAL to a high index should auto-grow the locals array."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.STORE_LOCAL, 5),
                Instruction(OpCode.LOAD_LOCAL, 5),
                Instruction(OpCode.HALT),
            ],
            constants=[77],
        )
        assert vm.stack == [77]
        assert len(vm.locals) == 6  # slots 0-5

    def test_store_local_missing_operand(self) -> None:
        """STORE_LOCAL without an operand should raise InvalidOperandError."""
        with pytest.raises(InvalidOperandError, match="requires an operand"):
            run(
                [
                    Instruction(OpCode.LOAD_CONST, 0),
                    Instruction(OpCode.STORE_LOCAL),
                    Instruction(OpCode.HALT),
                ],
                constants=[42],
            )

    def test_load_local_missing_operand(self) -> None:
        """LOAD_LOCAL without an operand should raise InvalidOperandError."""
        with pytest.raises(InvalidOperandError, match="requires an operand"):
            run(
                [Instruction(OpCode.LOAD_LOCAL), Instruction(OpCode.HALT)],
            )


# =========================================================================
# Comparison Operations
# =========================================================================


class TestComparison:
    """Tests for CMP_EQ, CMP_LT, CMP_GT — comparison operations."""

    def test_cmp_eq_true(self) -> None:
        """CMP_EQ should push 1 when values are equal."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_EQ),
                Instruction(OpCode.HALT),
            ],
            constants=[42, 42],
        )
        assert vm.stack == [1]

    def test_cmp_eq_false(self) -> None:
        """CMP_EQ should push 0 when values are not equal."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_EQ),
                Instruction(OpCode.HALT),
            ],
            constants=[42, 99],
        )
        assert vm.stack == [0]

    def test_cmp_lt_true(self) -> None:
        """CMP_LT should push 1 when a < b."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),  # push 3 (a)
                Instruction(OpCode.LOAD_CONST, 1),   # push 7 (b)
                Instruction(OpCode.CMP_LT),          # 3 < 7 → 1
                Instruction(OpCode.HALT),
            ],
            constants=[3, 7],
        )
        assert vm.stack == [1]

    def test_cmp_lt_false(self) -> None:
        """CMP_LT should push 0 when a >= b."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_LT),
                Instruction(OpCode.HALT),
            ],
            constants=[7, 3],
        )
        assert vm.stack == [0]

    def test_cmp_gt_true(self) -> None:
        """CMP_GT should push 1 when a > b."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_GT),
                Instruction(OpCode.HALT),
            ],
            constants=[7, 3],
        )
        assert vm.stack == [1]

    def test_cmp_gt_false(self) -> None:
        """CMP_GT should push 0 when a <= b."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_GT),
                Instruction(OpCode.HALT),
            ],
            constants=[3, 7],
        )
        assert vm.stack == [0]

    def test_cmp_eq_strings(self) -> None:
        """CMP_EQ should work with strings too."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.CMP_EQ),
                Instruction(OpCode.HALT),
            ],
            constants=["hello", "hello"],
        )
        assert vm.stack == [1]


# =========================================================================
# Control Flow
# =========================================================================


class TestJump:
    """Tests for JUMP — unconditional jump."""

    def test_jump_forward(self) -> None:
        """JUMP should skip over instructions to the target."""
        # Program: push 1, jump past the "push 2", push 3, halt
        # Expected stack: [1, 3]  (2 is skipped)
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),  # 0: push 1
                Instruction(OpCode.JUMP, 3),          # 1: jump to 3
                Instruction(OpCode.LOAD_CONST, 1),   # 2: push 2 (SKIPPED)
                Instruction(OpCode.LOAD_CONST, 2),   # 3: push 3
                Instruction(OpCode.HALT),              # 4: halt
            ],
            constants=[1, 2, 3],
        )
        assert vm.stack == [1, 3]

    def test_jump_missing_operand(self) -> None:
        """JUMP without an operand should raise InvalidOperandError."""
        with pytest.raises(InvalidOperandError, match="requires an operand"):
            run([Instruction(OpCode.JUMP), Instruction(OpCode.HALT)])


class TestJumpIfFalse:
    """Tests for JUMP_IF_FALSE — conditional branch on falsy values."""

    def test_jump_when_false(self) -> None:
        """JUMP_IF_FALSE should jump when the top of stack is 0 (falsy)."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),       # 0: push 0 (false)
                Instruction(OpCode.JUMP_IF_FALSE, 3),     # 1: jump to 3
                Instruction(OpCode.LOAD_CONST, 1),        # 2: push 999 (SKIPPED)
                Instruction(OpCode.HALT),                  # 3: halt
            ],
            constants=[0, 999],
        )
        assert vm.stack == []  # 0 was consumed by the jump, 999 was skipped

    def test_no_jump_when_true(self) -> None:
        """JUMP_IF_FALSE should NOT jump when the top of stack is truthy."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),       # 0: push 1 (true)
                Instruction(OpCode.JUMP_IF_FALSE, 3),     # 1: doesn't jump
                Instruction(OpCode.LOAD_CONST, 1),        # 2: push 42
                Instruction(OpCode.HALT),                  # 3: halt
            ],
            constants=[1, 42],
        )
        assert vm.stack == [42]

    def test_jump_on_none(self) -> None:
        """JUMP_IF_FALSE should consider None as falsy."""
        # Store None by storing and re-checking. We can use a trick:
        # Load 0, then use it. None is harder to get on the stack directly,
        # so we test with 0 and empty string instead.
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),       # push ""
                Instruction(OpCode.JUMP_IF_FALSE, 3),     # jump (empty string is falsy)
                Instruction(OpCode.LOAD_CONST, 1),        # SKIPPED
                Instruction(OpCode.HALT),
            ],
            constants=["", 999],
        )
        assert vm.stack == []


class TestJumpIfTrue:
    """Tests for JUMP_IF_TRUE — conditional branch on truthy values."""

    def test_jump_when_true(self) -> None:
        """JUMP_IF_TRUE should jump when the top of stack is truthy."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),       # 0: push 1 (true)
                Instruction(OpCode.JUMP_IF_TRUE, 3),      # 1: jump to 3
                Instruction(OpCode.LOAD_CONST, 1),        # 2: push 999 (SKIPPED)
                Instruction(OpCode.HALT),                  # 3: halt
            ],
            constants=[1, 999],
        )
        assert vm.stack == []

    def test_no_jump_when_false(self) -> None:
        """JUMP_IF_TRUE should NOT jump when the top of stack is falsy."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),       # 0: push 0 (false)
                Instruction(OpCode.JUMP_IF_TRUE, 3),      # 1: doesn't jump
                Instruction(OpCode.LOAD_CONST, 1),        # 2: push 42
                Instruction(OpCode.HALT),                  # 3: halt
            ],
            constants=[0, 42],
        )
        assert vm.stack == [42]


class TestLoop:
    """Test loops built from JUMP instructions."""

    def test_simple_countdown(self) -> None:
        """A countdown loop using JUMP_IF_FALSE.

        Equivalent to:
            x = 3
            while x > 0:
                print(x)
                x = x - 1
        """
        vm, _ = run(
            [
                # x = 3
                Instruction(OpCode.LOAD_CONST, 0),    # 0: push 3
                Instruction(OpCode.STORE_NAME, 0),     # 1: x = 3

                # while x > 0:
                Instruction(OpCode.LOAD_NAME, 0),      # 2: push x
                Instruction(OpCode.LOAD_CONST, 1),     # 3: push 0
                Instruction(OpCode.CMP_GT),             # 4: x > 0?
                Instruction(OpCode.JUMP_IF_FALSE, 13),  # 5: if not, exit loop

                # print(x)
                Instruction(OpCode.LOAD_NAME, 0),      # 6: push x
                Instruction(OpCode.PRINT),              # 7: print x

                # x = x - 1
                Instruction(OpCode.LOAD_NAME, 0),      # 8: push x
                Instruction(OpCode.LOAD_CONST, 2),     # 9: push 1
                Instruction(OpCode.SUB),                # 10: x - 1
                Instruction(OpCode.STORE_NAME, 0),     # 11: x = x - 1

                Instruction(OpCode.JUMP, 2),            # 12: back to loop start

                Instruction(OpCode.HALT),               # 13: done
            ],
            constants=[3, 0, 1],
            names=["x"],
        )
        assert vm.output == ["3", "2", "1"]
        assert vm.variables["x"] == 0


# =========================================================================
# I/O Operations
# =========================================================================


class TestPrint:
    """Tests for PRINT — capturing output."""

    def test_print_integer(self) -> None:
        """PRINT should convert an integer to string and capture it."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
        )
        assert vm.output == ["42"]
        assert vm.stack == []  # PRINT consumes the value

    def test_print_string(self) -> None:
        """PRINT should capture a string as-is."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.HALT),
            ],
            constants=["hello world"],
        )
        assert vm.output == ["hello world"]

    def test_print_multiple(self) -> None:
        """Multiple PRINT instructions should append to the output list."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.HALT),
            ],
            constants=["hello", "world"],
        )
        assert vm.output == ["hello", "world"]

    def test_print_empty_stack(self) -> None:
        """PRINT on an empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.PRINT), Instruction(OpCode.HALT)])


# =========================================================================
# HALT and VM Control
# =========================================================================


class TestHalt:
    """Tests for HALT — stopping execution."""

    def test_halt_stops_execution(self) -> None:
        """HALT should stop the VM immediately."""
        vm, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.HALT),
                Instruction(OpCode.LOAD_CONST, 1),  # Should NOT execute
            ],
            constants=[1, 2],
        )
        assert vm.stack == [1]  # Only first LOAD_CONST ran
        assert vm.halted is True
        assert len(traces) == 2  # LOAD_CONST + HALT

    def test_program_without_halt(self) -> None:
        """A program without HALT should still terminate (PC runs off the end)."""
        vm, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
            ],
            constants=[42],
        )
        assert vm.stack == [42]
        assert len(traces) == 1


# =========================================================================
# Function Operations
# =========================================================================


class TestFunctions:
    """Tests for CALL and RETURN — function calling."""

    def test_call_simple_function(self) -> None:
        """CALL should execute a function stored as a CodeObject variable."""
        # Define a function that pushes 99 and prints it.
        func_code = assemble_code(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.RETURN),
            ],
            constants=[99],
        )

        # Main program: store the function, call it.
        vm = VirtualMachine()
        vm.variables["my_func"] = func_code

        main_code = assemble_code(
            [
                Instruction(OpCode.CALL, 0),
                Instruction(OpCode.HALT),
            ],
            names=["my_func"],
        )
        vm.execute(main_code)
        assert vm.output == ["99"]

    def test_call_undefined_function(self) -> None:
        """CALL on an undefined name should raise UndefinedNameError."""
        with pytest.raises(UndefinedNameError, match="'no_such_func' is not defined"):
            run(
                [Instruction(OpCode.CALL, 0), Instruction(OpCode.HALT)],
                names=["no_such_func"],
            )

    def test_call_non_callable(self) -> None:
        """CALL on a non-CodeObject should raise VMError."""
        vm = VirtualMachine()
        vm.variables["not_func"] = 42

        code = assemble_code(
            [Instruction(OpCode.CALL, 0), Instruction(OpCode.HALT)],
            names=["not_func"],
        )
        with pytest.raises(VMError, match="not callable"):
            vm.execute(code)

    def test_return_at_top_level(self) -> None:
        """RETURN at the top level (no call frame) should act like HALT."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.RETURN),
                Instruction(OpCode.LOAD_CONST, 1),  # Should NOT execute
            ],
            constants=[42, 99],
        )
        assert vm.stack == [42]
        assert vm.halted is True


# =========================================================================
# Trace Output
# =========================================================================


class TestTrace:
    """Tests for VMTrace — the execution trace system."""

    def test_trace_captures_stack_states(self) -> None:
        """Each trace should capture stack before and after the instruction."""
        _, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.ADD),
                Instruction(OpCode.HALT),
            ],
            constants=[3, 4],
        )
        # LOAD_CONST 3
        assert traces[0].stack_before == []
        assert traces[0].stack_after == [3]
        assert traces[0].pc == 0

        # LOAD_CONST 4
        assert traces[1].stack_before == [3]
        assert traces[1].stack_after == [3, 4]
        assert traces[1].pc == 1

        # ADD
        assert traces[2].stack_before == [3, 4]
        assert traces[2].stack_after == [7]
        assert traces[2].pc == 2

    def test_trace_captures_variables(self) -> None:
        """Traces should snapshot the variables dict after each step."""
        _, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.STORE_NAME, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        # After STORE_NAME, variables should contain x=42
        assert traces[1].variables == {"x": 42}

    def test_trace_captures_print_output(self) -> None:
        """PRINT traces should have the output field set."""
        _, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
        )
        # The PRINT trace (index 1) should have output
        assert traces[1].output == "42"

        # Non-PRINT traces should have None
        assert traces[0].output is None

    def test_trace_has_descriptions(self) -> None:
        """Every trace should have a non-empty description."""
        _, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[42],
        )
        for trace in traces:
            assert trace.description != ""

    def test_trace_description_content(self) -> None:
        """Verify specific description strings for key operations."""
        _, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.ADD),
                Instruction(OpCode.STORE_NAME, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[3, 4],
            names=["x"],
        )
        assert "42" not in traces[0].description  # Shouldn't mention 42
        assert "3" in traces[0].description        # Should mention the constant
        assert "sum" in traces[2].description.lower() or "+" in traces[2].description
        assert "x" in traces[3].description


# =========================================================================
# Error Cases
# =========================================================================


class TestErrors:
    """Tests for runtime error handling."""

    def test_stack_underflow_on_arithmetic(self) -> None:
        """Arithmetic on an empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.SUB), Instruction(OpCode.HALT)])

    def test_stack_underflow_on_comparison(self) -> None:
        """Comparison on an empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.CMP_EQ), Instruction(OpCode.HALT)])

    def test_unknown_opcode(self) -> None:
        """An unrecognized opcode should raise InvalidOpcodeError."""
        # Create an instruction with a fake opcode value.
        fake_instruction = Instruction.__new__(Instruction)
        fake_instruction.opcode = 0xAA  # Not a valid OpCode
        fake_instruction.operand = None

        code = CodeObject(
            instructions=[fake_instruction],
            constants=[],
            names=[],
        )
        vm = VirtualMachine()
        with pytest.raises(InvalidOpcodeError, match="Unknown opcode"):
            vm.execute(code)


# =========================================================================
# End-to-End Programs
# =========================================================================


class TestEndToEnd:
    """End-to-end tests that represent real programs compiled to bytecode."""

    def test_x_equals_1_plus_2(self) -> None:
        """Compile and run: x = 1 + 2

        This is the canonical introductory example. A compiler would produce:
            LOAD_CONST 0  (1)
            LOAD_CONST 1  (2)
            ADD
            STORE_NAME 0  (x)
            HALT
        """
        vm, traces = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.ADD),
                Instruction(OpCode.STORE_NAME, 0),
                Instruction(OpCode.HALT),
            ],
            constants=[1, 2],
            names=["x"],
        )
        assert vm.variables["x"] == 3
        assert vm.stack == []  # Stack should be clean after store
        assert len(traces) == 5

    def test_if_else_branch(self) -> None:
        """Compile and run an if/else:

            x = 10
            if x > 5:
                print("big")
            else:
                print("small")

        Expected: prints "big"
        """
        vm, _ = run(
            [
                # x = 10
                Instruction(OpCode.LOAD_CONST, 0),    # 0: push 10
                Instruction(OpCode.STORE_NAME, 0),     # 1: x = 10

                # if x > 5:
                Instruction(OpCode.LOAD_NAME, 0),      # 2: push x
                Instruction(OpCode.LOAD_CONST, 1),     # 3: push 5
                Instruction(OpCode.CMP_GT),             # 4: x > 5?
                Instruction(OpCode.JUMP_IF_FALSE, 9),   # 5: if false, goto else

                # then: print("big")
                Instruction(OpCode.LOAD_CONST, 2),     # 6: push "big"
                Instruction(OpCode.PRINT),              # 7: print
                Instruction(OpCode.JUMP, 11),           # 8: skip else

                # else: print("small")
                Instruction(OpCode.LOAD_CONST, 3),     # 9: push "small"
                Instruction(OpCode.PRINT),              # 10: print

                Instruction(OpCode.HALT),               # 11: done
            ],
            constants=[10, 5, "big", "small"],
            names=["x"],
        )
        assert vm.output == ["big"]

    def test_sum_1_to_5(self) -> None:
        """Compile and run: sum of 1 to 5 (corrected jump targets).

            total = 0
            i = 1
            while i <= 5:
                total = total + i
                i = i + 1
            print(total)

        Expected: prints "15"
        """
        vm, _ = run(
            [
                # total = 0
                Instruction(OpCode.LOAD_CONST, 0),    # 0: push 0
                Instruction(OpCode.STORE_NAME, 0),     # 1: total = 0

                # i = 1
                Instruction(OpCode.LOAD_CONST, 1),    # 2: push 1
                Instruction(OpCode.STORE_NAME, 1),     # 3: i = 1

                # while i <= 5:  →  if i > 5: break
                Instruction(OpCode.LOAD_NAME, 1),      # 4: push i
                Instruction(OpCode.LOAD_CONST, 2),     # 5: push 5
                Instruction(OpCode.CMP_GT),             # 6: i > 5?
                Instruction(OpCode.JUMP_IF_TRUE, 17),   # 7: if i > 5, exit

                # total = total + i
                Instruction(OpCode.LOAD_NAME, 0),      # 8: push total
                Instruction(OpCode.LOAD_NAME, 1),      # 9: push i
                Instruction(OpCode.ADD),                # 10: total + i
                Instruction(OpCode.STORE_NAME, 0),     # 11: total = ...

                # i = i + 1
                Instruction(OpCode.LOAD_NAME, 1),      # 12: push i
                Instruction(OpCode.LOAD_CONST, 1),     # 13: push 1
                Instruction(OpCode.ADD),                # 14: i + 1
                Instruction(OpCode.STORE_NAME, 1),     # 15: i = ...

                Instruction(OpCode.JUMP, 4),            # 16: back to loop

                # print(total)
                Instruction(OpCode.LOAD_NAME, 0),      # 17: push total
                Instruction(OpCode.PRINT),              # 18: print
                Instruction(OpCode.HALT),               # 19: done
            ],
            constants=[0, 1, 5],
            names=["total", "i"],
        )
        assert vm.output == ["15"]
        assert vm.variables["total"] == 15
        assert vm.variables["i"] == 6  # i incremented past 5

    def test_string_concatenation(self) -> None:
        """Compile and run: greeting = "hello" + " " + "world"

        Demonstrates that the VM handles strings with the same arithmetic
        opcodes as integers — dynamic typing in action.
        """
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),   # "hello"
                Instruction(OpCode.LOAD_CONST, 1),   # " "
                Instruction(OpCode.ADD),              # "hello "
                Instruction(OpCode.LOAD_CONST, 2),   # "world"
                Instruction(OpCode.ADD),              # "hello world"
                Instruction(OpCode.STORE_NAME, 0),   # greeting = "hello world"
                Instruction(OpCode.HALT),
            ],
            constants=["hello", " ", "world"],
            names=["greeting"],
        )
        assert vm.variables["greeting"] == "hello world"


# =========================================================================
# assemble_code helper
# =========================================================================


class TestAssembleCode:
    """Tests for the assemble_code convenience function."""

    def test_basic_assembly(self) -> None:
        """assemble_code should produce a valid CodeObject."""
        code = assemble_code(
            [Instruction(OpCode.HALT)],
            constants=[1, 2, 3],
            names=["x"],
        )
        assert isinstance(code, CodeObject)
        assert len(code.instructions) == 1
        assert code.constants == [1, 2, 3]
        assert code.names == ["x"]

    def test_defaults_to_empty_pools(self) -> None:
        """assemble_code with no constants/names should use empty lists."""
        code = assemble_code([Instruction(OpCode.HALT)])
        assert code.constants == []
        assert code.names == []


# =========================================================================
# VM Reset
# =========================================================================


class TestReset:
    """Tests for the VM's reset method."""

    def test_reset_clears_state(self) -> None:
        """reset() should restore the VM to its initial state."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.STORE_NAME, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.PRINT),
                Instruction(OpCode.HALT),
            ],
            constants=[42, 99],
            names=["x"],
        )

        # VM has state now
        assert vm.variables != {}
        assert vm.output != []
        assert vm.halted is True

        # Reset should clear everything
        vm.reset()
        assert vm.stack == []
        assert vm.variables == {}
        assert vm.locals == []
        assert vm.pc == 0
        assert vm.halted is False
        assert vm.output == []
        assert vm.call_stack == []


# =========================================================================
# Instruction repr
# =========================================================================


class TestInstructionRepr:
    """Tests for the Instruction __repr__ method."""

    def test_repr_with_operand(self) -> None:
        """Instructions with operands should show them."""
        instr = Instruction(OpCode.LOAD_CONST, 0)
        assert "LOAD_CONST" in repr(instr)
        assert "0" in repr(instr)

    def test_repr_without_operand(self) -> None:
        """Instructions without operands should show just the opcode name."""
        instr = Instruction(OpCode.ADD)
        assert "ADD" in repr(instr)

    def test_repr_with_string_operand(self) -> None:
        """String operands should be shown with quotes."""
        instr = Instruction(OpCode.LOAD_CONST, "hello")
        r = repr(instr)
        assert "LOAD_CONST" in r
        assert "hello" in r


# =========================================================================
# Edge Cases and Additional Coverage
# =========================================================================


class TestEdgeCases:
    """Additional edge case tests for thorough coverage."""

    def test_dup_preserves_original(self) -> None:
        """DUP should not affect the original value below."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.DUP),
                Instruction(OpCode.HALT),
            ],
            constants=[10, 20],
        )
        assert vm.stack == [10, 20, 20]

    def test_cmp_lt_equal_values(self) -> None:
        """CMP_LT with equal values should push 0."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.CMP_LT),
                Instruction(OpCode.HALT),
            ],
            constants=[5],
        )
        assert vm.stack == [0]

    def test_cmp_gt_equal_values(self) -> None:
        """CMP_GT with equal values should push 0."""
        vm, _ = run(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.CMP_GT),
                Instruction(OpCode.HALT),
            ],
            constants=[5],
        )
        assert vm.stack == [0]

    def test_mul_underflow(self) -> None:
        """MUL on empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.MUL), Instruction(OpCode.HALT)])

    def test_div_underflow(self) -> None:
        """DIV on empty stack should raise StackUnderflowError."""
        with pytest.raises(StackUnderflowError):
            run([Instruction(OpCode.DIV), Instruction(OpCode.HALT)])

    def test_describe_div_by_zero(self) -> None:
        """The describe method should handle div-by-zero case in description."""
        code = assemble_code(
            [
                Instruction(OpCode.LOAD_CONST, 0),
                Instruction(OpCode.LOAD_CONST, 1),
                Instruction(OpCode.DIV),
                Instruction(OpCode.HALT),
            ],
            constants=[10, 0],
        )
        vm = VirtualMachine()
        # Execute LOAD_CONST twice
        vm.step(code)
        vm.step(code)
        # The DIV step will raise, but let's check the describe path
        # by calling _describe directly before the error
        desc = vm._describe(
            Instruction(OpCode.DIV),
            code,
            list(vm.stack),
        )
        assert "DIVISION BY ZERO" in desc

    def test_describe_with_empty_stack(self) -> None:
        """Descriptions should handle empty stacks gracefully."""
        code = assemble_code([Instruction(OpCode.POP)])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.POP), code, [])
        assert "?" in desc

    def test_describe_sub(self) -> None:
        """SUB description should show the difference."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.SUB), code, [10, 3])
        assert "7" in desc

    def test_describe_mul(self) -> None:
        """MUL description should show the product."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.MUL), code, [6, 7])
        assert "42" in desc

    def test_describe_jump(self) -> None:
        """JUMP description should mention the target."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.JUMP, 5), code, [])
        assert "5" in desc

    def test_describe_jump_if_true(self) -> None:
        """JUMP_IF_TRUE description should mention truthy and target."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(
            Instruction(OpCode.JUMP_IF_TRUE, 5), code, [1]
        )
        assert "truthy" in desc.lower()

    def test_describe_call(self) -> None:
        """CALL description should mention the function name."""
        code = assemble_code([], names=["my_func"])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.CALL, 0), code, [])
        assert "my_func" in desc

    def test_describe_return(self) -> None:
        """RETURN description should mention returning."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.RETURN), code, [])
        assert "return" in desc.lower() or "Return" in desc

    def test_describe_halt(self) -> None:
        """HALT description should mention halting."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.HALT), code, [])
        assert "halt" in desc.lower() or "Halt" in desc

    def test_describe_load_local(self) -> None:
        """LOAD_LOCAL description should mention the slot."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.LOAD_LOCAL, 3), code, [])
        assert "3" in desc

    def test_describe_store_local(self) -> None:
        """STORE_LOCAL description should mention the slot and value."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.STORE_LOCAL, 2), code, [99])
        assert "2" in desc

    def test_is_falsy(self) -> None:
        """Test the _is_falsy static method with various values."""
        assert VirtualMachine._is_falsy(0) is True
        assert VirtualMachine._is_falsy(None) is True
        assert VirtualMachine._is_falsy("") is True
        assert VirtualMachine._is_falsy(1) is False
        assert VirtualMachine._is_falsy("hello") is False
        assert VirtualMachine._is_falsy(-1) is False
        assert VirtualMachine._is_falsy(42) is False

    def test_call_frame_dataclass(self) -> None:
        """CallFrame should store return address and saved state."""
        frame = CallFrame(
            return_address=5,
            saved_variables={"x": 42},
            saved_locals=[1, 2, 3],
        )
        assert frame.return_address == 5
        assert frame.saved_variables == {"x": 42}
        assert frame.saved_locals == [1, 2, 3]

    def test_vm_trace_dataclass(self) -> None:
        """VMTrace should store all fields correctly."""
        trace = VMTrace(
            pc=0,
            instruction=Instruction(OpCode.LOAD_CONST, 0),
            stack_before=[],
            stack_after=[42],
            variables={},
            output=None,
            description="Push constant 42",
        )
        assert trace.pc == 0
        assert trace.stack_after == [42]
        assert trace.output is None
        assert trace.description == "Push constant 42"

    def test_describe_cmp_eq_with_empty_stack(self) -> None:
        """CMP_EQ description should handle empty stacks."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.CMP_EQ), code, [])
        assert "equality" in desc.lower() or "Compare" in desc

    def test_describe_cmp_lt_with_empty_stack(self) -> None:
        """CMP_LT description should handle empty stacks."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.CMP_LT), code, [])
        assert "less" in desc.lower() or "Compare" in desc

    def test_describe_cmp_gt_with_empty_stack(self) -> None:
        """CMP_GT description should handle empty stacks."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.CMP_GT), code, [])
        assert "greater" in desc.lower() or "Compare" in desc

    def test_describe_add_with_empty_stack(self) -> None:
        """ADD description should handle empty stacks."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.ADD), code, [])
        assert "Add" in desc or "add" in desc.lower()

    def test_describe_dup(self) -> None:
        """DUP description should mention the value being duplicated."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.DUP), code, [42])
        assert "42" in desc

    def test_describe_load_name(self) -> None:
        """LOAD_NAME description should mention the variable name."""
        code = assemble_code([], names=["foo"])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.LOAD_NAME, 0), code, [])
        assert "foo" in desc

    def test_describe_print(self) -> None:
        """PRINT description should mention the value being printed."""
        code = assemble_code([])
        vm = VirtualMachine()
        desc = vm._describe(Instruction(OpCode.PRINT), code, [42])
        assert "42" in desc
