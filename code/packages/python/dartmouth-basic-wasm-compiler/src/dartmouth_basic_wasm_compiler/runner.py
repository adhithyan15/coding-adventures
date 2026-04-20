"""Dartmouth BASIC → WebAssembly pipeline runner.

This module is the public interface for the full compiled WASM pipeline.  It
chains four independent packages — parser, IR compiler, WASM backend, and WASM
runtime — into one ``run_basic()`` call that accepts a BASIC source string and
returns the program's standard output.

Pipeline
--------

1. **Lex & parse** (``dartmouth_basic_parser``) — tokenise the BASIC source
   and build an AST.
2. **IR compile** (``dartmouth_basic_ir_compiler``) — lower the AST to a
   target-independent ``IrProgram``.  The ``char_encoding="ascii"`` flag makes
   all ``PRINT`` char codes use standard ASCII byte values so the WASM
   ``fd_write`` syscall produces correct output.
3. **WASM backend** (``ir_to_wasm_compiler``) — lower the ``IrProgram`` to a
   ``WasmModule`` targeting the WASI preview-1 ABI.
4. **Encode** (``wasm_module_encoder``) — serialise the ``WasmModule`` to raw
   WebAssembly 1.0 bytes.
5. **Run** (``wasm_runtime``) — instantiate the binary, bind a WASI host that
   captures stdout, and call the ``_start`` export.

Character encoding note
-----------------------

The GE-225 typewriter used a proprietary 6-bit character code.  The WASM
backend writes raw bytes to stdout via the WASI ``fd_write`` syscall, which
expects standard ASCII.  The IR compiler therefore runs in ``char_encoding=
"ascii"`` mode: string literals are emitted as their ``ord()`` values and
numeric digits are offset by 48 (``ord('0')``) before printing.

Variable values
---------------

Unlike the GE-225 backend (which stores all registers as memory words that
can be read after halting), the WASM backend maps each IR virtual register to
a WASM local variable.  WASM locals are stack-allocated and disappear when the
``_start`` function returns, so ``RunResult.var_values`` is always empty for
the WASM backend.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class BasicError(Exception):
    """Raised when the BASIC program cannot be compiled or executed.

    Wraps ``CompileError`` from the IR compiler, ``WasmLoweringError`` from
    the WASM backend, and runtime errors from the WASM runtime.  The original
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
        var_values:  Always ``{}`` for the WASM backend.  WASM locals vanish
                     when ``_start`` returns, so variable state is not
                     recoverable after execution.
        steps:       Always ``0`` for the WASM backend (no instruction
                     counter in the WASM runtime).
        halt_address: Always ``0`` for the WASM backend (halt is a function
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
    """Compile and run a Dartmouth BASIC program via WebAssembly.

    This is the single public entry point.  It chains all five pipeline stages
    and returns the program's standard output.

    Args:
        source:    Dartmouth BASIC source text.  Lines must begin with a line
                   number followed by a statement.  Newlines between lines are
                   required.
        max_steps: Accepted for interface parity with the GE-225 runner but
                   not enforced; the WASM runtime has no step counter.

    Returns:
        A ``RunResult`` with the standard output.  ``var_values``, ``steps``,
        and ``halt_address`` are always zero/empty for the WASM backend.

    Raises:
        BasicError: If the program cannot be parsed, uses a V1-unsupported
                    feature (GOSUB, DIM, INPUT, arrays, ``^`` operator), the
                    string contains a character with no GE-225 typewriter code,
                    or the WASM runtime raises an unexpected error.

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

    # ── Stage 2: IR compilation (ASCII char encoding for WASM) ───────────────
    try:
        from dartmouth_basic_ir_compiler import compile_basic
        ir_result = compile_basic(ast, char_encoding="ascii")
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 3: WASM backend ────────────────────────────────────────────────
    try:
        from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler, WasmLoweringError
        wasm_module = IrToWasmCompiler().compile(
            ir_result.program,
            function_signatures=[
                FunctionSignature(label="_start", param_count=0, export_name="_start")
            ],
            strategy="dispatch_loop",
        )
    except Exception as exc:
        raise BasicError(str(exc)) from exc

    # ── Stage 4: encode to bytes ─────────────────────────────────────────────
    try:
        from wasm_module_encoder import encode_module
        wasm_bytes = encode_module(wasm_module)
    except Exception as exc:
        raise BasicError(f"WASM encode error: {exc}") from exc

    # ── Stage 5: run ─────────────────────────────────────────────────────────
    try:
        from wasm_runtime import WasiConfig, WasiHost, WasmRuntime
        output_chunks: list[str] = []
        config = WasiConfig(stdout=output_chunks.append)
        host = WasiHost(config=config)
        runtime = WasmRuntime(host=host)
        runtime.load_and_run(wasm_bytes, "_start", [])
    except BasicError:
        raise
    except Exception as exc:
        raise BasicError(f"runtime error: {exc}") from exc

    return RunResult(output="".join(output_chunks))
