"""End-to-end compiler from Nib source to JVM class-file bytes."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_jvm_class_file import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
)
from ir_to_jvm_class_file import (
    write_class_file as backend_write_class_file,
)
from jvm_class_file import parse_class_file
from nib_ir_compiler import BuildConfig, compile_nib, release_config
from nib_parser import parse_nib
from nib_type_checker import check


@dataclass(frozen=True)
class PackageResult:
    source: str
    class_name: str
    ast: object
    typed_ast: object
    raw_ir: object
    optimization: OptimizationResult
    optimized_ir: object
    artifact: JVMClassArtifact
    parsed_class: object
    class_bytes: bytes
    class_file_path: Path | None = None


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


class NibJvmCompiler:
    """Compile Nib source into JVM class-file bytes."""

    def __init__(
        self,
        *,
        class_name: str = "NibProgram",
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
        emit_main_wrapper: bool = True,
    ) -> None:
        self.class_name = class_name
        self.build_config = build_config
        self.optimize_ir = optimize_ir
        self.emit_main_wrapper = emit_main_wrapper

    def compile_source(
        self,
        source: str,
        *,
        class_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        emit_main_wrapper: bool | None = None,
    ) -> PackageResult:
        resolved_class_name = class_name if class_name is not None else self.class_name
        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()

        use_optimizer = self.optimize_ir if optimize_ir is None else optimize_ir
        use_main_wrapper = (
            self.emit_main_wrapper
            if emit_main_wrapper is None
            else emit_main_wrapper
        )

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
            artifact = lower_ir_to_jvm_class_file(
                optimization.program,
                JvmBackendConfig(
                    class_name=resolved_class_name,
                    emit_main_wrapper=use_main_wrapper,
                ),
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("lower-jvm", str(exc), exc) from exc

        try:
            parsed_class = parse_class_file(artifact.class_bytes)
        except Exception as exc:  # pragma: no cover
            raise PackageError("validate-class", str(exc), exc) from exc

        return PackageResult(
            source=source,
            class_name=resolved_class_name,
            ast=ast,
            typed_ast=type_result.typed_ast,
            raw_ir=ir_result.program,
            optimization=optimization,
            optimized_ir=optimization.program,
            artifact=artifact,
            parsed_class=parsed_class,
            class_bytes=artifact.class_bytes,
        )

    def write_class_file(
        self,
        source: str,
        output_dir: str | Path,
        *,
        class_name: str | None = None,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
        emit_main_wrapper: bool | None = None,
    ) -> PackageResult:
        result = self.compile_source(
            source,
            class_name=class_name,
            build_config=build_config,
            optimize_ir=optimize_ir,
            emit_main_wrapper=emit_main_wrapper,
        )

        try:
            path = backend_write_class_file(result.artifact, output_dir)
        except Exception as exc:  # pragma: no cover
            raise PackageError("write", str(exc), exc) from exc

        return replace(result, class_file_path=path)


def compile_source(
    source: str,
    *,
    class_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    emit_main_wrapper: bool = True,
) -> PackageResult:
    return NibJvmCompiler(
        class_name=class_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        emit_main_wrapper=emit_main_wrapper,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    class_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    emit_main_wrapper: bool = True,
) -> PackageResult:
    return compile_source(
        source,
        class_name=class_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        emit_main_wrapper=emit_main_wrapper,
    )


def write_class_file(
    source: str,
    output_dir: str | Path,
    *,
    class_name: str = "NibProgram",
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
    emit_main_wrapper: bool = True,
) -> PackageResult:
    return NibJvmCompiler(
        class_name=class_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        emit_main_wrapper=emit_main_wrapper,
    ).write_class_file(
        source,
        output_dir,
        class_name=class_name,
        build_config=build_config,
        optimize_ir=optimize_ir,
        emit_main_wrapper=emit_main_wrapper,
    )
