"""End-to-end compiler from Nib source to Intel HEX."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from compiler_ir import IrProgram
from intel_4004_assembler import AssemblerError, assemble
from intel_4004_ir_validator import IrValidationError
from intel_4004_packager import encode_hex
from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_intel_4004_compiler import IrToIntel4004Compiler
from lang_parser import ASTNode
from nib_ir_compiler import BuildConfig, compile_nib, release_config
from nib_parser import parse_nib
from nib_type_checker import check


@dataclass(frozen=True)
class PackageResult:
    """All artifacts produced by a successful Nib compilation."""

    source: str
    ast: ASTNode
    typed_ast: ASTNode
    raw_ir: IrProgram
    optimization: OptimizationResult
    optimized_ir: IrProgram
    assembly: str
    binary: bytes
    hex_text: str
    origin: int
    hex_path: Path | None = None


class PackageError(Exception):
    """Raised when one stage of the Nib compiler pipeline fails."""

    def __init__(self, stage: str, message: str, cause: Exception | None = None) -> None:
        super().__init__(message)
        self.stage = stage
        self.message = message
        self.cause = cause

    def __str__(self) -> str:
        return f"[{self.stage}] {self.message}"


class NibCompiler:
    """Compile Nib source into Intel HEX."""

    def __init__(
        self,
        *,
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
    ) -> None:
        self.build_config = build_config
        self.optimize_ir = optimize_ir
        self.compiler = IrToIntel4004Compiler()

    def compile_source(
        self,
        source: str,
        *,
        origin: int = 0,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        """Compile Nib source into Intel HEX and return all intermediate artifacts."""

        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()

        use_optimizer = self.optimize_ir if optimize_ir is None else optimize_ir

        try:
            ast = parse_nib(source)
        except Exception as exc:  # pragma: no cover - parser exception type varies
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
        except Exception as exc:  # pragma: no cover - defensive wrapper
            raise PackageError("ir-compile", str(exc), exc) from exc

        optimizer = IrOptimizer.default_passes() if use_optimizer else IrOptimizer.no_op()
        optimization = optimizer.optimize(ir_result.program)

        try:
            assembly = self.compiler.compile(optimization.program)
        except IrValidationError as exc:
            raise PackageError("validate", str(exc), exc) from exc

        try:
            binary = assemble(assembly)
        except AssemblerError as exc:
            raise PackageError("assemble", str(exc), exc) from exc

        try:
            hex_text = encode_hex(binary, origin=origin)
        except ValueError as exc:
            raise PackageError("package", str(exc), exc) from exc

        return PackageResult(
            source=source,
            ast=ast,
            typed_ast=type_result.typed_ast,
            raw_ir=ir_result.program,
            optimization=optimization,
            optimized_ir=optimization.program,
            assembly=assembly,
            binary=binary,
            hex_text=hex_text,
            origin=origin,
        )

    def write_hex_file(
        self,
        source: str,
        output_path: str | Path,
        *,
        origin: int = 0,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        """Compile Nib source and write the Intel HEX text to disk."""

        result = self.compile_source(
            source,
            origin=origin,
            build_config=build_config,
            optimize_ir=optimize_ir,
        )

        path = Path(output_path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(result.hex_text, encoding="utf-8")
        except OSError as exc:
            raise PackageError("write", str(exc), exc) from exc

        return replace(result, hex_path=path)


def compile_source(
    source: str,
    *,
    origin: int = 0,
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    """Compile Nib source into Intel HEX."""

    return NibCompiler(
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).compile_source(source, origin=origin)


def pack_source(
    source: str,
    *,
    origin: int = 0,
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    """Alias for compile_source()."""

    return compile_source(
        source,
        origin=origin,
        build_config=build_config,
        optimize_ir=optimize_ir,
    )


def write_hex_file(
    source: str,
    output_path: str | Path,
    *,
    origin: int = 0,
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    """Compile Nib source and write the resulting Intel HEX file."""

    return NibCompiler(
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).write_hex_file(source, output_path, origin=origin)
