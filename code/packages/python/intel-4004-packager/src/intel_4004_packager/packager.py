"""Intel 4004 ROM Packager — orchestrates the full Nib → Intel HEX pipeline.

=== Pipeline Overview ===

This module is the last stage of the Nib compiler toolchain:

  Nib source text
      ↓  nib-parser       (text → untyped AST)
      ↓  nib-type-checker (untyped AST → typed AST)
      ↓  nib-ir-compiler  (typed AST → IrProgram)
      ↓  ir-optimizer     (IrProgram → optimized IrProgram)
      ↓  intel-4004-backend (IrProgram → assembly text)
      ↓  intel-4004-assembler (assembly text → binary bytes)
      ↓  intel-4004-packager  (binary bytes → Intel HEX)  ← this module

``Intel4004Packager.pack_source(source)`` runs every stage in sequence and
returns a ``PackageResult`` containing the Intel HEX string alongside
intermediate artifacts for debugging and testing.

=== Why Keep Intermediate Artifacts? ===

Every compiler stage introduces the possibility of bugs.  When a test fails
you want to know *which* stage went wrong — did the IR compiler emit the
wrong register?  Did the backend mishandle a branch?  Did the assembler
compute the wrong address in pass 1?

``PackageResult`` keeps all of this: the typed AST, the raw ``IrProgram``,
the optimized ``IrProgram``, the assembly text, the binary bytes, and the
final Intel HEX.  A failing test can ``print(result.asm_text)`` and see
exactly what the backend emitted, without re-running individual stages.

=== Error Handling Strategy ===

Each stage can fail in a different way:

  - Parser errors → raise from nib-parser (syntax)
  - Type errors   → ``TypeCheckResult.errors`` (semantics)
  - IR errors     → raise from nib-ir-compiler (should not happen after TC)
  - Validation    → ``IrValidationError`` from intel-4004-backend (ISA limits)
  - Assembler     → ``AssemblerError`` from intel-4004-assembler (encoding)

``PackageError`` wraps all of these into a single exception type so callers
only need one except clause.  The ``stage`` field tells you which stage
failed so you can diagnose the root cause.
"""

from __future__ import annotations

from dataclasses import dataclass

from compiler_ir import IrProgram
from intel_4004_assembler import Intel4004Assembler
from intel_4004_backend import Intel4004Backend
from ir_optimizer import IrOptimizer
from lang_parser import ASTNode
from nib_ir_compiler import CompileResult, compile_nib
from nib_parser import parse_nib
from nib_type_checker import NibTypeChecker

from intel_4004_packager.hex_encoder import encode_hex


@dataclass(frozen=True)
class PackageResult:
    """All artifacts produced by the Nib → Intel HEX pipeline.

    Every field is populated, even for intermediate stages.  This makes
    integration tests easy to write: assert on ``hex_text`` for the
    end-to-end result, or drill into ``asm_text`` or ``ir_program`` to
    pin down a specific stage.

    Attributes
    ----------
    typed_ast:
        The typed AST produced by the type checker.  Useful for verifying
        that the type checker annotated everything correctly.
    raw_ir:
        The ``IrProgram`` produced by the Nib IR compiler before any
        optimization.  Compare with ``optimized_ir`` to see what the
        optimizer removed.
    optimized_ir:
        The ``IrProgram`` after all optimizer passes.  This is what the
        backend receives.
    asm_text:
        The Intel 4004 assembly text emitted by the backend code generator.
        Human-readable; useful for debugging codegen bugs.
    binary:
        The raw machine code bytes, output of the two-pass assembler.
        Feed this directly to ``Intel4004Simulator.execute(binary)``.
    hex_text:
        The Intel HEX string ready to be written to a ``.hex`` file and
        burned to an EPROM.
    """

    typed_ast: ASTNode
    raw_ir: IrProgram
    optimized_ir: IrProgram
    asm_text: str
    binary: bytes
    hex_text: str


class PackageError(Exception):
    """Raised when any stage of the Nib → Intel HEX pipeline fails.

    A regular Exception subclass (not a frozen dataclass) so that Python's
    exception machinery can set ``__traceback__`` and ``__cause__`` freely.

    Attributes
    ----------
    stage:
        Which pipeline stage raised the error (``"parse"``, ``"typecheck"``,
        ``"ir_compile"``, ``"optimize"``, ``"backend"``, ``"assemble"``,
        ``"pack"``).
    message:
        Human-readable description of the failure.
    cause:
        The original exception, if any.
    """

    def __init__(
        self,
        stage: str,
        message: str,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(f"[{stage}] {message}" + (f": {cause}" if cause else ""))
        self.stage = stage
        self.message = message
        self.cause = cause


class Intel4004Packager:
    """Orchestrate the full Nib → Intel HEX compiler pipeline.

    This class wires together all seven stages into a single
    ``pack_source()`` call.  Each stage is independently replaceable —
    the packager holds references to each stage as instance attributes,
    so tests can swap in a mock backend or disable the optimizer.

    Usage
    -----
    ::

        packager = Intel4004Packager()
        result = packager.pack_source('''
            fn main() -> u4 {
                let x: u4 = 5
                return x
            }
        ''')
        print(result.hex_text)
        # :...
        # :00000001FF

    Parameters
    ----------
    optimize:
        If ``True`` (default), run the IR optimizer between the IR compiler
        and the backend.  Set to ``False`` to skip optimization (useful
        when debugging the IR compiler output directly).
    origin:
        ROM load address for the Intel HEX output (default 0x000).
    """

    def __init__(
        self,
        *,
        optimize: bool = True,
        origin: int = 0x000,
    ) -> None:
        self._optimize = optimize
        self._origin = origin
        self._type_checker = NibTypeChecker()
        self._optimizer = IrOptimizer.default_passes() if optimize else IrOptimizer.no_op()
        self._backend = Intel4004Backend()
        self._assembler = Intel4004Assembler()

    def pack_source(self, source: str) -> PackageResult:
        """Run the complete pipeline: Nib source → Intel HEX.

        Parameters
        ----------
        source:
            A complete Nib source program as a string.

        Returns
        -------
        PackageResult:
            All pipeline artifacts, including the final Intel HEX string.

        Raises
        ------
        PackageError:
            On failure at any pipeline stage.  The ``stage`` attribute
            indicates which stage failed.
        """
        # Stage 1 — parse
        try:
            untyped_ast = parse_nib(source)
        except Exception as exc:
            raise PackageError("parse", "failed to parse Nib source", exc) from exc

        # Stage 2 — type check
        try:
            tc_result = self._type_checker.check(untyped_ast)
        except Exception as exc:
            raise PackageError("typecheck", "type checker raised an exception", exc) from exc

        if not tc_result.ok:
            msgs = "; ".join(str(e) for e in tc_result.errors)
            raise PackageError("typecheck", f"type errors: {msgs}")

        typed_ast = tc_result.typed_ast

        # Stage 3 — IR compile
        try:
            compile_result: CompileResult = compile_nib(typed_ast)
            raw_ir = compile_result.program
        except Exception as exc:
            raise PackageError("ir_compile", "IR compiler raised an exception", exc) from exc

        # Stage 4 — optimize
        try:
            opt_result = self._optimizer.optimize(raw_ir)
            optimized_ir = opt_result.program
        except Exception as exc:
            raise PackageError("optimize", "IR optimizer raised an exception", exc) from exc

        # Stage 5 — backend (validate + codegen)
        try:
            asm_text = self._backend.compile(optimized_ir)
        except Exception as exc:
            raise PackageError("backend", "backend raised an exception", exc) from exc

        # Stage 6 — assemble
        try:
            binary = self._assembler.assemble(asm_text)
        except Exception as exc:
            raise PackageError("assemble", "assembler raised an exception", exc) from exc

        # Stage 7 — package to Intel HEX
        try:
            hex_text = encode_hex(binary, origin=self._origin)
        except Exception as exc:
            raise PackageError("pack", "Intel HEX encoder raised an exception", exc) from exc

        return PackageResult(
            typed_ast=typed_ast,
            raw_ir=raw_ir,
            optimized_ir=optimized_ir,
            asm_text=asm_text,
            binary=binary,
            hex_text=hex_text,
        )
