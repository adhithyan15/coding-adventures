"""Tests for individual Brainfuck opcode handlers."""

from __future__ import annotations

import pytest

from virtual_machine import CodeObject, Instruction

from brainfuck.handlers import HANDLERS, TAPE_SIZE, BrainfuckError
from brainfuck.opcodes import Op
from brainfuck.vm import create_brainfuck_vm


def exec_code(
    instructions: list[Instruction],
    input_data: str = "",
) -> object:
    """Helper: create a BF VM, execute instructions, return the VM."""
    code = CodeObject(
        instructions=instructions + [Instruction(Op.HALT, None)],
        constants=[],
        names=[],
    )
    vm = create_brainfuck_vm(input_data)
    vm.execute(code)
    return vm


class TestHandlerRegistry:
    """All 9 handlers are registered."""

    def test_all_opcodes_registered(self) -> None:
        assert len(HANDLERS) == 9

    def test_handler_opcodes(self) -> None:
        expected = {
            Op.RIGHT, Op.LEFT, Op.INC, Op.DEC,
            Op.OUTPUT, Op.INPUT, Op.LOOP_START, Op.LOOP_END, Op.HALT,
        }
        assert set(HANDLERS.keys()) == expected

    def test_vm_has_all_handlers(self) -> None:
        vm = create_brainfuck_vm()
        for opcode in HANDLERS:
            assert opcode in vm._handlers


class TestPointerMovement:
    """> and < handlers."""

    def test_right_moves_pointer(self) -> None:
        vm = exec_code([Instruction(Op.RIGHT, None)])
        assert vm.dp == 1  # type: ignore[attr-defined]

    def test_left_moves_pointer(self) -> None:
        vm = exec_code([
            Instruction(Op.RIGHT, None),
            Instruction(Op.LEFT, None),
        ])
        assert vm.dp == 0  # type: ignore[attr-defined]

    def test_multiple_rights(self) -> None:
        vm = exec_code([Instruction(Op.RIGHT, None)] * 10)
        assert vm.dp == 10  # type: ignore[attr-defined]

    def test_left_at_zero_raises(self) -> None:
        with pytest.raises(BrainfuckError, match="before start"):
            exec_code([Instruction(Op.LEFT, None)])

    def test_right_past_tape_raises(self) -> None:
        with pytest.raises(BrainfuckError, match="past end"):
            exec_code([Instruction(Op.RIGHT, None)] * TAPE_SIZE)


class TestCellModification:
    """+ and - handlers."""

    def test_inc_increments_cell(self) -> None:
        vm = exec_code([Instruction(Op.INC, None)])
        assert vm.tape[0] == 1  # type: ignore[attr-defined]

    def test_multiple_incs(self) -> None:
        vm = exec_code([Instruction(Op.INC, None)] * 5)
        assert vm.tape[0] == 5  # type: ignore[attr-defined]

    def test_dec_decrements_cell(self) -> None:
        vm = exec_code([
            Instruction(Op.INC, None),
            Instruction(Op.INC, None),
            Instruction(Op.DEC, None),
        ])
        assert vm.tape[0] == 1  # type: ignore[attr-defined]

    def test_inc_wraps_at_255(self) -> None:
        """255 + 1 = 0 (byte wrapping)."""
        vm = exec_code([Instruction(Op.INC, None)] * 256)
        assert vm.tape[0] == 0  # type: ignore[attr-defined]

    def test_dec_wraps_at_0(self) -> None:
        """0 - 1 = 255 (byte wrapping)."""
        vm = exec_code([Instruction(Op.DEC, None)])
        assert vm.tape[0] == 255  # type: ignore[attr-defined]

    def test_inc_different_cell(self) -> None:
        """Increment a cell that isn't cell 0."""
        vm = exec_code([
            Instruction(Op.RIGHT, None),
            Instruction(Op.INC, None),
            Instruction(Op.INC, None),
        ])
        assert vm.tape[0] == 0  # type: ignore[attr-defined]
        assert vm.tape[1] == 2  # type: ignore[attr-defined]


class TestOutput:
    """. handler."""

    def test_output_ascii(self) -> None:
        """Output cell value as ASCII character."""
        # Set cell to 65 ('A')
        vm = exec_code([Instruction(Op.INC, None)] * 65 + [Instruction(Op.OUTPUT, None)])
        assert vm.output == ["A"]

    def test_output_zero(self) -> None:
        """Output cell 0 = null character."""
        vm = exec_code([Instruction(Op.OUTPUT, None)])
        assert vm.output == ["\x00"]

    def test_multiple_outputs(self) -> None:
        vm = exec_code([
            Instruction(Op.INC, None),  # cell = 1
            Instruction(Op.OUTPUT, None),
            Instruction(Op.INC, None),  # cell = 2
            Instruction(Op.OUTPUT, None),
        ])
        assert len(vm.output) == 2


class TestInput:
    """, handler."""

    def test_read_one_byte(self) -> None:
        vm = exec_code(
            [Instruction(Op.INPUT, None)],
            input_data="A",
        )
        assert vm.tape[0] == 65  # type: ignore[attr-defined]  # ord('A')

    def test_read_multiple_bytes(self) -> None:
        vm = exec_code(
            [
                Instruction(Op.INPUT, None),
                Instruction(Op.RIGHT, None),
                Instruction(Op.INPUT, None),
            ],
            input_data="AB",
        )
        assert vm.tape[0] == 65  # type: ignore[attr-defined]
        assert vm.tape[1] == 66  # type: ignore[attr-defined]

    def test_eof_gives_zero(self) -> None:
        """Reading past end of input gives 0."""
        vm = exec_code(
            [Instruction(Op.INPUT, None)],
            input_data="",
        )
        assert vm.tape[0] == 0  # type: ignore[attr-defined]

    def test_eof_after_input(self) -> None:
        vm = exec_code(
            [
                Instruction(Op.INPUT, None),
                Instruction(Op.RIGHT, None),
                Instruction(Op.INPUT, None),
            ],
            input_data="X",
        )
        assert vm.tape[0] == ord("X")  # type: ignore[attr-defined]
        assert vm.tape[1] == 0  # type: ignore[attr-defined]  # EOF


class TestControlFlow:
    """[ and ] handlers."""

    def test_skip_loop_when_zero(self) -> None:
        """[..] is skipped entirely if cell is 0."""
        code = CodeObject(
            instructions=[
                Instruction(Op.LOOP_START, 3),  # skip to index 3
                Instruction(Op.INC, None),       # should be skipped
                Instruction(Op.LOOP_END, 0),     # should be skipped
                Instruction(Op.HALT, None),
            ],
            constants=[],
            names=[],
        )
        vm = create_brainfuck_vm()
        vm.execute(code)
        assert vm.tape[0] == 0  # type: ignore[attr-defined]  # INC was skipped

    def test_enter_loop_when_nonzero(self) -> None:
        """Loop body executes when cell != 0."""
        code = CodeObject(
            instructions=[
                Instruction(Op.INC, None),       # cell = 1
                Instruction(Op.LOOP_START, 5),   # cell != 0, enter loop
                Instruction(Op.DEC, None),       # cell = 0
                Instruction(Op.RIGHT, None),     # dp = 1
                Instruction(Op.LOOP_END, 1),     # cell[1] == 0, exit
                Instruction(Op.HALT, None),
            ],
            constants=[],
            names=[],
        )
        vm = create_brainfuck_vm()
        vm.execute(code)
        assert vm.tape[0] == 0  # type: ignore[attr-defined]
        assert vm.dp == 1  # type: ignore[attr-defined]

    def test_loop_repeats(self) -> None:
        """Loop body repeats until cell becomes 0."""
        # Set cell to 3, then loop: [>+<-] (move value to cell 1)
        code = CodeObject(
            instructions=[
                Instruction(Op.INC, None),       # cell[0] = 1
                Instruction(Op.INC, None),       # cell[0] = 2
                Instruction(Op.INC, None),       # cell[0] = 3
                Instruction(Op.LOOP_START, 8),   # [
                Instruction(Op.RIGHT, None),     # dp = 1
                Instruction(Op.INC, None),       # cell[1]++
                Instruction(Op.LEFT, None),      # dp = 0
                Instruction(Op.DEC, None),       # cell[0]--
                Instruction(Op.LOOP_END, 3),     # ]
                Instruction(Op.HALT, None),
            ],
            constants=[],
            names=[],
        )
        vm = create_brainfuck_vm()
        vm.execute(code)
        assert vm.tape[0] == 0  # type: ignore[attr-defined]
        assert vm.tape[1] == 3  # type: ignore[attr-defined]


class TestVMState:
    """GenericVM state initialization."""

    def test_tape_size(self) -> None:
        vm = create_brainfuck_vm()
        assert len(vm.tape) == TAPE_SIZE  # type: ignore[attr-defined]

    def test_tape_initialized_to_zero(self) -> None:
        vm = create_brainfuck_vm()
        assert all(c == 0 for c in vm.tape)  # type: ignore[attr-defined]

    def test_dp_starts_at_zero(self) -> None:
        vm = create_brainfuck_vm()
        assert vm.dp == 0  # type: ignore[attr-defined]

    def test_input_buffer_set(self) -> None:
        vm = create_brainfuck_vm(input_data="hello")
        assert vm.input_buffer == "hello"  # type: ignore[attr-defined]
        assert vm.input_pos == 0  # type: ignore[attr-defined]
