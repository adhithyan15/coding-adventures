"""End-to-end compiler from Brainfuck source to CLR assembly bytes."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from brainfuck import parse_brainfuck
from brainfuck_ir_compiler import BuildConfig, compile_brainfuck, release_config
from cli_assembly_writer import (
    CLIAssemblyArtifact,
    CLIAssemblyConfig,
    write_cli_assembly,
)
from clr_pe_file import CLRPEFile, decode_clr_pe_file
from clr_vm_simulator import CLRVMResult, CLRVMStdlibHost, run_clr_entry_point
from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILProgramArtifact,
    lower_ir_to_cil_bytecode,
)


@dataclass(frozen=True)
class PackageResult:
    source: str
    filename: str
    assembly_name: str
    type_name: str
    ast: object
    raw_ir: object
    optimization: OptimizationResult
    optimized_ir: object
    cil_artifact: CILProgramArtifact
    assembly_artifact: CLIAssemblyArtifact
    decoded_assembly: CLRPEFile
    assembly_bytes: bytes
    assembly_path: Path | None = None


@dataclass(frozen=True)
class ExecutionResult:
    compilation: PackageResult
    vm_result: CLRVMResult


class PackageError(Exception):
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


class BrainfuckClrCompiler:
    """Compile Brainfuck source into a minimal PE/CLI assembly."""

    def __init__(
        self,
        *,
        filename: str = "program.bf",
        assembly_name: str = "BrainfuckProgram",
        type_name: str = "BrainfuckProgram",
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
        cil_config: CILBackendConfig | None = None,
    ) -> None:
        self.filename = filename
        self.assembly_name = assembly_name
        self.type_name = type_name
        self.build_config = build_config
        self.optimize_ir = optimize_ir
        self.cil_config = cil_config

    def compile_source(
        self,
        source: str,
        *,
        filename: str | None = None,
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        resolved_filename = filename if filename is not None else self.filename
        resolved_assembly_name = (
            assembly_name if assembly_name is not None else self.assembly_name
        )
        resolved_type_name = type_name if type_name is not None else self.type_name
        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()
        resolved_cil_config = cil_config if cil_config is not None else self.cil_config
        if resolved_cil_config is None:
            resolved_cil_config = CILBackendConfig(syscall_arg_reg=4)
        use_optimizer = self.optimize_ir if optimize_ir is None else optimize_ir

        try:
            ast = parse_brainfuck(source)
        except Exception as exc:  # pragma: no cover
            raise PackageError("parse", str(exc), exc) from exc

        try:
            ir_result = compile_brainfuck(ast, resolved_filename, config)
        except Exception as exc:  # pragma: no cover
            raise PackageError("ir-compile", str(exc), exc) from exc

        optimizer = (
            IrOptimizer.default_passes()
            if use_optimizer
            else IrOptimizer.no_op()
        )
        optimization = optimizer.optimize(ir_result.program)

        try:
            cil_artifact = lower_ir_to_cil_bytecode(
                optimization.program,
                resolved_cil_config,
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("lower-cil", str(exc), exc) from exc

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

        try:
            decoded_assembly = decode_clr_pe_file(assembly_artifact.assembly_bytes)
        except Exception as exc:  # pragma: no cover
            raise PackageError("validate-cli", str(exc), exc) from exc

        return PackageResult(
            source=source,
            filename=resolved_filename,
            assembly_name=resolved_assembly_name,
            type_name=resolved_type_name,
            ast=ast,
            raw_ir=ir_result.program,
            optimization=optimization,
            optimized_ir=optimization.program,
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
        filename: str | None = None,
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        result = self.compile_source(
            source,
            filename=filename,
            assembly_name=assembly_name,
            type_name=type_name,
            build_config=build_config,
            optimize_ir=optimize_ir,
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
        max_steps: int = 100000,
        filename: str | None = None,
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> ExecutionResult:
        compilation = self.compile_source(
            source,
            filename=filename,
            assembly_name=assembly_name,
            type_name=type_name,
            build_config=build_config,
            optimize_ir=optimize_ir,
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


def compile_source(
    source: str,
    *,
    filename: str = "program.bf",
    assembly_name: str = "BrainfuckProgram",
    type_name: str = "BrainfuckProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return BrainfuckClrCompiler(
        filename=filename,
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    filename: str = "program.bf",
    assembly_name: str = "BrainfuckProgram",
    type_name: str = "BrainfuckProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return compile_source(
        source,
        filename=filename,
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    )


def write_assembly_file(
    source: str,
    output_path: str | Path,
    *,
    filename: str = "program.bf",
    assembly_name: str = "BrainfuckProgram",
    type_name: str = "BrainfuckProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return BrainfuckClrCompiler(
        filename=filename,
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).write_assembly_file(source, output_path)


def run_source(
    source: str,
    *,
    input_bytes: bytes = b"",
    memory_size: int = 65536,
    max_steps: int = 100000,
    filename: str = "program.bf",
    assembly_name: str = "BrainfuckProgram",
    type_name: str = "BrainfuckProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> ExecutionResult:
    return BrainfuckClrCompiler(
        filename=filename,
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).run_source(
        source,
        input_bytes=input_bytes,
        memory_size=memory_size,
        max_steps=max_steps,
    )
