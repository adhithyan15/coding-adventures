"""End-to-end compiler from Oct source to CLR assembly bytes.

Pipeline
--------

::

    Oct source text
        → oct-lexer / oct-parser      (characters → AST)
        → oct-type-checker            (untyped AST → typed AST)
        → oct-ir-compiler (CLR_IO)    (typed AST → IrProgram with CLR SYSCALLs)
        → ir-to-cil-bytecode          (IrProgram → CIL bytecode + metadata)
        → cli-assembly-writer         (CIL → PE/CLI binary)
        → clr-vm-simulator            (optional: execute the PE file)

The ``CLR_IO`` config (``write_byte_syscall=1``, ``read_byte_syscall=2``)
tells the Oct IR compiler to emit ``SYSCALL 1`` for ``out()`` and
``SYSCALL 2`` for ``in()``, matching the CLR host's ABI instead of the
Intel 8008 port-based encoding.

Oct programs compiled here are equivalent to brainfuck-clr-compiler
programs in structure — both lower through the same IR → CIL → PE pipeline.
The difference is the front end: Oct is a typed, structured language whereas
Brainfuck is unstructured.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from cli_assembly_writer import (
    CLIAssemblyArtifact,
    CLIAssemblyConfig,
    write_cli_assembly,
)
from clr_pe_file import CLRPEFile, decode_clr_pe_file
from clr_vm_simulator import CLRVMResult, CLRVMStdlibHost, run_clr_entry_point
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILProgramArtifact,
    lower_ir_to_cil_bytecode,
)
from oct_ir_compiler import CLR_IO, OctCompileResult, compile_oct
from oct_parser import parse_oct
from oct_type_checker import check_oct


@dataclass(frozen=True)
class PackageResult:
    """All artifacts produced by a successful Oct → CLR compilation.

    Attributes:
        source:            The original Oct source string.
        assembly_name:     The CLI assembly / module name.
        type_name:         The CLI type that owns the ``_start`` method.
        ast:               The untyped AST (from ``oct-parser``).
        ir:                The compiled ``IrProgram`` (from ``oct-ir-compiler``).
        cil_artifact:      CIL bytecode + metadata table (from ``ir-to-cil-bytecode``).
        assembly_artifact: The raw PE/CLI artifact (from ``cli-assembly-writer``).
        decoded_assembly:  Decoded and validated ``CLRPEFile`` object.
        assembly_bytes:    Raw bytes of the ``.dll`` / ``.exe`` file.
        assembly_path:     Where the assembly was written
                           (``write_assembly_file`` only).
    """

    source: str
    assembly_name: str
    type_name: str
    ast: object
    ir: object
    cil_artifact: CILProgramArtifact
    assembly_artifact: CLIAssemblyArtifact
    decoded_assembly: CLRPEFile
    assembly_bytes: bytes
    assembly_path: Path | None = None


@dataclass(frozen=True)
class ExecutionResult:
    """The result of compiling *and* running an Oct program on the CLR VM.

    Attributes:
        compilation: The full ``PackageResult`` from compilation.
        vm_result:   The ``CLRVMResult`` from the simulator.
    """

    compilation: PackageResult
    vm_result: CLRVMResult


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


class OctClrCompiler:
    """Compile Oct source into a minimal PE/CLI assembly.

    The compiler runs the full Oct pipeline — lexing, parsing, type-checking,
    IR compilation with ``CLR_IO`` config, CIL lowering, and CLI assembly
    packing — and optionally executes the result on the CLR VM simulator.

    Example::

        from oct_clr_compiler import OctClrCompiler

        compiler = OctClrCompiler(assembly_name="MyProgram", type_name="MyProgram")
        result = compiler.run_source("fn main() { }")
        assert result.vm_result.output == ""
    """

    def __init__(
        self,
        *,
        assembly_name: str = "OctProgram",
        type_name: str = "OctProgram",
        cil_config: CILBackendConfig | None = None,
    ) -> None:
        self.assembly_name = assembly_name
        self.type_name = type_name
        # Default syscall_arg_reg=4 matches the register the Oct IR compiler
        # puts the output byte into for the CLR target (v2 for out(), v1 for
        # in(); the CIL backend needs to know which register to inspect).
        self.cil_config = cil_config

    def compile_source(
        self,
        source: str,
        *,
        assembly_name: str | None = None,
        type_name: str | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        """Compile Oct *source* to a PE/CLI assembly artifact.

        Args:
            source:        Oct source text.
            assembly_name: Override the assembly name for this call.
            type_name:     Override the CLI type name for this call.
            cil_config:    Override the CIL backend config for this call.

        Returns:
            A ``PackageResult`` with all compilation artifacts.

        Raises:
            PackageError: If any pipeline stage fails.
        """
        resolved_assembly_name = (
            assembly_name if assembly_name is not None else self.assembly_name
        )
        resolved_type_name = type_name if type_name is not None else self.type_name
        resolved_cil_config = cil_config if cil_config is not None else self.cil_config
        if resolved_cil_config is None:
            # The Oct IR compiler stages the byte-to-write in v2 (register index 2)
            # for out() under CLR_IO.  The CIL backend needs to know which virtual
            # register holds the syscall argument so it can load it before invoking
            # the host helper.
            resolved_cil_config = CILBackendConfig(syscall_arg_reg=2)

        # ── Stage 1: Parse ───────────────────────────────────────────────────
        try:
            ast = parse_oct(source)
        except Exception as exc:
            raise PackageError("parse", str(exc), exc) from exc

        # ── Stage 2: Type-check ──────────────────────────────────────────────
        tc_result = check_oct(ast)
        if not tc_result.ok:
            # Collect all error messages into a single summary.
            msgs = "; ".join(str(e.message) for e in tc_result.errors)
            raise PackageError("type-check", msgs)

        # ── Stage 3: IR compile (CLR_IO config) ──────────────────────────────
        # CLR_IO sets write_byte_syscall=1 and read_byte_syscall=2, so Oct's
        # out(PORT, val) emits SYSCALL 1 and in(PORT) emits SYSCALL 2 — the
        # numbers wired into the CLR VM host instead of the 8008 port encoding.
        try:
            ir_result: OctCompileResult = compile_oct(
                tc_result.typed_ast, config=CLR_IO
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("ir-compile", str(exc), exc) from exc

        # ── Stage 4: Lower to CIL bytecode ───────────────────────────────────
        try:
            cil_artifact = lower_ir_to_cil_bytecode(
                ir_result.program, resolved_cil_config
            )
        except Exception as exc:
            raise PackageError("lower-cil", str(exc), exc) from exc

        # ── Stage 5: Pack to PE/CLI assembly ─────────────────────────────────
        try:
            assembly_artifact = write_cli_assembly(
                cil_artifact,
                CLIAssemblyConfig(
                    assembly_name=resolved_assembly_name,
                    module_name=f"{resolved_assembly_name}.dll",
                    type_name=resolved_type_name,
                ),
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("write-cli", str(exc), exc) from exc

        # ── Stage 6: Decode and validate the PE file ─────────────────────────
        try:
            decoded_assembly = decode_clr_pe_file(assembly_artifact.assembly_bytes)
        except Exception as exc:  # pragma: no cover
            raise PackageError("validate-cli", str(exc), exc) from exc

        return PackageResult(
            source=source,
            assembly_name=resolved_assembly_name,
            type_name=resolved_type_name,
            ast=ast,
            ir=ir_result.program,
            cil_artifact=cil_artifact,
            assembly_artifact=assembly_artifact,
            decoded_assembly=decoded_assembly,
            assembly_bytes=assembly_artifact.assembly_bytes,
        )

    def write_assembly_file(
        self,
        source: str,
        output_path: str | Path,
        *,
        assembly_name: str | None = None,
        type_name: str | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        """Compile *source* and write the PE/CLI assembly to *output_path*.

        Args:
            source:      Oct source text.
            output_path: File path for the ``.dll`` / ``.exe`` output.

        Returns:
            A ``PackageResult`` with ``assembly_path`` set to *output_path*.

        Raises:
            PackageError: If compilation or the file write fails.
        """
        result = self.compile_source(
            source,
            assembly_name=assembly_name,
            type_name=type_name,
            cil_config=cil_config,
        )
        path = Path(output_path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(result.assembly_bytes)
        except OSError as exc:
            raise PackageError("write", str(exc), exc) from exc
        return replace(result, assembly_path=path)

    def run_source(
        self,
        source: str,
        *,
        input_bytes: bytes = b"",
        memory_size: int = 65536,
        max_steps: int = 100_000,
        assembly_name: str | None = None,
        type_name: str | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> ExecutionResult:
        """Compile *source* and execute it on the CLR VM simulator.

        Args:
            source:      Oct source text.
            input_bytes: Bytes fed to the program's ``in()`` calls.
            memory_size: CLR VM heap size in bytes.
            max_steps:   Maximum CLR VM execution steps (guards against infinite loops).

        Returns:
            An ``ExecutionResult`` with both compilation artifacts and the VM result.

        Raises:
            PackageError: If compilation or execution fails.
        """
        compilation = self.compile_source(
            source,
            assembly_name=assembly_name,
            type_name=type_name,
            cil_config=cil_config,
        )
        host = CLRVMStdlibHost(memory_size=memory_size, input_bytes=input_bytes)
        try:
            vm_result = run_clr_entry_point(
                compilation.assembly_bytes,
                host=host,
                max_steps=max_steps,
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("execute", str(exc), exc) from exc
        return ExecutionResult(compilation=compilation, vm_result=vm_result)


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------
# These mirror the class API so callers can use the package without
# instantiating OctClrCompiler explicitly.


def compile_source(
    source: str,
    *,
    assembly_name: str = "OctProgram",
    type_name: str = "OctProgram",
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    """Compile Oct *source* to a PE/CLI assembly (stateless convenience wrapper).

    Equivalent to ``OctClrCompiler(...).compile_source(source)``.
    """
    return OctClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        cil_config=cil_config,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    assembly_name: str = "OctProgram",
    type_name: str = "OctProgram",
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    """Alias for :func:`compile_source` — compile Oct source to a PE/CLI assembly."""
    return compile_source(
        source,
        assembly_name=assembly_name,
        type_name=type_name,
        cil_config=cil_config,
    )


def write_assembly_file(
    source: str,
    output_path: str | Path,
    *,
    assembly_name: str = "OctProgram",
    type_name: str = "OctProgram",
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    """Compile Oct *source* and write the PE/CLI assembly to *output_path*."""
    return OctClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        cil_config=cil_config,
    ).write_assembly_file(source, output_path)


def run_source(
    source: str,
    *,
    input_bytes: bytes = b"",
    memory_size: int = 65536,
    max_steps: int = 100_000,
    assembly_name: str = "OctProgram",
    type_name: str = "OctProgram",
    cil_config: CILBackendConfig | None = None,
) -> ExecutionResult:
    """Compile Oct *source* and execute it on the CLR VM simulator."""
    return OctClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        cil_config=cil_config,
    ).run_source(
        source,
        input_bytes=input_bytes,
        memory_size=memory_size,
        max_steps=max_steps,
    )
