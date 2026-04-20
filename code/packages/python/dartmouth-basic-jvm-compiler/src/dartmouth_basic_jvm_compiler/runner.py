"""Dartmouth BASIC → JVM pipeline runner.

This module is the public interface for the full compiled JVM pipeline.  It
chains four independent packages — parser, IR compiler, JVM lowerer, and JVM
simulator — into one ``run_basic()`` call that accepts a BASIC source string
and returns the program's standard output.

Pipeline
--------

1. **Lex & parse** (``dartmouth_basic_parser``) — tokenise the BASIC source
   and build an AST.
2. **IR compile** (``dartmouth_basic_ir_compiler``) — lower the AST to a
   target-independent ``IrProgram``.  The ``char_encoding="ascii"`` flag makes
   all ``PRINT`` char codes use standard ASCII byte values so the JVM
   ``PrintStream.write()`` call produces correct output.
3. **JVM lower** (``ir_to_jvm_class_file``) — lower the ``IrProgram`` to a
   JVM ``.class`` file.  ``syscall_arg_reg=0`` tells the lowerer that the
   BASIC IR places the print argument in register 0 (not the Brainfuck
   default of 4).
4. **Run** (``jvm_runtime``) — load the class file into the JVM simulator,
   invoke the ``_start`` method, and capture stdout.

Character encoding note
-----------------------

The GE-225 typewriter used a proprietary 6-bit character code.  The JVM
backend writes raw bytes to stdout via ``System.out.write(byte)``, which
expects standard ASCII.  The IR compiler therefore runs in
``char_encoding="ascii"`` mode: string literals are emitted as their
``ord()`` values and numeric digits are offset by 48 (``ord('0')``) before
``SYSCALL 1``.

Variable values
---------------

Unlike the GE-225 backend (which stores all registers as memory words that
can be read after halting), the JVM backend maps each IR virtual register to
a JVM static field.  The JVM simulator does not expose these fields after
execution, so ``RunResult.var_values`` is always empty for the JVM backend.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class BasicError(Exception):
    """Raised when the BASIC program cannot be compiled or executed.

    Wraps ``CompileError`` from the IR compiler, ``JvmBackendError`` from
    the JVM lowerer, and runtime errors from the JVM simulator.  The original
    exception is available as ``__cause__``.

    Examples::

        try:
            run_basic("10 GOSUB 100\\n20 END\\n")
        except BasicError as e:
            print(e)  # "GOSUB is not supported in V1 of the compiled pipeline"
    """


@dataclass
class RunResult:
    """Outcome of a successful ``run_basic()`` call.

    Attributes:
        output:      Standard output produced by ``PRINT`` statements (ASCII
                     text, newline-terminated).
        var_values:  Always ``{}`` for the JVM backend.  The JVM simulator
                     does not expose register state after execution.
        steps:       Always ``0`` for the JVM backend (no instruction
                     counter in the JVM runtime).
        halt_address: Always ``0`` for the JVM backend (halt is a method
                      return, not a self-loop at a fixed address).

    Example::

        result = run_basic("10 LET A = 6 * 7\\n20 PRINT A\\n30 END\\n")
        assert result.output == "42\\n"
    """

    output: str
    var_values: dict[str, int] = field(default_factory=dict)
    steps: int = 0
    halt_address: int = 0


def run_basic(
    source: str,
    *,
    max_steps: int = 100_000,  # noqa: ARG001 — accepted for API parity with GE-225 runner
) -> RunResult:
    """Compile and run a Dartmouth BASIC program via the JVM simulator.

    This is the single public entry point.  It chains all four pipeline stages
    and returns the program's standard output.

    Args:
        source:    Dartmouth BASIC source text.  Lines must begin with a line
                   number followed by a statement.  Newlines between lines are
                   required.
        max_steps: Accepted for interface parity with the GE-225 runner but
                   not enforced; the JVM simulator has no step counter.

    Returns:
        A ``RunResult`` with the standard output.  ``var_values``, ``steps``,
        and ``halt_address`` are always zero/empty for the JVM backend.

    Raises:
        BasicError: If the program cannot be parsed, uses a V1-unsupported
                    feature (GOSUB, DIM, INPUT, arrays, ``^`` operator), the
                    JVM lowerer cannot handle the IR, or the JVM simulator
                    raises an unexpected error.

    Example — sum of 1 to 100::

        result = run_basic(\"\"\"
        10 LET S = 0
        20 FOR I = 1 TO 100
        30 LET S = S + I
        40 NEXT I
        50 PRINT S
        60 END
        \"\"\")
        assert result.output == "5050\\n"
    """
    # ── Stage 1: parse ───────────────────────────────────────────────────────
    try:
        from dartmouth_basic_parser import parse_dartmouth_basic
        ast = parse_dartmouth_basic(source)
    except Exception as exc:
        raise BasicError(f"parse error: {exc}") from exc

    # ── Stage 2: IR compilation (ASCII char encoding for JVM) ────────────────
    try:
        from dartmouth_basic_ir_compiler import compile_basic
        ir_result = compile_basic(ast, char_encoding="ascii")
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 3: JVM lower ───────────────────────────────────────────────────
    try:
        from ir_to_jvm_class_file import JvmBackendConfig, lower_ir_to_jvm_class_file
        artifact = lower_ir_to_jvm_class_file(
            ir_result.program,
            JvmBackendConfig(
                class_name="BasicProgram",
                emit_main_wrapper=False,
            ),
        )
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 4: run ─────────────────────────────────────────────────────────
    try:
        from jvm_runtime import JVMRuntime
        runtime = JVMRuntime()
        jvm_result = runtime.run_method(
            artifact.class_bytes,
            method_name="_start",
            descriptor="()I",
        )
    except BasicError:
        raise
    except Exception as exc:
        raise BasicError(f"runtime error: {exc}") from exc

    return RunResult(output=jvm_result.output)
