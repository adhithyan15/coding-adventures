"""End-to-end compiler from Brainfuck source to WebAssembly bytes."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from brainfuck import parse_brainfuck
from brainfuck_ir_compiler import BuildConfig, compile_brainfuck, release_config
from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_wasm_assembly import emit_wasm_assembly
from ir_to_wasm_compiler import FunctionSignature
from ir_to_wasm_validator import validate as validate_ir_to_wasm
from wasm_assembler import assemble, parse_assembly
from wasm_validator import ValidatedModule, ValidationError, validate


@dataclass(frozen=True)
class PackageResult:
    source: str
    filename: str
    ast: object
    raw_ir: object
    optimization: OptimizationResult
    optimized_ir: object
    wasm_assembly: str
    module: object
    validated_module: ValidatedModule
    binary: bytes
    wasm_path: Path | None = None


class PackageError(Exception):
    def __init__(self, stage: str, message: str, cause: Exception | None = None) -> None:
        super().__init__(message)
        self.stage = stage
        self.message = message
        self.cause = cause

    def __str__(self) -> str:
        return f"[{self.stage}] {self.message}"


class BrainfuckWasmCompiler:
    """Compile Brainfuck source into WebAssembly bytes."""

    def __init__(
        self,
        *,
        filename: str = "program.bf",
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
    ) -> None:
        self.filename = filename
        self.build_config = build_config
        self.optimize_ir = optimize_ir

    def compile_source(
        self,
        source: str,
        *,
        filename: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        resolved_filename = filename if filename is not None else self.filename
        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()

        use_optimizer = self.optimize_ir if optimize_ir is None else optimize_ir

        try:
            ast = parse_brainfuck(source)
        except Exception as exc:  # pragma: no cover
            raise PackageError("parse", str(exc), exc) from exc

        try:
            ir_result = compile_brainfuck(ast, resolved_filename, config)
        except Exception as exc:  # pragma: no cover
            raise PackageError("ir-compile", str(exc), exc) from exc

        optimizer = IrOptimizer.default_passes() if use_optimizer else IrOptimizer.no_op()
        optimization = optimizer.optimize(ir_result.program)

        signatures = [FunctionSignature(label="_start", param_count=0, export_name="_start")]
        lowering_errors = validate_ir_to_wasm(optimization.program, signatures)
        if lowering_errors:
            raise PackageError("validate-ir", lowering_errors[0].message)

        try:
            wasm_assembly = emit_wasm_assembly(optimization.program, signatures)
        except Exception as exc:  # pragma: no cover
            raise PackageError("assembly", str(exc), exc) from exc

        try:
            module = parse_assembly(wasm_assembly)
            validated_module = validate(module)
        except ValidationError as exc:
            raise PackageError("validate-wasm", str(exc), exc) from exc

        try:
            binary = assemble(wasm_assembly)
        except Exception as exc:  # pragma: no cover
            raise PackageError("assemble", str(exc), exc) from exc

        return PackageResult(
            source=source,
            filename=resolved_filename,
            ast=ast,
            raw_ir=ir_result.program,
            optimization=optimization,
            optimized_ir=optimization.program,
            wasm_assembly=wasm_assembly,
            module=module,
            validated_module=validated_module,
            binary=binary,
        )

    def write_wasm_file(
        self,
        source: str,
        output_path: str | Path,
        *,
        filename: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        result = self.compile_source(
            source,
            filename=filename,
            build_config=build_config,
            optimize_ir=optimize_ir,
        )

        path = Path(output_path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(result.binary)
        except OSError as exc:
            raise PackageError("write", str(exc), exc) from exc

        return replace(result, wasm_path=path)


def compile_source(
    source: str,
    *,
    filename: str = "program.bf",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return BrainfuckWasmCompiler(
        filename=filename,
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).compile_source(source, filename=filename)


def pack_source(
    source: str,
    *,
    filename: str = "program.bf",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return compile_source(
        source,
        filename=filename,
        build_config=build_config,
        optimize_ir=optimize_ir,
    )


def write_wasm_file(
    source: str,
    output_path: str | Path,
    *,
    filename: str = "program.bf",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return BrainfuckWasmCompiler(
        filename=filename,
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).write_wasm_file(
        source,
        output_path,
        filename=filename,
        build_config=build_config,
        optimize_ir=optimize_ir,
    )
