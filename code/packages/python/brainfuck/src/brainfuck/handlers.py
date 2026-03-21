"""Brainfuck Opcode Handlers — Teaching the GenericVM a New Language.

==========================================================================
How Handlers Plug Into the GenericVM
==========================================================================

The GenericVM is a blank slate — it knows how to fetch-decode-execute
instructions, but it doesn't know what any opcode *means*. That's where
handlers come in.

Each handler is a function with the signature::

    def handle_xxx(vm, instruction, code) -> str | None

The handler receives:

- **vm** — The GenericVM instance. We use ``vm.tape``, ``vm.dp``,
  ``vm.output``, and ``vm.advance_pc()`` / ``vm.jump_to()``.
- **instruction** — The current instruction (opcode + optional operand).
- **code** — The CodeObject (unused by most Brainfuck handlers, since
  Brainfuck has no constant or name pools).

The handler returns a string if it produces output (the ``.`` command),
otherwise None.

==========================================================================
Brainfuck's Extra State
==========================================================================

The GenericVM provides a stack, variables, and locals — none of which
Brainfuck uses. Instead, Brainfuck needs:

- **tape** — A list of 30,000 byte cells, initialized to 0.
- **dp** (data pointer) — Index into the tape, starts at 0.
- **input_buffer** — Bytes to read from (simulates stdin).
- **input_pos** — Current position in the input buffer.

These are attached as attributes on the GenericVM instance in the
factory function (``create_brainfuck_vm()``). Python's dynamic nature
lets us add arbitrary attributes to any object. Rust and Go will need
a different approach (wrapper struct or extra fields).

==========================================================================
Cell Wrapping
==========================================================================

Brainfuck cells are unsigned bytes: values 0–255. Incrementing 255
wraps to 0; decrementing 0 wraps to 255. This is modular arithmetic::

    cell = (cell + 1) % 256   # INC
    cell = (cell - 1) % 256   # DEC

Python's ``%`` operator handles negative numbers correctly:
``(-1) % 256 == 255``. Some languages don't — watch out when porting.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from virtual_machine import CodeObject, Instruction

from brainfuck.opcodes import Op

if TYPE_CHECKING:
    from virtual_machine.generic_vm import GenericVM


# =========================================================================
# Tape size constant
# =========================================================================

TAPE_SIZE = 30_000
"""The number of cells on the Brainfuck tape.

The original Brainfuck specification uses 30,000 cells. Some implementations
use more (or even dynamically grow), but 30,000 is the classic size.
"""


# =========================================================================
# Error type
# =========================================================================


class BrainfuckError(Exception):
    """Runtime error during Brainfuck execution."""


# =========================================================================
# Pointer movement handlers
# =========================================================================


def handle_right(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``>`` — Move the data pointer one cell to the right.

    If the pointer is already at the last cell, this raises an error.
    Some Brainfuck implementations wrap around; we choose to error
    because silent wrapping hides bugs in BF programs.
    """
    vm.dp += 1  # type: ignore[attr-defined]
    if vm.dp >= TAPE_SIZE:  # type: ignore[attr-defined]
        raise BrainfuckError(
            f"Data pointer moved past end of tape (position {vm.dp}). "  # type: ignore[attr-defined]
            f"The tape has {TAPE_SIZE} cells (indices 0–{TAPE_SIZE - 1})."
        )
    vm.advance_pc()
    return None


def handle_left(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``<`` — Move the data pointer one cell to the left.

    If the pointer is already at cell 0, this raises an error.
    """
    vm.dp -= 1  # type: ignore[attr-defined]
    if vm.dp < 0:  # type: ignore[attr-defined]
        raise BrainfuckError(
            "Data pointer moved before start of tape (position -1). "
            "The tape starts at index 0."
        )
    vm.advance_pc()
    return None


# =========================================================================
# Cell modification handlers
# =========================================================================


def handle_inc(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``+`` — Increment the byte at the data pointer.

    Wraps from 255 to 0 (unsigned byte arithmetic).
    """
    vm.tape[vm.dp] = (vm.tape[vm.dp] + 1) % 256  # type: ignore[attr-defined]
    vm.advance_pc()
    return None


def handle_dec(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``-`` — Decrement the byte at the data pointer.

    Wraps from 0 to 255 (unsigned byte arithmetic).
    """
    vm.tape[vm.dp] = (vm.tape[vm.dp] - 1) % 256  # type: ignore[attr-defined]
    vm.advance_pc()
    return None


# =========================================================================
# I/O handlers
# =========================================================================


def handle_output(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``.`` — Output the current cell's value as an ASCII character.

    The character is appended to ``vm.output`` (the GenericVM's output
    capture list) and also returned as the handler's output string.
    """
    char = chr(vm.tape[vm.dp])  # type: ignore[attr-defined]
    vm.output.append(char)
    vm.advance_pc()
    return char


def handle_input(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``,`` — Read one byte of input into the current cell.

    Reads from ``vm.input_buffer`` at position ``vm.input_pos``.
    If the input is exhausted (EOF), the cell is set to 0.

    Different Brainfuck implementations handle EOF differently:
    - Set cell to 0 (our choice — clean and predictable)
    - Set cell to -1 (255 in unsigned)
    - Leave cell unchanged
    """
    if vm.input_pos < len(vm.input_buffer):  # type: ignore[attr-defined]
        vm.tape[vm.dp] = ord(vm.input_buffer[vm.input_pos])  # type: ignore[attr-defined]
        vm.input_pos += 1  # type: ignore[attr-defined]
    else:
        # EOF: set cell to 0
        vm.tape[vm.dp] = 0  # type: ignore[attr-defined]
    vm.advance_pc()
    return None


# =========================================================================
# Control flow handlers
# =========================================================================


def handle_loop_start(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``[`` — Jump forward past the matching ``]`` if the current cell is zero.

    If the cell is **nonzero**, execution continues to the next instruction
    (entering the loop body). If the cell is **zero**, the VM jumps to the
    instruction index stored in the operand (one past the matching ``]``),
    effectively skipping the loop entirely.

    This is the "while" test: ``while (tape[dp] != 0) { ... }``
    """
    if vm.tape[vm.dp] == 0:  # type: ignore[attr-defined]
        # Cell is zero — skip the loop
        vm.jump_to(instruction.operand)
    else:
        # Cell is nonzero — enter the loop
        vm.advance_pc()
    return None


def handle_loop_end(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """``]`` — Jump backward to the matching ``[`` if the current cell is nonzero.

    If the cell is **nonzero**, jump back to the matching ``[`` (which will
    re-test the condition). If the cell is **zero**, fall through to the next
    instruction (exiting the loop).

    Together with LOOP_START, this implements::

        while tape[dp] != 0:
            <loop body>
    """
    if vm.tape[vm.dp] != 0:  # type: ignore[attr-defined]
        # Cell is nonzero — loop again
        vm.jump_to(instruction.operand)
    else:
        # Cell is zero — exit loop
        vm.advance_pc()
    return None


# =========================================================================
# HALT handler
# =========================================================================


def handle_halt(
    vm: GenericVM, instruction: Instruction, code: CodeObject
) -> str | None:
    """Stop the VM."""
    vm.halted = True
    return None


# =========================================================================
# Handler registry — maps opcode numbers to handler functions
# =========================================================================

#: All Brainfuck opcode handlers, keyed by opcode number.
#: Used by ``create_brainfuck_vm()`` to register all handlers at once.
HANDLERS: dict[int, object] = {
    Op.RIGHT: handle_right,
    Op.LEFT: handle_left,
    Op.INC: handle_inc,
    Op.DEC: handle_dec,
    Op.OUTPUT: handle_output,
    Op.INPUT: handle_input,
    Op.LOOP_START: handle_loop_start,
    Op.LOOP_END: handle_loop_end,
    Op.HALT: handle_halt,
}
