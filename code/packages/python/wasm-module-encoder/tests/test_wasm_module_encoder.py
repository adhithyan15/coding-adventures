from __future__ import annotations

import pytest
from wasm_module_parser import WasmModuleParser
from wasm_types import (
    CustomSection,
    DataSegment,
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

from wasm_module_encoder import WASM_MAGIC, WASM_VERSION, WasmEncodeError, encode_module


def _minimal_module() -> WasmModule:
    return WasmModule(
        types=[FuncType(params=(ValueType.I32,), results=(ValueType.I32,))],
        imports=[],
        functions=[0],
        tables=[],
        memories=[],
        globals=[],
        exports=[Export(name="identity", kind=ExternalKind.FUNCTION, index=0)],
        start=None,
        elements=[],
        code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
        data=[],
        customs=[],
    )


def test_encode_minimal_module_round_trips_through_parser() -> None:
    module = _minimal_module()
    encoded = encode_module(module)
    parsed = WasmModuleParser().parse(encoded)

    assert encoded.startswith(WASM_MAGIC + WASM_VERSION)
    assert parsed.types == module.types
    assert parsed.functions == module.functions
    assert parsed.exports == module.exports
    assert parsed.code == module.code


def test_encode_module_with_memory_data_global_and_start_round_trips() -> None:
    module = WasmModule(
        types=[FuncType(params=(), results=(ValueType.I32,))],
        imports=[],
        functions=[0],
        tables=[],
        memories=[MemoryType(limits=Limits(min=1, max=2))],
        globals=[
            Global(
                global_type=GlobalType(value_type=ValueType.I32, mutable=False),
                init_expr=bytes([0x41, 0x2A, 0x0B]),
            )
        ],
        exports=[
            Export(name="main", kind=ExternalKind.FUNCTION, index=0),
            Export(name="memory", kind=ExternalKind.MEMORY, index=0),
        ],
        start=0,
        elements=[],
        code=[FunctionBody(locals=(ValueType.I32,), code=bytes([0x41, 0x07, 0x0B]))],
        data=[DataSegment(memory_index=0, offset_expr=bytes([0x41, 0x00, 0x0B]), data=b"Nib")],
        customs=[],
    )

    parsed = WasmModuleParser().parse(encode_module(module))
    assert parsed.memories == module.memories
    assert parsed.globals == module.globals
    assert parsed.start == module.start
    assert parsed.data == module.data


def test_encode_module_with_imports_table_and_custom_section_round_trips() -> None:
    module = WasmModule(
        types=[FuncType(params=(), results=())],
        imports=[
            Import(module_name="env", name="f", kind=ExternalKind.FUNCTION, type_info=0),
            Import(
                module_name="env",
                name="table",
                kind=ExternalKind.TABLE,
                type_info=TableType(element_type=0x70, limits=Limits(min=1, max=4)),
            ),
            Import(
                module_name="env",
                name="memory",
                kind=ExternalKind.MEMORY,
                type_info=MemoryType(limits=Limits(min=1, max=None)),
            ),
            Import(
                module_name="env",
                name="glob",
                kind=ExternalKind.GLOBAL,
                type_info=GlobalType(value_type=ValueType.I32, mutable=True),
            ),
        ],
        functions=[],
        tables=[],
        memories=[],
        globals=[],
        exports=[],
        start=None,
        elements=[],
        code=[],
        data=[],
        customs=[CustomSection(name="name", data=b"\x01\x02")],
    )

    parsed = WasmModuleParser().parse(encode_module(module))
    assert parsed.imports == module.imports
    assert parsed.customs == module.customs


def test_encode_invalid_function_import_type_raises() -> None:
    module = WasmModule(
        types=[],
        imports=[
            Import(
                module_name="env",
                name="f",
                kind=ExternalKind.FUNCTION,
                type_info=MemoryType(limits=Limits(min=1, max=None)),
            )
        ],
        functions=[],
        tables=[],
        memories=[],
        globals=[],
        exports=[],
        start=None,
        elements=[],
        code=[],
        data=[],
        customs=[],
    )

    with pytest.raises(WasmEncodeError, match="function imports require an integer type index"):
        encode_module(module)
