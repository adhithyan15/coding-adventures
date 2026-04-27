"""Assemble readable WASM assembly text into .wasm bytes."""

from __future__ import annotations

from dataclasses import dataclass

from wasm_leb128 import encode_signed, encode_unsigned
from wasm_module_encoder import encode_module
from wasm_opcodes import get_opcode_by_name
from wasm_types import (
    DataSegment,
    Export,
    ExternalKind,
    FuncType,
    FunctionBody,
    GlobalType,
    Import,
    Limits,
    MemoryType,
    TableType,
    ValueType,
    WasmModule,
)


class WasmAssemblerError(Exception):
    """Raised when readable WASM assembly cannot be parsed."""


def assemble(text: str) -> bytes:
    return encode_module(parse_assembly(text))


def parse_assembly(text: str) -> WasmModule:
    module = WasmModule()

    types: dict[int, FuncType] = {}
    memories: dict[int, MemoryType] = {}
    functions: dict[int, int] = {}
    code: dict[int, FunctionBody] = {}
    imports: list[Import] = []
    exports: list[Export] = []
    data_segments: list[DataSegment] = []

    current_func_index: int | None = None
    current_func_type: int | None = None
    current_locals: tuple[ValueType, ...] = ()
    current_body = bytearray()

    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or line.startswith(";"):
            continue

        if current_func_index is not None and not line.startswith("."):
            current_body.extend(_assemble_instruction(line))
            continue

        if line.startswith(".import "):
            _, kind_text, module_name, name, *rest = line.split()
            kv = {piece.split("=", 1)[0]: piece.split("=", 1)[1] for piece in rest}
            kind = _parse_external_kind(kind_text)
            if kind == ExternalKind.FUNCTION:
                type_info = int(kv["type"])
            elif kind == ExternalKind.MEMORY:
                max_text = kv["max"]
                type_info = MemoryType(
                    limits=Limits(
                        min=int(kv["min"]),
                        max=None if max_text == "none" else int(max_text),
                    )
                )
            elif kind == ExternalKind.TABLE:
                max_text = kv["max"]
                type_info = TableType(
                    element_type=_parse_element_type(kv["elem"]),
                    limits=Limits(
                        min=int(kv["min"]),
                        max=None if max_text == "none" else int(max_text),
                    ),
                )
            else:
                type_info = GlobalType(
                    value_type=_parse_value_type(kv["type"]),
                    mutable=kv["mutable"] == "true",
                )
            imports.append(
                Import(
                    module_name=module_name,
                    name=name,
                    kind=kind,
                    type_info=type_info,
                )
            )
            continue

        if line.startswith(".type "):
            _, index_text, params_part, results_part = line.split()
            index = int(index_text)
            params = _parse_types(params_part.split("=", 1)[1])
            results = _parse_types(results_part.split("=", 1)[1])
            types[index] = FuncType(params=params, results=results)
            continue

        if line.startswith(".memory "):
            _, index_text, min_part, max_part = line.split()
            index = int(index_text)
            min_pages = int(min_part.split("=", 1)[1])
            max_text = max_part.split("=", 1)[1]
            max_pages = None if max_text == "none" else int(max_text)
            memories[index] = MemoryType(limits=Limits(min=min_pages, max=max_pages))
            continue

        if line.startswith(".export "):
            _, kind_text, name, index_text = line.split()
            exports.append(
                Export(
                    name=name,
                    kind=_parse_external_kind(kind_text),
                    index=int(index_text),
                )
            )
            continue

        if line.startswith(".func "):
            _, index_text, type_part, locals_part = line.split()
            current_func_index = int(index_text)
            current_func_type = int(type_part.split("=", 1)[1])
            current_locals = _parse_types(locals_part.split("=", 1)[1])
            current_body = bytearray()
            continue

        if line == ".endfunc":
            if current_func_index is None or current_func_type is None:
                raise WasmAssemblerError(".endfunc without active function")
            functions[current_func_index] = current_func_type
            code[current_func_index] = FunctionBody(locals=current_locals, code=bytes(current_body))
            current_func_index = None
            current_func_type = None
            current_locals = ()
            current_body = bytearray()
            continue

        if line.startswith(".data "):
            _, memory_text, offset_part, bytes_part = line.split()
            offset = int(offset_part.split("=", 1)[1])
            byte_text = bytes_part.split("=", 1)[1]
            data = b"" if byte_text == "none" else bytes(int(piece, 16) for piece in byte_text.split(","))
            data_segments.append(
                DataSegment(memory_index=int(memory_text), offset_expr=_const_expr(offset), data=data)
            )
            continue

        raise WasmAssemblerError(f"unrecognized assembly line: {line}")

    if current_func_index is not None:
        raise WasmAssemblerError("unterminated .func block")

    module.types = [types[index] for index in sorted(types)]
    module.imports = imports
    module.memories = [memories[index] for index in sorted(memories)]
    module.functions = [functions[index] for index in sorted(functions)]
    module.code = [code[index] for index in sorted(code)]
    module.exports = exports
    module.data = data_segments
    return module


def _assemble_instruction(line: str) -> bytes:
    parts = line.split()
    if not parts:
        return b""

    mnemonic = parts[0]
    info = get_opcode_by_name(mnemonic)
    if info is None:
        raise WasmAssemblerError(f"unknown instruction: {mnemonic}")

    encoded = bytearray([info.opcode])

    if info.immediates == ():
        return bytes(encoded)

    if info.immediates == ("i32",):
        encoded.extend(encode_signed(int(parts[1])))
        return bytes(encoded)

    if info.immediates == ("blocktype",):
        encoded.extend(_encode_blocktype(parts[1]))
        return bytes(encoded)

    if info.immediates == ("memarg",):
        kv = {piece.split("=", 1)[0]: piece.split("=", 1)[1] for piece in parts[1:]}
        encoded.extend(encode_unsigned(int(kv["align"])))
        encoded.extend(encode_unsigned(int(kv["offset"])))
        return bytes(encoded)

    immediate = int(parts[1])
    encoded.extend(encode_unsigned(immediate))
    return bytes(encoded)


def _parse_types(text: str) -> tuple[ValueType, ...]:
    if text == "none":
        return ()
    return tuple(_parse_value_type(piece) for piece in text.split(","))


def _parse_external_kind(text: str) -> ExternalKind:
    mapping = {
        "function": ExternalKind.FUNCTION,
        "table": ExternalKind.TABLE,
        "memory": ExternalKind.MEMORY,
        "global": ExternalKind.GLOBAL,
    }
    return mapping[text]


def _parse_value_type(text: str) -> ValueType:
    mapping = {
        "i32": ValueType.I32,
        "i64": ValueType.I64,
        "f32": ValueType.F32,
        "f64": ValueType.F64,
    }
    return mapping[text]


def _parse_element_type(text: str) -> int:
    if text == "funcref":
        return 0x70
    return int(text)


def _encode_blocktype(text: str) -> bytes:
    mapping = {
        "void": bytes([0x40]),
        "i32": bytes([int(ValueType.I32)]),
        "i64": bytes([int(ValueType.I64)]),
        "f32": bytes([int(ValueType.F32)]),
        "f64": bytes([int(ValueType.F64)]),
    }
    if text in mapping:
        return mapping[text]
    return encode_signed(int(text))


def _const_expr(value: int) -> bytes:
    i32_const = get_opcode_by_name("i32.const").opcode
    end = get_opcode_by_name("end").opcode
    return bytes([i32_const]) + encode_signed(value) + bytes([end])


__all__ = [
    "WasmAssemblerError",
    "assemble",
    "parse_assembly",
]
