"""test_runtime.py --- Tests for WasmRuntime instantiation and call API.

Covers: instantiate with memory, globals, data segments, element segments,
call with argument type conversion, export lookup failures, and the
load_and_run shortcut.
"""

from __future__ import annotations

import pytest
from wasm_execution import TrapError, WasmExecutionLimits
from wasm_types import (
    DataSegment,
    Element,
    Export,
    ExternalKind,
    FunctionBody,
    FuncType,
    Global,
    GlobalType,
    Import,
    Limits,
    MemoryType,
    TableType,
    ValueType,
    WasmModule,
)

from wasm_runtime.runtime import WasmRuntime

# ===========================================================================
# Helper: build a module that computes f(x) = x * x
# ===========================================================================


def _square_module() -> WasmModule:
    """Build a module with a single function: square(x: i32) -> i32.

    WASM bytecode: local.get 0, local.get 0, i32.mul, end
    """
    return WasmModule(
        types=[FuncType(params=(ValueType.I32,), results=(ValueType.I32,))],
        imports=[],
        functions=[0],
        tables=[],
        memories=[],
        globals=[],
        exports=[Export(name="square", kind=ExternalKind.FUNCTION, index=0)],
        start=None,
        elements=[],
        code=[
            FunctionBody(
                locals=(),
                code=bytes([0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B]),
            )
        ],
        data=[],
        customs=[],
    )


def _identity_module() -> WasmModule:
    """Build a module with identity(x: i32) -> i32 = local.get 0, end."""
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


def _spinning_module() -> WasmModule:
    """Build a module with spin() = loop { br 0 }."""
    return WasmModule(
        types=[FuncType(params=(), results=())],
        imports=[],
        functions=[0],
        tables=[],
        memories=[],
        globals=[],
        exports=[Export(name="spin", kind=ExternalKind.FUNCTION, index=0)],
        start=None,
        elements=[],
        code=[
            FunctionBody(
                locals=(),
                code=bytes([0x03, 0x40, 0x0C, 0x00, 0x0B]),
            )
        ],
        data=[],
        customs=[],
    )


# ===========================================================================
# Basic instantiation and call
# ===========================================================================


class TestRuntimeBasic:
    def test_instantiate_and_call_square(self) -> None:
        runtime = WasmRuntime()
        module = _square_module()
        runtime.validate(module)
        instance = runtime.instantiate(module)
        result = runtime.call(instance, "square", [5])
        assert result == [25]

    def test_call_identity(self) -> None:
        runtime = WasmRuntime()
        module = _identity_module()
        instance = runtime.instantiate(module)
        result = runtime.call(instance, "identity", [42])
        assert result == [42]

    def test_call_nonexistent_export(self) -> None:
        runtime = WasmRuntime()
        module = _square_module()
        instance = runtime.instantiate(module)
        with pytest.raises(TrapError, match='export "missing" not found'):
            runtime.call(instance, "missing", [])

    def test_call_non_function_export(self) -> None:
        """Calling a non-function export should trap."""
        module = WasmModule(
            types=[FuncType(params=(ValueType.I32,), results=(ValueType.I32,))],
            imports=[],
            functions=[0],
            tables=[],
            memories=[MemoryType(limits=Limits(min=1, max=None))],
            globals=[],
            exports=[
                Export(name="mem", kind=ExternalKind.MEMORY, index=0),
                Export(name="f", kind=ExternalKind.FUNCTION, index=0),
            ],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        with pytest.raises(TrapError, match="not a function"):
            runtime.call(instance, "mem", [])

    def test_instruction_budget_traps_nonterminating_function(self) -> None:
        runtime = WasmRuntime(limits=WasmExecutionLimits(max_instructions=8))
        instance = runtime.instantiate(_spinning_module())

        with pytest.raises(TrapError, match="instruction budget exhausted"):
            runtime.call(instance, "spin", [])


# ===========================================================================
# Memory and data segments
# ===========================================================================


class TestRuntimeMemory:
    def test_memory_allocated(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(), results=())],
            imports=[],
            functions=[0],
            tables=[],
            memories=[MemoryType(limits=Limits(min=1, max=10))],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        assert instance.memory is not None
        assert instance.memory.size() == 1

    def test_data_segment_applied(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(), results=())],
            imports=[],
            functions=[0],
            tables=[],
            memories=[MemoryType(limits=Limits(min=1, max=None))],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
            data=[
                DataSegment(
                    memory_index=0,
                    offset_expr=bytes([0x41, 0x00, 0x0B]),  # i32.const 0
                    data=b"WASM",
                ),
            ],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        assert instance.memory.load_i32_8u(0) == ord("W")
        assert instance.memory.load_i32_8u(1) == ord("A")
        assert instance.memory.load_i32_8u(2) == ord("S")
        assert instance.memory.load_i32_8u(3) == ord("M")


# ===========================================================================
# Globals
# ===========================================================================


class TestRuntimeGlobals:
    def test_globals_initialized(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(), results=())],
            imports=[],
            functions=[0],
            tables=[],
            memories=[],
            globals=[
                Global(
                    global_type=GlobalType(value_type=ValueType.I32, mutable=False),
                    init_expr=bytes([0x41, 0x2A, 0x0B]),  # i32.const 42
                ),
            ],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        assert len(instance.globals) == 1
        assert instance.globals[0].value == 42


# ===========================================================================
# Tables and element segments
# ===========================================================================


class TestRuntimeTables:
    def test_table_allocated(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(), results=())],
            imports=[],
            functions=[0],
            tables=[TableType(element_type=0x70, limits=Limits(min=10, max=None))],
            memories=[],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        assert len(instance.tables) == 1
        assert instance.tables[0].size() == 10

    def test_element_segment_applied(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(), results=())],
            imports=[],
            functions=[0],
            tables=[TableType(element_type=0x70, limits=Limits(min=10, max=None))],
            memories=[],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[
                Element(
                    table_index=0,
                    offset_expr=bytes([0x41, 0x00, 0x0B]),  # i32.const 0
                    function_indices=[0],
                ),
            ],
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        assert instance.tables[0].get(0) == 0


# ===========================================================================
# Argument type conversion
# ===========================================================================


class TestArgumentConversion:
    def test_f64_argument(self) -> None:
        """Runtime should convert float args based on param type."""
        module = WasmModule(
            types=[FuncType(params=(ValueType.F64,), results=(ValueType.F64,))],
            imports=[],
            functions=[0],
            tables=[],
            memories=[],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        result = runtime.call(instance, "f", [3.14])
        assert result[0] == pytest.approx(3.14)

    def test_i64_argument(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(ValueType.I64,), results=(ValueType.I64,))],
            imports=[],
            functions=[0],
            tables=[],
            memories=[],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        result = runtime.call(instance, "f", [100])
        assert result == [100]

    def test_f32_argument(self) -> None:
        module = WasmModule(
            types=[FuncType(params=(ValueType.F32,), results=(ValueType.F32,))],
            imports=[],
            functions=[0],
            tables=[],
            memories=[],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
            data=[],
            customs=[],
        )
        runtime = WasmRuntime()
        instance = runtime.instantiate(module)
        result = runtime.call(instance, "f", [1.5])
        assert result[0] == pytest.approx(1.5, abs=1e-5)


# ===========================================================================
# WASI host integration
# ===========================================================================


class TestRuntimeWithWasi:
    def test_wasi_host_integration(self) -> None:
        """Runtime with WASI host should be able to resolve imported funcs."""
        from wasm_runtime.wasi_host import WasiHost
        wasi = WasiHost()
        runtime = WasmRuntime(host=wasi)

        module = WasmModule(
            types=[
                FuncType(
                    params=(
                        ValueType.I32,
                        ValueType.I32,
                        ValueType.I32,
                        ValueType.I32,
                    ),
                    results=(ValueType.I32,),
                ),
                FuncType(params=(ValueType.I32,), results=(ValueType.I32,)),
            ],
            imports=[
                Import(
                    module_name="wasi_snapshot_preview1",
                    name="fd_write",
                    kind=ExternalKind.FUNCTION,
                    type_info=0,
                ),
            ],
            functions=[1],
            tables=[],
            memories=[MemoryType(limits=Limits(min=1, max=None))],
            globals=[],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=1)],
            start=None,
            elements=[],
            code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
            data=[],
            customs=[],
        )
        instance = runtime.instantiate(module)
        # The imported fd_write should be a host function
        assert instance.host_functions[0] is not None
        # Our module function should work
        result = runtime.call(instance, "f", [7])
        assert result == [7]
