"""Readable WASM assembly emission from the generic compiler IR."""

from __future__ import annotations

from wasm_leb128 import decode_signed
from wasm_execution.decoder import decode_function_body
from wasm_opcodes import get_opcode
from wasm_types import BlockType, ExternalKind, ValueType, WasmModule

from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler
from ir_to_wasm_validator import validate


class WasmAssemblyError(Exception):
    """Raised when readable WASM assembly cannot be generated."""


def emit_wasm_assembly(
    program,
    function_signatures: list[FunctionSignature] | None = None,
) -> str:
    errors = validate(program, function_signatures)
    if errors:
        raise WasmAssemblyError(errors[0].message)
    module = IrToWasmCompiler().compile(program, function_signatures)
    return print_module(module)


def print_module(module: WasmModule) -> str:
    lines: list[str] = []

    for index, func_type in enumerate(module.types):
        lines.append(
            f".type {index} params={_types_csv(func_type.params)} "
            f"results={_types_csv(func_type.results)}"
        )

    for imp in module.imports:
        if imp.kind == ExternalKind.FUNCTION:
            lines.append(f".import function {imp.module_name} {imp.name} type={imp.type_info}")
        elif imp.kind == ExternalKind.MEMORY:
            memory = imp.type_info
            max_part = "none" if memory.limits.max is None else str(memory.limits.max)
            lines.append(
                f".import memory {imp.module_name} {imp.name} "
                f"min={memory.limits.min} max={max_part}"
            )
        elif imp.kind == ExternalKind.TABLE:
            table = imp.type_info
            max_part = "none" if table.limits.max is None else str(table.limits.max)
            elem = "funcref" if table.element_type == 0x70 else str(table.element_type)
            lines.append(
                f".import table {imp.module_name} {imp.name} "
                f"elem={elem} min={table.limits.min} max={max_part}"
            )
        elif imp.kind == ExternalKind.GLOBAL:
            global_type = imp.type_info
            lines.append(
                f".import global {imp.module_name} {imp.name} "
                f"type={_type_name(global_type.value_type)} "
                f"mutable={str(global_type.mutable).lower()}"
            )

    for index, memory in enumerate(module.memories):
        max_part = "none" if memory.limits.max is None else str(memory.limits.max)
        lines.append(f".memory {index} min={memory.limits.min} max={max_part}")

    for export in module.exports:
        kind = {
            ExternalKind.FUNCTION: "function",
            ExternalKind.TABLE: "table",
            ExternalKind.MEMORY: "memory",
            ExternalKind.GLOBAL: "global",
        }[export.kind]
        lines.append(f".export {kind} {export.name} {export.index}")

    for index, type_index in enumerate(module.functions):
        body = module.code[index]
        lines.append(
            f".func {index} type={type_index} locals={_types_csv(body.locals)}"
        )
        for instruction in decode_function_body(body.code):
            lines.append(f"  {_format_instruction(instruction.opcode, instruction.operand)}")
        lines.append(".endfunc")

    for segment in module.data:
        lines.append(
            f".data {segment.memory_index} offset={_const_offset(segment.offset_expr)} "
            f"bytes={_bytes_csv(segment.data)}"
        )

    return "\n".join(lines) + "\n"


def _format_instruction(opcode: int, operand: object) -> str:
    info = get_opcode(opcode)
    if info is None:
        raise WasmAssemblyError(f"unknown opcode byte: 0x{opcode:02X}")

    if operand is None:
        return info.name

    if info.immediates == ("blocktype",):
        return f"{info.name} {_blocktype_name(operand)}"

    if info.immediates == ("memarg",):
        if not isinstance(operand, dict):
            raise WasmAssemblyError(f"expected memarg for {info.name}")
        return f"{info.name} align={operand['align']} offset={operand['offset']}"

    return f"{info.name} {operand}"


def _blocktype_name(value: object) -> str:
    if value == int(BlockType.EMPTY):
        return "void"
    value_type_map = {
        int(ValueType.I32): "i32",
        int(ValueType.I64): "i64",
        int(ValueType.F32): "f32",
        int(ValueType.F64): "f64",
    }
    if value in value_type_map:
        return value_type_map[int(value)]
    return str(value)


def _types_csv(types: tuple[ValueType, ...]) -> str:
    if not types:
        return "none"
    return ",".join(_type_name(value_type) for value_type in types)


def _type_name(value_type: ValueType) -> str:
    type_names = {
        ValueType.I32: "i32",
        ValueType.I64: "i64",
        ValueType.F32: "f32",
        ValueType.F64: "f64",
    }
    return type_names[value_type]


def _const_offset(expr: bytes) -> int:
    if len(expr) < 2 or expr[0] != 0x41 or expr[-1] != 0x0B:
        raise WasmAssemblyError("only i32.const data offsets are supported")
    value, _ = decode_signed(expr, 1)
    return value


def _bytes_csv(data: bytes) -> str:
    if not data:
        return "none"
    return ",".join(f"{byte:02X}" for byte in data)


__all__ = [
    "WasmAssemblyError",
    "emit_wasm_assembly",
    "print_module",
]
