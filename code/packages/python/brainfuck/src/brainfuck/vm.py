"""Brainfuck VM Factory — Plugging Brainfuck Into the GenericVM.

==========================================================================
The Factory Pattern
==========================================================================

This module provides ``create_brainfuck_vm()`` — a factory function that
creates a GenericVM fully configured for Brainfuck. It:

1. Creates a fresh GenericVM instance.
2. Attaches Brainfuck-specific state (tape, data pointer, input buffer).
3. Registers all 9 opcode handlers.

The result is a GenericVM that speaks Brainfuck — same execution engine
as Starlark, different language semantics.

==========================================================================
Convenience Executor
==========================================================================

For simple use cases, ``execute_brainfuck()`` wraps the full pipeline::

    result = execute_brainfuck("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.")

This translates the source, creates a VM, and executes in one call.
"""

from __future__ import annotations

from dataclasses import dataclass

from virtual_machine import CodeObject, VMTrace
from virtual_machine.generic_vm import GenericVM

from brainfuck.handlers import HANDLERS, TAPE_SIZE
from brainfuck.translator import translate


@dataclass
class BrainfuckResult:
    """The result of executing a Brainfuck program.

    Attributes
    ----------
    output : str
        The program's output (concatenation of all ``.`` commands).
    tape : list[int]
        The final state of the tape (all 30,000 cells).
    dp : int
        The final data pointer position.
    traces : list[VMTrace]
        Step-by-step execution traces (for debugging/visualization).
    steps : int
        Total number of instructions executed.
    """

    output: str
    tape: list[int]
    dp: int
    traces: list[VMTrace]
    steps: int


def create_brainfuck_vm(input_data: str = "") -> GenericVM:
    """Create a GenericVM configured for Brainfuck execution.

    This is the factory function that wires up Brainfuck's handlers and
    state. The returned VM is ready to execute any Brainfuck CodeObject.

    Parameters
    ----------
    input_data : str
        Input to feed to ``,`` commands. Each character is one byte.
        Default is empty (all ``,`` commands produce 0 / EOF).

    Returns
    -------
    GenericVM
        A VM with Brainfuck handlers registered and tape initialized.

    Example
    -------
    >>> from brainfuck import translate, create_brainfuck_vm
    >>> code = translate("+++.")
    >>> vm = create_brainfuck_vm()
    >>> traces = vm.execute(code)
    >>> "".join(vm.output)
    '\\x03'
    """
    vm = GenericVM()

    # -- Attach Brainfuck-specific state ----------------------------------
    # Python lets us add arbitrary attributes to objects. The handlers
    # read and write these attributes to implement Brainfuck semantics.
    vm.tape = [0] * TAPE_SIZE  # type: ignore[attr-defined]
    vm.dp = 0  # type: ignore[attr-defined]
    vm.input_buffer = input_data  # type: ignore[attr-defined]
    vm.input_pos = 0  # type: ignore[attr-defined]

    # -- Register all opcode handlers -------------------------------------
    for opcode, handler in HANDLERS.items():
        vm.register_opcode(opcode, handler)

    return vm


def execute_brainfuck(
    source: str,
    input_data: str = "",
) -> BrainfuckResult:
    """Translate and execute a Brainfuck program in one call.

    This is the convenience function for quick execution. It handles
    the full pipeline: source → translate → create VM → execute → result.

    Parameters
    ----------
    source : str
        The Brainfuck source code.
    input_data : str
        Input bytes for ``,`` commands.

    Returns
    -------
    BrainfuckResult
        The program's output, final tape state, and execution traces.

    Examples
    --------
    Simple addition (2 + 5 = 7):

    >>> result = execute_brainfuck("++>+++++[<+>-]")
    >>> result.tape[0]
    7

    Hello character (ASCII 72 = 'H'):

    >>> result = execute_brainfuck("+++++++++[>++++++++<-]>.")
    >>> result.output
    'H'
    """
    code: CodeObject = translate(source)
    vm: GenericVM = create_brainfuck_vm(input_data)
    traces: list[VMTrace] = vm.execute(code)

    return BrainfuckResult(
        output="".join(vm.output),
        tape=list(vm.tape),  # type: ignore[attr-defined]
        dp=vm.dp,  # type: ignore[attr-defined]
        traces=traces,
        steps=len(traces),
    )
