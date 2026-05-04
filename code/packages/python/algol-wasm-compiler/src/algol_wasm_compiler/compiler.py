"""End-to-end ALGOL 60 to WebAssembly packaging."""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from algol_ir_compiler import CompileResult, compile_algol
from algol_parser import parse_algol
from algol_type_checker import TypeCheckResult, check_algol
from compiler_ir import IrLabel, IrOp, IrProgram
from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler
from ir_to_wasm_validator import validate as validate_ir_to_wasm
from wasm_module_encoder import encode_module
from wasm_types import ValueType, WasmModule
from wasm_validator import ValidatedModule, ValidationError, validate

MAX_SOURCE_LENGTH = 256 * 1024


@dataclass(frozen=True)
class AlgolWasmResult:
    """Artifacts produced by a successful ALGOL 60 WASM compilation."""

    source: str
    ast: object
    typed: TypeCheckResult
    ir: CompileResult
    module: WasmModule
    validated_module: ValidatedModule
    binary: bytes
    wasm_path: Path | None = None


class AlgolWasmError(Exception):
    """Wrap an end-to-end compiler failure with the stage that produced it."""

    def __init__(
        self, stage: str, message: str, cause: Exception | None = None
    ) -> None:
        super().__init__(message)
        self.stage = stage
        self.message = message
        self.cause = cause

    def __str__(self) -> str:
        return f"[{self.stage}] {self.message}"


class AlgolWasmCompiler:
    """Compile the current ALGOL 60 subset into WebAssembly bytes."""

    def compile_source(self, source: str) -> AlgolWasmResult:
        if len(source) > MAX_SOURCE_LENGTH:
            raise AlgolWasmError(
                "source",
                "ALGOL source length "
                f"{len(source)} exceeds configured limit {MAX_SOURCE_LENGTH}",
            )

        try:
            ast = parse_algol(source)
        except Exception as exc:
            raise AlgolWasmError("parse", str(exc), exc) from exc

        typed = check_algol(ast)
        if not typed.ok:
            diagnostics = "\n".join(
                f"Line {diag.line}, Col {diag.column}: {diag.message}"
                for diag in typed.diagnostics
            )
            raise AlgolWasmError("type-check", diagnostics)

        try:
            ir = compile_algol(typed)
        except Exception as exc:
            raise AlgolWasmError("ir-compile", str(exc), exc) from exc

        signatures = [
            FunctionSignature(label="_start", param_count=0, export_name="_start")
        ]
        signatures.extend(
            FunctionSignature(
                label=label,
                param_count=plan.param_count,
                require_explicit_args=True,
                param_types=tuple(
                    _wasm_value_type(type_name) for type_name in plan.param_types
                ),
                result_types=(
                    (_wasm_value_type(plan.return_type),)
                    if plan.return_type is not None
                    else ()
                ),
            )
            for label, plan in sorted(ir.procedure_signatures.items())
        )
        strategy = _wasm_lowering_strategy(ir.program)
        lowering_errors = validate_ir_to_wasm(
            ir.program,
            signatures,
            strategy=strategy,
        )
        if lowering_errors:
            raise AlgolWasmError("validate-ir", lowering_errors[0].message)

        try:
            module = IrToWasmCompiler().compile(
                ir.program,
                signatures,
                strategy=strategy,
            )
        except Exception as exc:
            raise AlgolWasmError("wasm-lower", str(exc), exc) from exc

        try:
            validated_module = validate(module)
        except ValidationError as exc:
            raise AlgolWasmError("validate-wasm", str(exc), exc) from exc

        try:
            binary = encode_module(module)
        except Exception as exc:
            raise AlgolWasmError("encode", str(exc), exc) from exc

        return AlgolWasmResult(
            source=source,
            ast=ast,
            typed=typed,
            ir=ir,
            module=module,
            validated_module=validated_module,
            binary=binary,
        )

    def write_wasm_file(self, source: str, output_path: str | Path) -> AlgolWasmResult:
        result = self.compile_source(source)
        path = Path(output_path)
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(result.binary)
        except OSError as exc:
            raise AlgolWasmError("write", str(exc), exc) from exc
        return replace(result, wasm_path=path)


def compile_source(source: str) -> AlgolWasmResult:
    return AlgolWasmCompiler().compile_source(source)


def pack_source(source: str) -> AlgolWasmResult:
    return compile_source(source)


def write_wasm_file(source: str, output_path: str | Path) -> AlgolWasmResult:
    return AlgolWasmCompiler().write_wasm_file(source, output_path)


def _wasm_lowering_strategy(program: IrProgram) -> str:
    for instruction in program.instructions:
        if instruction.opcode not in (IrOp.BRANCH_NZ, IrOp.BRANCH_Z, IrOp.JUMP):
            continue
        if (
            instruction.operands
            and isinstance(instruction.operands[-1], IrLabel)
            and not _structured_lowerer_target(instruction.operands[-1].name)
        ):
            return "dispatch_loop"
    return "structured"


def _structured_lowerer_target(label_name: str) -> bool:
    parts = label_name.split("_")
    if len(parts) != 3 or not parts[1].isdigit():
        return False
    prefix, _index, suffix = parts
    return (prefix == "if" and suffix in {"else", "end"}) or (
        prefix == "loop" and suffix in {"start", "end"}
    )


def _wasm_value_type(type_name: str | None) -> ValueType:
    if type_name == "real":
        return ValueType.F64
    return ValueType.I32
