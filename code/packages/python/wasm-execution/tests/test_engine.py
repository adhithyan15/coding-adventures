"""Basic tests for the WASM execution engine."""

import pytest
from wasm_types import FuncType, FunctionBody, GlobalType, ValueType

from wasm_execution import (
    LinearMemory,
    Table,
    TrapError,
    WasmExecutionEngine,
    as_i32,
    default_value,
    i32,
)


def _make_engine(
    body_code: bytes,
    params: tuple[ValueType, ...] = (),
    results: tuple[ValueType, ...] = (),
    locals_: tuple[ValueType, ...] = (),
) -> WasmExecutionEngine:
    """Create a simple engine with one function."""
    func_type = FuncType(params=params, results=results)
    body = FunctionBody(locals=locals_, code=body_code)
    return WasmExecutionEngine(
        memory=None,
        tables=[],
        globals=[],
        global_types=[],
        func_types=[func_type],
        func_bodies=[body],
        host_functions=[None],
    )


class TestI32Const:
    """Test i32.const instruction."""

    def test_const_42(self) -> None:
        # i32.const 42; end
        code = bytes([0x41, 0x2A, 0x0B])
        engine = _make_engine(code, results=(ValueType.I32,))
        result = engine.call_function(0, [])
        assert len(result) == 1
        assert as_i32(result[0]) == 42

    def test_const_negative(self) -> None:
        # i32.const -1; end
        # -1 in signed LEB128 is 0x7F
        code = bytes([0x41, 0x7F, 0x0B])
        engine = _make_engine(code, results=(ValueType.I32,))
        result = engine.call_function(0, [])
        assert as_i32(result[0]) == -1


class TestI32Arithmetic:
    """Test basic i32 arithmetic."""

    def test_add(self) -> None:
        # i32.const 3; i32.const 4; i32.add; end
        code = bytes([0x41, 0x03, 0x41, 0x04, 0x6A, 0x0B])
        engine = _make_engine(code, results=(ValueType.I32,))
        result = engine.call_function(0, [])
        assert as_i32(result[0]) == 7

    def test_mul(self) -> None:
        # i32.const 5; i32.const 6; i32.mul; end
        code = bytes([0x41, 0x05, 0x41, 0x06, 0x6C, 0x0B])
        engine = _make_engine(code, results=(ValueType.I32,))
        result = engine.call_function(0, [])
        assert as_i32(result[0]) == 30

    def test_div_by_zero_traps(self) -> None:
        # i32.const 1; i32.const 0; i32.div_s; end
        code = bytes([0x41, 0x01, 0x41, 0x00, 0x6D, 0x0B])
        engine = _make_engine(code, results=(ValueType.I32,))
        with pytest.raises(TrapError, match="divide by zero"):
            engine.call_function(0, [])


class TestLocalVariables:
    """Test local.get and local.set."""

    def test_local_get_param(self) -> None:
        # local.get 0; end
        code = bytes([0x20, 0x00, 0x0B])
        engine = _make_engine(
            code,
            params=(ValueType.I32,),
            results=(ValueType.I32,),
        )
        result = engine.call_function(0, [i32(99)])
        assert as_i32(result[0]) == 99


class TestCalls:
    """Test function calls across module-defined functions."""

    def test_module_function_call_returns_result(self) -> None:
        caller = FunctionBody(locals=(), code=bytes([0x41, 0x06, 0x10, 0x01, 0x0B]))
        callee = FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x41, 0x02, 0x6C, 0x0B]))
        engine = WasmExecutionEngine(
            memory=None,
            tables=[],
            globals=[],
            global_types=[],
            func_types=[
                FuncType(params=(), results=(ValueType.I32,)),
                FuncType(params=(ValueType.I32,), results=(ValueType.I32,)),
            ],
            func_bodies=[caller, callee],
            host_functions=[None, None],
        )

        result = engine.call_function(0, [])
        assert as_i32(result[0]) == 12


class TestLinearMemory:
    """Test LinearMemory directly."""

    def test_store_load_i32(self) -> None:
        mem = LinearMemory(1)  # 1 page
        mem.store_i32(0, 42)
        assert mem.load_i32(0) == 42

    def test_out_of_bounds(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.load_i32(65536)  # Just past the end

    def test_grow(self) -> None:
        mem = LinearMemory(1, max_pages=3)
        old = mem.grow(1)
        assert old == 1
        assert mem.size() == 2


class TestTable:
    """Test Table directly."""

    def test_get_set(self) -> None:
        table = Table(4)
        table.set(0, 42)
        assert table.get(0) == 42
        assert table.get(1) is None

    def test_out_of_bounds(self) -> None:
        table = Table(2)
        with pytest.raises(TrapError, match="Out of bounds"):
            table.get(5)


class TestDefaultValue:
    """Test default value initialization."""

    def test_i32_default(self) -> None:
        v = default_value(ValueType.I32)
        assert v.type == ValueType.I32
        assert v.value == 0

    def test_unknown_type(self) -> None:
        with pytest.raises(TrapError):
            default_value(0x00)
