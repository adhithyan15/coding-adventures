"""End-to-end compiler from Nib source to CLR assembly bytes."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from cli_assembly_writer import (
    CLIAssemblyArtifact,
    CLIAssemblyConfig,
    write_cli_assembly,
)
from clr_pe_file import CLRPEFile, decode_clr_pe_file
from clr_vm_simulator import CLRVMResult, run_clr_entry_point
from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILProgramArtifact,
    lower_ir_to_cil_bytecode,
)
from nib_ir_compiler import BuildConfig, compile_nib, release_config
from nib_parser import parse_nib
from nib_type_checker import check


@dataclass(frozen=True)
class PackageResult:
    source: str
    assembly_name: str
    type_name: str
    ast: object
    typed_ast: object
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


class NibClrCompiler:
    """Compile Nib source into a minimal PE/CLI assembly."""

    def __init__(
        self,
        *,
        assembly_name: str = "NibProgram",
        type_name: str = "NibProgram",
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
        cil_config: CILBackendConfig | None = None,
    ) -> None:
        self.assembly_name = assembly_name
        self.type_name = type_name
        self.build_config = build_config
        self.optimize_ir = optimize_ir
        self.cil_config = cil_config

    def compile_source(
        self,
        source: str,
        *,
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        resolved_assembly_name = (
            assembly_name if assembly_name is not None else self.assembly_name
        )
        resolved_type_name = type_name if type_name is not None else self.type_name
        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()
        resolved_cil_config = cil_config if cil_config is not None else self.cil_config
        if resolved_cil_config is None:
            resolved_cil_config = CILBackendConfig(call_register_count=None)
        use_optimizer = self.optimize_ir if optimize_ir is None else optimize_ir

        try:
            ast = parse_nib(source)
        except Exception as exc:  # pragma: no cover
            raise PackageError("parse", str(exc), exc) from exc

        type_result = check(ast)
        if not type_result.ok:
            diagnostics = "\n".join(
                f"Line {error.line}, Col {error.column}: {error.message}"
                for error in type_result.errors
            )
            raise PackageError("type-check", diagnostics)

        try:
            ir_result = compile_nib(type_result.typed_ast, config)
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
            assembly_name=resolved_assembly_name,
            type_name=resolved_type_name,
            ast=ast,
            typed_ast=type_result.typed_ast,
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
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> PackageResult:
        result = self.compile_source(
            source,
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
        max_steps: int = 100000,
        assembly_name: str | None = None,
        type_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        cil_config: CILBackendConfig | None = None,
    ) -> ExecutionResult:
        compilation = self.compile_source(
            source,
            assembly_name=assembly_name,
            type_name=type_name,
            build_config=build_config,
            optimize_ir=optimize_ir,
            cil_config=cil_config,
        )
        try:
            vm_result = run_clr_entry_point(
                compilation.assembly_bytes,
                max_steps=max_steps,
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("execute", str(exc), exc) from exc
        return ExecutionResult(compilation=compilation, vm_result=vm_result)


def compile_source(
    source: str,
    *,
    assembly_name: str = "NibProgram",
    type_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return NibClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    assembly_name: str = "NibProgram",
    type_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return compile_source(
        source,
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
    assembly_name: str = "NibProgram",
    type_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> PackageResult:
    return NibClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).write_assembly_file(source, output_path)


def run_source(
    source: str,
    *,
    max_steps: int = 100000,
    assembly_name: str = "NibProgram",
    type_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    cil_config: CILBackendConfig | None = None,
) -> ExecutionResult:
    return NibClrCompiler(
        assembly_name=assembly_name,
        type_name=type_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        cil_config=cil_config,
    ).run_source(source, max_steps=max_steps)
