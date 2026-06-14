"""End-to-end execution tests for ``BrainfuckVM``.

For each program we either assert against a known-good byte sequence or
against the output of the existing :func:`brainfuck.execute_brainfuck`
reference interpreter.  The latter parity check is the strongest
correctness signal we can write — if the IIR-compiled program behaves
like the reference Brainfuck interpreter on representative inputs, the
LANG pipeline is doing its job.
"""

from __future__ import annotations

import pytest
from brainfuck import execute_brainfuck

from brainfuck_iir_compiler import BrainfuckVM

HELLO_WORLD = (
    "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>."
    ">---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
)


# ---------------------------------------------------------------------------
# Tiny golden cases
# ---------------------------------------------------------------------------


def test_empty_program_produces_no_output() -> None:
    vm = BrainfuckVM()
    assert vm.run("") == b""


def test_single_increment_then_output() -> None:
    vm = BrainfuckVM()
    assert vm.run("+.") == b"\x01"


def test_three_increments_then_output() -> None:
    vm = BrainfuckVM()
    assert vm.run("+++.") == b"\x03"


def test_underflow_wraps_via_u8() -> None:
    # Decrement once from cell 0 (which is 0) → wraps to 255
    vm = BrainfuckVM()
    assert vm.run("-.") == b"\xff"


def test_overflow_wraps_via_u8() -> None:
    # Increment 256 times from cell 0 → wraps back to 0
    source = "+" * 256 + "."
    vm = BrainfuckVM()
    assert vm.run(source) == b"\x00"


def test_pointer_movement_isolates_cells() -> None:
    # cell 0 := 2, cell 1 := 3, output cell 0 then cell 1
    vm = BrainfuckVM()
    out = vm.run("++>+++<.>.")
    assert out == b"\x02\x03"


# ---------------------------------------------------------------------------
# The classic move-and-add loop
# ---------------------------------------------------------------------------


def test_canonical_loop_moves_value() -> None:
    # ++[>+<-]>. — moves 2 from cell 0 into cell 1, then outputs cell 1
    vm = BrainfuckVM()
    assert vm.run("++[>+<-]>.") == b"\x02"


def test_loop_skipped_when_cell_already_zero() -> None:
    # Cell 0 starts at 0, so the loop body never runs.  Output should be
    # the initial 0.
    vm = BrainfuckVM()
    assert vm.run("[>+<-].") == b"\x00"


def test_zero_cell_idiom() -> None:
    # `[-]` is the canonical "zero this cell" idiom.
    vm = BrainfuckVM()
    assert vm.run("+++++[-].") == b"\x00"


# ---------------------------------------------------------------------------
# I/O
# ---------------------------------------------------------------------------


def test_echo_one_byte() -> None:
    vm = BrainfuckVM()
    assert vm.run(",.", input_bytes=b"X") == b"X"


def test_echo_multiple_bytes() -> None:
    vm = BrainfuckVM()
    assert vm.run(",.,.,.", input_bytes=b"abc") == b"abc"


def test_eof_yields_zero() -> None:
    vm = BrainfuckVM()
    # Input is empty, so `,` reads 0; `.` outputs the 0 byte.
    assert vm.run(",.", input_bytes=b"") == b"\x00"


# ---------------------------------------------------------------------------
# Hello, World!
# ---------------------------------------------------------------------------


def test_hello_world_classic_program() -> None:
    vm = BrainfuckVM()
    assert vm.run(HELLO_WORLD) == b"Hello World!\n"


# ---------------------------------------------------------------------------
# Reference parity — every output must match the existing brainfuck VM
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "source,input_bytes",
    [
        ("+.", b""),
        ("+++>++<.>.", b""),
        ("++[>+<-]>.", b""),
        ("+++++[-].", b""),
        ("[+]", b""),  # never enters the loop, no output
        (",.", b"Z"),
        (",+.", b"A"),  # input + 1
        (",.,.", b"\x10\x20"),
        (HELLO_WORLD, b""),
    ],
)
def test_matches_reference_interpreter(source: str, input_bytes: bytes) -> None:
    vm = BrainfuckVM()
    iir_output = vm.run(source, input_bytes=input_bytes)

    ref_input_str = input_bytes.decode("latin-1")
    ref_result = execute_brainfuck(source, input_data=ref_input_str)
    ref_output = ref_result.output.encode("latin-1")

    assert iir_output == ref_output, (
        f"divergence on {source!r} with input {input_bytes!r}: "
        f"IIR={iir_output!r} vs ref={ref_output!r}"
    )
