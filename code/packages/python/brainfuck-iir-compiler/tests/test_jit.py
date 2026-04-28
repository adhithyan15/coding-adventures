"""End-to-end tests for ``BrainfuckVM(jit=True)``.

The BF05 contract for the JIT path:

- Programs without I/O (no ``.``, no ``,``) compile to WASM via
  ``WASMBackend`` and run on ``wasm-runtime``.  ``is_jit_compiled``
  becomes ``True`` after the run.
- Programs with I/O fail to lower (``call_builtin`` is not yet handled
  by ``cir-to-compiler-ir``).  The JIT path silently deopts to the
  interpreter.  ``is_jit_compiled`` stays ``False``.  Behaviour
  (output bytes) is unchanged.

These tests exercise both branches.  We don't yet have a way to inspect
the WASM linear memory after a JIT'd run (that's BF06's WASI plumbing),
so for I/O-free programs we assert ``is_jit_compiled`` and verify the
program ran without raising — proving the pipeline is end-to-end alive.
"""

from __future__ import annotations

from brainfuck_iir_compiler import BrainfuckVM

# ---------------------------------------------------------------------------
# I/O-free programs JIT successfully
# ---------------------------------------------------------------------------


def test_empty_program_jits() -> None:
    vm = BrainfuckVM(jit=True)
    out = vm.run("")
    assert out == b""
    assert vm.is_jit_compiled is True, (
        "an empty program lowers cleanly (just const + ret_void) — "
        "the JIT must succeed here"
    )


def test_pure_arithmetic_program_jits() -> None:
    """No loops, no I/O, just `+` / `>` — exercises load_mem, store_mem,
    add, const, sub.  All of these have lowerings in BF05.
    """
    vm = BrainfuckVM(jit=True)
    out = vm.run("+++>++<")
    assert out == b""
    assert vm.is_jit_compiled is True


def test_loop_program_jits() -> None:
    """A bounded loop with no I/O.  ``+++[->+++<]`` moves 9 from cell 0
    to cell 1.  Exercises label / jmp_if_false / jmp_if_true on top of
    the arithmetic ops.
    """
    vm = BrainfuckVM(jit=True)
    out = vm.run("+++[->+++<]")
    assert out == b""
    assert vm.is_jit_compiled is True


def test_zero_cell_idiom_jits() -> None:
    """`[-]` is the canonical "zero this cell" idiom.  No I/O."""
    vm = BrainfuckVM(jit=True)
    out = vm.run("+++++[-]")
    assert out == b""
    assert vm.is_jit_compiled is True


# ---------------------------------------------------------------------------
# I/O programs gracefully deopt
# ---------------------------------------------------------------------------
#
# These programs include `.` or `,`, which compile to ``call_builtin``.
# ``cir-to-compiler-ir`` does not yet lower ``call_builtin`` (BF06).
# The JIT path's exception handler catches the lowering failure and
# falls back to the interpreter — so the user-visible behaviour is
# identical to ``jit=False``.


def test_putchar_program_deopts_and_runs_interpreted() -> None:
    vm = BrainfuckVM(jit=True)
    out = vm.run("+++.")
    assert out == b"\x03"  # interpreter ran correctly
    assert vm.is_jit_compiled is False, (
        "programs with `.` should deopt because call_builtin is not yet "
        "lowered (BF06)"
    )


def test_getchar_program_deopts_and_runs_interpreted() -> None:
    vm = BrainfuckVM(jit=True)
    out = vm.run(",.", input_bytes=b"X")
    assert out == b"X"  # interpreter ran correctly
    assert vm.is_jit_compiled is False


def test_hello_world_deopts_and_runs_interpreted() -> None:
    """The classic Hello World — uses many `.` calls — must deopt
    cleanly and produce the right bytes via the interpreter.
    """
    hello = (
        "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>."
        ">---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
    )
    vm = BrainfuckVM(jit=True)
    assert vm.run(hello) == b"Hello World!\n"
    assert vm.is_jit_compiled is False


# ---------------------------------------------------------------------------
# Output parity: jit=True and jit=False produce identical bytes
# ---------------------------------------------------------------------------


def test_jit_and_interpreted_produce_same_output_for_io_free_programs() -> None:
    program = "+++[->+++<]"  # no output either way
    out_interp = BrainfuckVM(jit=False).run(program)
    out_jit = BrainfuckVM(jit=True).run(program)
    assert out_jit == out_interp


def test_jit_and_interpreted_produce_same_output_for_io_programs() -> None:
    program = "++[>++++<-]>."  # outputs '\x08'
    out_interp = BrainfuckVM(jit=False).run(program)
    out_jit = BrainfuckVM(jit=True).run(program)
    assert out_jit == out_interp


# ---------------------------------------------------------------------------
# is_jit_compiled is sticky-per-run, not sticky-forever
# ---------------------------------------------------------------------------


def test_is_jit_compiled_resets_on_each_run() -> None:
    """A successful JIT run sets is_jit_compiled True; a subsequent
    run that deopts must reset it to False (and vice versa).
    """
    vm = BrainfuckVM(jit=True)

    vm.run("+++")
    assert vm.is_jit_compiled is True

    vm.run("+++.")
    assert vm.is_jit_compiled is False

    vm.run("++>+<")
    assert vm.is_jit_compiled is True
