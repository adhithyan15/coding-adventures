"""Dartmouth BASIC → GE-225 pipeline runner.

This module is the public interface for the full compiled pipeline.  It
chains the four independent packages — parser, IR compiler, GE-225 backend,
and simulator — into one ``run_basic()`` call that accepts a BASIC source
string and returns the program's typewriter output together with the final
values of all BASIC variables.

Historical context
------------------

In 1964, a Dartmouth student typed a BASIC program on a Teletype terminal,
pressed RETURN, and within seconds the GE-225 time-sharing system printed the
output on the same terminal.  This runner recreates that sequence:

1. **Lex & parse** (``dartmouth_basic_parser``) — tokenise the BASIC source
   and build an AST.
2. **IR compile** (``dartmouth_basic_ir_compiler``) — lower the AST to a
   target-independent ``IrProgram`` where every variable occupies a fixed
   virtual register.
3. **GE-225 backend** (``ir_to_ge225_compiler``) — run a three-pass assembler
   that emits 20-bit GE-225 machine words packed three bytes each.
4. **Simulate** (``ge225_simulator``) — load the binary into a behavioural
   GE-225 and step until the halt stub is reached.

Memory layout reminder::

    addr 0           : TON  (enable typewriter — emitted by the backend)
    addr 1 …         : compiled IR code
    addr code_end    : BRU code_end  (halt self-loop)
    addr data_base … : spill slots (one per virtual register)
    addr …           : constants table

The simulator's typewriter subsystem collects characters as the program calls
SYSCALL 1 (``LDA v0; SAN 6; TYP``).  Carriage-return codes (GE-225 0o37) are
translated to Unix newlines for readability.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class BasicError(Exception):
    """Raised when the BASIC program cannot be compiled or executed.

    Wraps ``CompileError`` from the IR compiler, ``CodeGenError`` from the
    GE-225 backend, and runtime errors (e.g. division by zero in the simulator).
    The original exception is available as ``__cause__``.

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
        output:      Typewriter output produced by ``PRINT`` statements.
                     GE-225 carriage-return codes (0o37) are converted to
                     Unix newlines (``\\n``).
        var_values:  Final integer values of all 26 BASIC scalar variables
                     A–Z after the program halts.  The 20-bit two's-complement
                     words are sign-extended to Python integers.
        steps:       Number of GE-225 instructions executed (useful for
                     benchmarking and loop-count verification).
        halt_address: Word address of the halt stub (``BRU halt_address``).
                     The integration layer stops when
                     ``trace.address == halt_address``.

    Example::

        result = run_basic("10 LET A = 6 * 7\\n20 END\\n")
        assert result.var_values["A"] == 42
        assert result.output == ""
    """

    output: str
    var_values: dict[str, int] = field(default_factory=dict)
    steps: int = 0
    halt_address: int = 0


def run_basic(
    source: str,
    *,
    memory_words: int = 4096,
    max_steps: int = 100_000,
) -> RunResult:
    """Compile and run a Dartmouth BASIC program on the GE-225 simulator.

    This is the single public entry point.  It chains all four pipeline
    stages and returns the program's output together with its final variable
    state.

    Args:
        source:       Dartmouth BASIC source text.  Lines must begin with a
                      line number followed by a statement.  Newlines between
                      lines are required.
        memory_words: Total GE-225 memory in 20-bit words (default 4096,
                      i.e. the full 4K machine). Increase for programs with
                      many variables or long PRINT chains.
        max_steps:    Safety limit on the number of simulated instructions.
                      Raises ``BasicError`` if the program has not halted by
                      this point (likely an infinite loop).

    Returns:
        A ``RunResult`` with the typewriter output, final variable values,
        instruction count, and halt address.

    Raises:
        BasicError: If the program cannot be parsed, uses a V1-unsupported
                    feature (GOSUB, DIM, INPUT, arrays, ``^`` operator), the
                    string contains a character with no GE-225 typewriter code,
                    a division by zero occurs, or ``max_steps`` is reached.

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
        assert result.var_values["S"] == 5050

    Example — mixed PRINT::

        result = run_basic(\"\"\"
        10 LET X = 6
        20 LET Y = 7
        30 PRINT \"PRODUCT IS \", X * Y
        40 END
        \"\"\")
        assert result.output == "PRODUCT IS 42\\n"
    """
    # ── Stage 1: parse ───────────────────────────────────────────────────────
    try:
        from dartmouth_basic_parser import parse_dartmouth_basic
        ast = parse_dartmouth_basic(source)
    except Exception as exc:
        raise BasicError(f"parse error: {exc}") from exc

    # ── Stage 2: IR compilation ──────────────────────────────────────────────
    try:
        from dartmouth_basic_ir_compiler import CompileError, compile_basic
        # The GE-225 is a 20-bit machine (max positive value 2^19-1 = 524,287).
        # Passing int_bits=20 ensures that the digit-extraction power constants
        # emitted by _emit_print_number never exceed the 20-bit register range.
        # Using the default int_bits=32 would emit LOAD_IMM 1_000_000_000,
        # which overflows a 20-bit word and corrupts the digit extraction.
        ir_result = compile_basic(ast, int_bits=20)
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 3: GE-225 backend ──────────────────────────────────────────────
    try:
        from ir_to_ge225_compiler import CodeGenError, compile_to_ge225
        ge225_result = compile_to_ge225(ir_result.program)
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 4: simulation ──────────────────────────────────────────────────
    try:
        from ge225_simulator import GE225Simulator
        sim = GE225Simulator(memory_words=memory_words)
        sim.load_program_bytes(ge225_result.binary)

        steps = 0
        while steps < max_steps:
            trace = sim.step()
            steps += 1
            if trace.address == ge225_result.halt_address:
                break
        else:
            raise BasicError(
                f"program did not halt within {max_steps} GE-225 instructions "
                f"(possible infinite loop)"
            )
    except BasicError:
        raise
    except Exception as exc:
        raise BasicError(f"runtime error: {exc}") from exc

    # ── Collect results ──────────────────────────────────────────────────────
    # Read final values of A–Z from their spill slots; sign-extend from 20 bits.
    var_values: dict[str, int] = {}
    for var_name, reg_idx in ir_result.var_regs.items():
        raw = sim.read_word(ge225_result.data_base + reg_idx)
        var_values[var_name] = raw - (1 << 20) if raw & (1 << 19) else raw

    # The GE-225 carriage-return code (0o37) maps to "\r" in the simulator;
    # convert to Unix newlines for convenient string comparison.
    output = sim.get_typewriter_output().replace("\r", "\n")

    return RunResult(
        output=output,
        var_values=var_values,
        steps=steps,
        halt_address=ge225_result.halt_address,
    )
