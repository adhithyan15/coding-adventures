"""wasm-module-encoder --- Generic WebAssembly 1.0 module encoder."""

from __future__ import annotations

from wasm_leb128 import encode_unsigned
from wasm_types import (
    CustomSection,
    DataSegment,
    Element,
    Export,
    ExternalKind,
    FuncType,
    FunctionBody,
    Global,
    GlobalType,
    Import,
    Limits,
    MemoryType,
    TableType,
    ValueType,
    WasmModule,
)

__version__ = "0.1.0"

WASM_MAGIC = b"\x00asm"
WASM_VERSION = b"\x01\x00\x00\x00"


class WasmEncodeError(Exception):
    """Raised when a WasmModule cannot be encoded."""


def encode_module(module: WasmModule) -> bytes:
    """Encode a WasmModule into raw WebAssembly 1.0 bytes."""

    sections: list[bytes] = []

    for custom in module.customs:
        sections.append(_section(0, _encode_custom(custom)))
    if module.types:
        sections.append(_section(1, _vector(module.types, _encode_func_type)))
    if module.imports:
        sections.append(_section(2, _vector(module.imports, _encode_import)))
    if module.functions:
        sections.append(_section(3, _vector(module.functions, _u32)))
    if module.tables:
        sections.append(_section(4, _vector(module.tables, _encode_table_type)))
    if module.memories:
        sections.append(_section(5, _vector(module.memories, _encode_memory_type)))
    if module.globals:
        sections.append(_section(6, _vector(module.globals, _encode_global)))
    if module.exports:
        sections.append(_section(7, _vector(module.exports, _encode_export)))
    if module.start is not None:
        sections.append(_section(8, _u32(module.start)))
    if module.elements:
        sections.append(_section(9, _vector(module.elements, _encode_element)))
    if module.code:
        sections.append(_section(10, _vector(module.code, _encode_function_body)))
    if module.data:
        sections.append(_section(11, _vector(module.data, _encode_data_segment)))

    return WASM_MAGIC + WASM_VERSION + b"".join(sections)


def _section(section_id: int, payload: bytes) -> bytes:
    return bytes([section_id]) + _u32(len(payload)) + payload


def _u32(value: int) -> bytes:
    return encode_unsigned(value)


def _name(text: str) -> bytes:
    data = text.encode("utf-8")
    return _u32(len(data)) + data


def _vector(values: list, encoder) -> bytes:  # type: ignore[no-untyped-def]
    encoded = bytearray()
    encoded.extend(_u32(len(values)))
    for value in values:
        encoded.extend(encoder(value))
    return bytes(encoded)


def _value_types(types: tuple[ValueType, ...]) -> bytes:
    return _u32(len(types)) + bytes(int(value_type) for value_type in types)


def _encode_func_type(func_type: FuncType) -> bytes:
    return bytes([0x60]) + _value_types(func_type.params) + _value_types(func_type.results)


def _encode_limits(limits: Limits) -> bytes:
    if limits.max is None:
        return b"\x00" + _u32(limits.min)
    return b"\x01" + _u32(limits.min) + _u32(limits.max)


def _encode_memory_type(memory_type: MemoryType) -> bytes:
    return _encode_limits(memory_type.limits)


def _encode_table_type(table_type: TableType) -> bytes:
    return bytes([table_type.element_type]) + _encode_limits(table_type.limits)


def _encode_global_type(global_type: GlobalType) -> bytes:
    return bytes([int(global_type.value_type), 0x01 if global_type.mutable else 0x00])


def _encode_import(import_: Import) -> bytes:
    payload = bytearray()
    payload.extend(_name(import_.module_name))
    payload.extend(_name(import_.name))
    payload.append(int(import_.kind))

    if import_.kind == ExternalKind.FUNCTION:
        if not isinstance(import_.type_info, int):
            raise WasmEncodeError("function imports require an integer type index")
        payload.extend(_u32(import_.type_info))
    elif import_.kind == ExternalKind.TABLE:
        if not isinstance(import_.type_info, TableType):
            raise WasmEncodeError("table imports require TableType metadata")
        payload.extend(_encode_table_type(import_.type_info))
    elif import_.kind == ExternalKind.MEMORY:
        if not isinstance(import_.type_info, MemoryType):
            raise WasmEncodeError("memory imports require MemoryType metadata")
        payload.extend(_encode_memory_type(import_.type_info))
    elif import_.kind == ExternalKind.GLOBAL:
        if not isinstance(import_.type_info, GlobalType):
            raise WasmEncodeError("global imports require GlobalType metadata")
        payload.extend(_encode_global_type(import_.type_info))
    else:  # pragma: no cover - defensive guard
        raise WasmEncodeError(f"unsupported import kind: {import_.kind!r}")

    return bytes(payload)


def _encode_export(export: Export) -> bytes:
    return _name(export.name) + bytes([int(export.kind)]) + _u32(export.index)


def _encode_global(global_: Global) -> bytes:
    return _encode_global_type(global_.global_type) + global_.init_expr


def _encode_element(element: Element) -> bytes:
    payload = bytearray()
    payload.extend(_u32(element.table_index))
    payload.extend(element.offset_expr)
    payload.extend(_u32(len(element.function_indices)))
    for func_index in element.function_indices:
        payload.extend(_u32(func_index))
    return bytes(payload)


def _encode_data_segment(segment: DataSegment) -> bytes:
    return _u32(segment.memory_index) + segment.offset_expr + _u32(len(segment.data)) + segment.data


def _encode_function_body(body: FunctionBody) -> bytes:
    local_groups = _group_locals(body.locals)
    payload = bytearray()
    payload.extend(_u32(len(local_groups)))
    for count, value_type in local_groups:
        payload.extend(_u32(count))
        payload.append(int(value_type))
    payload.extend(body.code)
    return _u32(len(payload)) + bytes(payload)


def _group_locals(locals_: tuple[ValueType, ...]) -> list[tuple[int, ValueType]]:
    if not locals_:
        return []

    groups: list[tuple[int, ValueType]] = []
    current_type = locals_[0]
    count = 1

    for value_type in locals_[1:]:
        if value_type == current_type:
            count += 1
            continue
        groups.append((count, current_type))
        current_type = value_type
        count = 1

    groups.append((count, current_type))
    return groups


def _encode_custom(custom: CustomSection) -> bytes:
    return _name(custom.name) + custom.data


__all__ = [
    "WASM_MAGIC",
    "WASM_VERSION",
    "WasmEncodeError",
    "encode_module",
]
