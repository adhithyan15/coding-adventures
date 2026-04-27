"""End-to-end compiler from Nib source to WebAssembly bytes."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from ir_optimizer import IrOptimizer, OptimizationResult
from ir_to_wasm_assembly import emit_wasm_assembly
from ir_to_wasm_compiler import FunctionSignature
from ir_to_wasm_validator import validate as validate_ir_to_wasm
from nib_ir_compiler import BuildConfig, compile_nib, release_config
from nib_parser import parse_nib
from nib_type_checker import check
from wasm_assembler import assemble, parse_assembly
from wasm_validator import ValidatedModule, ValidationError, validate


@dataclass(frozen=True)
class PackageResult:
    source: str
    ast: object
    typed_ast: object
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


class NibWasmCompiler:
    """Compile Nib source into WebAssembly bytes."""

    def __init__(
        self,
        *,
        build_config: BuildConfig | None = None,
        optimize_ir: bool = True,
    ) -> None:
        self.build_config = build_config
        self.optimize_ir = optimize_ir

    def compile_source(
        self,
        source: str,
        *,
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        config = build_config if build_config is not None else self.build_config
        if config is None:
            config = release_config()

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

        optimizer = IrOptimizer.default_passes() if use_optimizer else IrOptimizer.no_op()
        optimization = optimizer.optimize(ir_result.program)

        signatures = _extract_signatures(type_result.typed_ast)
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
            ast=ast,
            typed_ast=type_result.typed_ast,
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
        build_config: BuildConfig | None = None,
        optimize_ir: bool | None = None,
    ) -> PackageResult:
        result = self.compile_source(
            source,
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
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return NibWasmCompiler(
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return compile_source(
        source,
        build_config=build_config,
        optimize_ir=optimize_ir,
    )


def write_wasm_file(
    source: str,
    output_path: str | Path,
    *,
    build_config: BuildConfig | None = None,
    optimize_ir: bool = True,
) -> PackageResult:
    return NibWasmCompiler(
        build_config=build_config,
        optimize_ir=optimize_ir,
    ).write_wasm_file(source, output_path, build_config=build_config, optimize_ir=optimize_ir)


def _extract_signatures(typed_ast) -> list[FunctionSignature]:  # type: ignore[no-untyped-def]
    signatures = [FunctionSignature(label="_start", param_count=0, export_name="_start")]

    for child in getattr(typed_ast, "children", []):
        inner = _unwrap_top_decl(child)
        if inner is None or getattr(inner, "rule_name", None) != "fn_decl":
            continue

        fn_name: str | None = None
        param_count = 0

        for node in inner.children:
            if hasattr(node, "value") and _token_type(node) == "NAME" and fn_name is None:
                fn_name = node.value
            elif getattr(node, "rule_name", None) == "param_list":
                param_count = _count_params(node)

        if fn_name is not None:
            signatures.append(
                FunctionSignature(
                    label=f"_fn_{fn_name}",
                    param_count=param_count,
                    export_name=fn_name,
                )
            )

    return signatures


def _unwrap_top_decl(child):  # type: ignore[no-untyped-def]
    if getattr(child, "children", None) is None:
        return None
    for grandchild in child.children:
        if getattr(grandchild, "rule_name", None) is not None:
            return grandchild
    return None


def _token_type(token) -> str:  # type: ignore[no-untyped-def]
    token_type = getattr(token, "type", None)
    return token_type if isinstance(token_type, str) else getattr(token_type, "name", "")


def _count_params(param_list) -> int:  # type: ignore[no-untyped-def]
    return sum(
        1
        for param in getattr(param_list, "children", [])
        if getattr(param, "rule_name", None) == "param"
    )
