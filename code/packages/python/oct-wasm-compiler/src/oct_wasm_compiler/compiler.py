"""End-to-end compiler from Oct source to WebAssembly bytes.

Pipeline
--------

::

    Oct source text
        → oct-lexer / oct-parser       (characters → AST)
        → oct-type-checker             (untyped AST → typed AST)
        → oct-ir-compiler (WASM_IO)    (typed AST → IrProgram with WASM SYSCALLs)
        → ir-to-wasm-validator         (pre-flight check: WASM IR compatibility)
        → ir-to-wasm-assembly          (IrProgram → WASM text-format assembly)
        → wasm-assembler               (text → binary + structural validation)
        → wasm-validator               (binary → validated WASM module)

The ``WASM_IO`` config (``write_byte_syscall=1``, ``read_byte_syscall=2``)
tells the Oct IR compiler to emit ``SYSCALL 1`` for ``out()`` and
``SYSCALL 2`` for ``in()``, matching the WASI preview-1 ABI used by the
``ir-to-wasm-compiler`` backend:

    SYSCALL 1 → ``fd_write`` (write byte to stdout)
    SYSCALL 2 → ``fd_read``  (read byte from stdin)
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from compiler_ir import IrOp, IrProgram
from ir_to_wasm_assembly import emit_wasm_assembly
from ir_to_wasm_compiler import FunctionSignature
from ir_to_wasm_validator import validate as validate_ir_to_wasm
from oct_ir_compiler import WASM_IO, OctCompileResult, compile_oct
from oct_parser import parse_oct
from oct_type_checker import check_oct
from wasm_assembler import assemble, parse_assembly
from wasm_validator import ValidatedModule, ValidationError, validate


def _build_wasm_signatures(program: IrProgram) -> list[FunctionSignature]:
    """Return a ``FunctionSignature`` for every LABEL in *program*.

    Oct programs generate at least two function labels: ``_start`` (the
    entry stub that sets up the call frame) and ``_fn_main`` (the compiled
    body of ``fn main``).  Programs with additional named functions produce
    extra labels.

    The WASM IR validator and assembly emitter require a signature for every
    label that appears as a CALL target.  Only ``_start`` is exported so
    that the WASI runtime can invoke the program.

    Args:
        program: The compiled ``IrProgram`` whose label names to collect.

    Returns:
        A list of ``FunctionSignature`` objects, one per LABEL instruction.
    """
    sigs: list[FunctionSignature] = []
    for instr in program.instructions:
        if instr.opcode == IrOp.LABEL:
            name = instr.operands[0].name  # type: ignore[union-attr]
            export = name if name == "_start" else None
            sigs.append(
                FunctionSignature(label=name, param_count=0, export_name=export)
            )
    return sigs


@dataclass(frozen=True)
class PackageResult:
    """All artifacts produced by a successful Oct → WASM compilation.

    Attributes:
        source:          The original Oct source string.
        ast:             The untyped AST (from ``oct-parser``).
        ir:              The compiled ``IrProgram`` (from ``oct-ir-compiler``).
        wasm_assembly:   The WASM text-format assembly string.
        module:          The parsed (but not yet validated) WASM module object.
        validated_module: The validated WASM module.
        binary:          Raw ``.wasm`` binary bytes.
        wasm_path:       Where the binary was written (``write_wasm_file`` only).
    """

    source: str
    ast: object
    ir: object
    wasm_assembly: str
    module: object
    validated_module: ValidatedModule
    binary: bytes
    wasm_path: Path | None = None


class PackageError(Exception):
    """Raised when any pipeline stage fails.

    Attributes:
        stage:   Which pipeline stage failed (e.g. ``"parse"``, ``"type-check"``).
        message: Human-readable description of the failure.
        cause:   The underlying exception, if any.
    """

    def __init__(
        self,
        stage: str,
        message: str,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(message)
        self.stage = stage
        self.message = message
        self.cause = cause

    def __str__(self) -> str:
        return f"[{self.stage}] {self.message}"


class OctWasmCompiler:
    """Compile Oct source into WebAssembly bytes.

    The compiler runs the full Oct pipeline — lexing, parsing, type-checking,
    IR compilation with ``WASM_IO`` config, WASM text emission, and binary
    assembly — and returns the validated ``.wasm`` binary.

    Example::

        from oct_wasm_compiler import OctWasmCompiler

        result = OctWasmCompiler().compile_source("fn main() { }")
        assert result.binary[:4] == b"\\x00asm"  # WASM magic number
    """

    def __init__(self) -> None:
        pass  # No configuration needed for the V1 WASM pipeline.

    def compile_source(self, source: str) -> PackageResult:
        """Compile Oct *source* to a validated WASM binary.

        Args:
            source: Oct source text.

        Returns:
            A ``PackageResult`` with all compilation artifacts including the
            validated ``.wasm`` binary.

        Raises:
            PackageError: If any pipeline stage fails.
        """
        # ── Stage 1: Parse ───────────────────────────────────────────────────
        try:
            ast = parse_oct(source)
        except Exception as exc:
            raise PackageError("parse", str(exc), exc) from exc

        # ── Stage 2: Type-check ──────────────────────────────────────────────
        tc_result = check_oct(ast)
        if not tc_result.ok:
            msgs = "; ".join(str(e.message) for e in tc_result.errors)
            raise PackageError("type-check", msgs)

        # ── Stage 3: IR compile (WASM_IO config) ─────────────────────────────
        # WASM_IO sets write_byte_syscall=1 and read_byte_syscall=2, so Oct's
        # out(PORT, val) emits SYSCALL 1 (fd_write) and in(PORT) emits
        # SYSCALL 2 (fd_read), matching the WASI ABI.
        try:
            ir_result: OctCompileResult = compile_oct(
                tc_result.typed_ast, config=WASM_IO
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("ir-compile", str(exc), exc) from exc

        # ── Stage 4: Pre-flight WASM IR validation ───────────────────────────
        # Build signatures dynamically: every LABEL in the IR needs one so
        # that the validator can resolve all CALL targets.
        signatures = _build_wasm_signatures(ir_result.program)
        lowering_errors = validate_ir_to_wasm(ir_result.program, signatures)
        if lowering_errors:
            raise PackageError("validate-ir", lowering_errors[0].message)

        # ── Stage 5: Emit WASM text-format assembly ───────────────────────────
        try:
            wasm_assembly = emit_wasm_assembly(ir_result.program, signatures)
        except Exception as exc:  # pragma: no cover
            raise PackageError("assembly", str(exc), exc) from exc

        # ── Stage 6: Parse and validate the WASM module ───────────────────────
        try:
            module = parse_assembly(wasm_assembly)
            validated_module = validate(module)
        except ValidationError as exc:
            raise PackageError("validate-wasm", str(exc), exc) from exc

        # ── Stage 7: Assemble to binary ───────────────────────────────────────
        try:
            binary = assemble(wasm_assembly)
        except Exception as exc:  # pragma: no cover
            raise PackageError("assemble", str(exc), exc) from exc

        return PackageResult(
            source=source,
            ast=ast,
            ir=ir_result.program,
            wasm_assembly=wasm_assembly,
            module=module,
            validated_module=validated_module,
            binary=binary,
        )

    def write_wasm_file(
        self,
        source: str,
        output_path: str | Path,
    ) -> PackageResult:
        """Compile *source* and write the WASM binary to *output_path*.

        Args:
            source:      Oct source text.
            output_path: File path for the ``.wasm`` output.

        Returns:
            A ``PackageResult`` with ``wasm_path`` set to *output_path*.

        Raises:
            PackageError: If compilation or the file write fails.
        """
        result = self.compile_source(source)
        path = Path(output_path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(result.binary)
        except OSError as exc:
            raise PackageError("write", str(exc), exc) from exc
        return replace(result, wasm_path=path)


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------


def compile_source(source: str) -> PackageResult:
    """Compile Oct *source* to a validated WASM binary (stateless wrapper).

    Equivalent to ``OctWasmCompiler().compile_source(source)``.
    """
    return OctWasmCompiler().compile_source(source)


def pack_source(source: str) -> PackageResult:
    """Alias for :func:`compile_source` — compile Oct source to a WASM binary."""
    return compile_source(source)


def write_wasm_file(source: str, output_path: str | Path) -> PackageResult:
    """Compile Oct *source* and write the WASM binary to *output_path*."""
    return OctWasmCompiler().write_wasm_file(source, output_path)
