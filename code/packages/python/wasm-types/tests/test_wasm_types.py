"""Tests for wasm-types — WASM 1.0 type system data structures.

These tests verify:
  - Enum byte values match the WASM binary encoding specification
  - Frozen dataclasses are constructable, equal-by-value, and immutable
  - WasmModule starts empty and supports population via list mutation
  - All 21 test scenarios from the package spec
"""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from wasm_types import (
    BlockType,
    CustomSection,
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
    __version__,
)


# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------
class TestVersion:
    """Package is importable and correctly versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ---------------------------------------------------------------------------
# Test 1 — ValueType enum byte values
#
# The WASM spec assigns these exact byte values. They double as signed
# LEB128 values -1, -2, -3, -4 which distinguishes them from non-negative
# type indices in the binary format.
# ---------------------------------------------------------------------------
class TestValueType:
    """ValueType enum values match WASM binary encoding spec."""

    def test_i32_value(self) -> None:
        assert ValueType.I32 == 0x7F

    def test_i64_value(self) -> None:
        assert ValueType.I64 == 0x7E

    def test_f32_value(self) -> None:
        assert ValueType.F32 == 0x7D

    def test_f64_value(self) -> None:
        assert ValueType.F64 == 0x7C

    def test_all_four_members(self) -> None:
        """Exactly four members — no extras."""
        assert len(ValueType) == 4

    def test_is_int_enum(self) -> None:
        """ValueType is an IntEnum so members behave as ints."""
        assert int(ValueType.I32) == 127
        assert int(ValueType.F64) == 124

    def test_usable_in_bytes(self) -> None:
        """Can embed ValueType values directly in a bytes literal."""
        encoded = bytes([ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64])
        assert encoded == bytes([0x7F, 0x7E, 0x7D, 0x7C])


# ---------------------------------------------------------------------------
# Test 2 — ExternalKind enum byte values
#
# Used in Import and Export section entries to tag what kind of entity
# is being described.
# ---------------------------------------------------------------------------
class TestExternalKind:
    """ExternalKind enum values match WASM binary encoding spec."""

    def test_function_value(self) -> None:
        assert ExternalKind.FUNCTION == 0x00

    def test_table_value(self) -> None:
        assert ExternalKind.TABLE == 0x01

    def test_memory_value(self) -> None:
        assert ExternalKind.MEMORY == 0x02

    def test_global_value(self) -> None:
        assert ExternalKind.GLOBAL == 0x03

    def test_all_four_members(self) -> None:
        assert len(ExternalKind) == 4

    def test_sequential_from_zero(self) -> None:
        """Members are 0, 1, 2, 3 — matches the binary spec table."""
        values = [int(k) for k in ExternalKind]
        assert sorted(values) == [0, 1, 2, 3]


# ---------------------------------------------------------------------------
# Test 3 — BlockType.EMPTY byte value
#
# 0x40 is the single-byte encoding for "no results" in a structured control
# flow block. It is designed to be outside the ValueType range (0x7C–0x7F).
# ---------------------------------------------------------------------------
class TestBlockType:
    """BlockType.EMPTY matches WASM binary encoding."""

    def test_empty_value(self) -> None:
        assert BlockType.EMPTY == 0x40

    def test_is_int(self) -> None:
        assert int(BlockType.EMPTY) == 64

    def test_distinct_from_value_types(self) -> None:
        """EMPTY must not collide with any ValueType byte."""
        for vt in ValueType:
            assert vt != BlockType.EMPTY


# ---------------------------------------------------------------------------
# Test 4 — FuncType construction and equality
# ---------------------------------------------------------------------------
class TestFuncType:
    """FuncType can be constructed and supports structural equality."""

    def test_basic_construction(self) -> None:
        ft = FuncType(params=(ValueType.I32,), results=(ValueType.I64,))
        assert ft.params == (ValueType.I32,)
        assert ft.results == (ValueType.I64,)

    # Test 5 — empty params and results
    def test_empty_params_and_results(self) -> None:
        ft = FuncType(params=(), results=())
        assert ft.params == ()
        assert ft.results == ()

    # Test 6 — multiple params and results
    def test_multiple_params_and_results(self) -> None:
        ft = FuncType(
            params=(ValueType.I32, ValueType.I64, ValueType.F32),
            results=(ValueType.F64, ValueType.I32),
        )
        assert len(ft.params) == 3
        assert len(ft.results) == 2
        assert ft.params[0] == ValueType.I32
        assert ft.results[1] == ValueType.I32

    def test_equality_same_contents(self) -> None:
        ft1 = FuncType(params=(ValueType.I32,), results=())
        ft2 = FuncType(params=(ValueType.I32,), results=())
        assert ft1 == ft2

    def test_inequality_different_params(self) -> None:
        ft1 = FuncType(params=(ValueType.I32,), results=())
        ft2 = FuncType(params=(ValueType.I64,), results=())
        assert ft1 != ft2

    def test_hashable_as_dict_key(self) -> None:
        """FuncType is frozen so it must be hashable."""
        ft = FuncType(params=(ValueType.I32,), results=())
        d = {ft: "found"}
        assert d[ft] == "found"

    # Test 21 — frozen struct cannot be mutated
    def test_frozen_cannot_mutate_params(self) -> None:
        ft = FuncType(params=(ValueType.I32,), results=())
        with pytest.raises(FrozenInstanceError):
            ft.params = (ValueType.I64,)  # type: ignore[misc]

    def test_frozen_cannot_mutate_results(self) -> None:
        ft = FuncType(params=(), results=(ValueType.F32,))
        with pytest.raises(FrozenInstanceError):
            ft.results = ()  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Tests 7–8 — Limits
# ---------------------------------------------------------------------------
class TestLimits:
    """Limits encodes min/max constraints with optional maximum."""

    # Test 7 — only min
    def test_min_only(self) -> None:
        lim = Limits(min=1)
        assert lim.min == 1
        assert lim.max is None

    # Test 8 — min and max
    def test_min_and_max(self) -> None:
        lim = Limits(min=0, max=10)
        assert lim.min == 0
        assert lim.max == 10

    def test_equality(self) -> None:
        assert Limits(min=1, max=4) == Limits(min=1, max=4)
        assert Limits(min=1) != Limits(min=1, max=4)

    def test_frozen_cannot_mutate(self) -> None:
        lim = Limits(min=1)
        with pytest.raises(FrozenInstanceError):
            lim.min = 2  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 9 — MemoryType construction
# ---------------------------------------------------------------------------
class TestMemoryType:
    """MemoryType wraps Limits for linear memory."""

    def test_construction(self) -> None:
        mt = MemoryType(limits=Limits(min=1, max=4))
        assert mt.limits.min == 1
        assert mt.limits.max == 4

    def test_unbounded_memory(self) -> None:
        mt = MemoryType(limits=Limits(min=1))
        assert mt.limits.max is None

    def test_frozen_cannot_mutate(self) -> None:
        mt = MemoryType(limits=Limits(min=1))
        with pytest.raises(FrozenInstanceError):
            mt.limits = Limits(min=2)  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 10 — TableType construction with default element_type=0x70
# ---------------------------------------------------------------------------
class TestTableType:
    """TableType represents a WASM table of opaque references."""

    def test_default_element_type(self) -> None:
        """Default element_type must be 0x70 (funcref)."""
        tt = TableType(limits=Limits(min=0))
        assert tt.element_type == 0x70

    def test_explicit_element_type(self) -> None:
        tt = TableType(element_type=0x70, limits=Limits(min=0, max=100))
        assert tt.element_type == 0x70
        assert tt.limits.max == 100

    def test_frozen_cannot_mutate(self) -> None:
        tt = TableType(limits=Limits(min=0))
        with pytest.raises(FrozenInstanceError):
            tt.element_type = 0x6F  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 11 — GlobalType mutable and immutable
# ---------------------------------------------------------------------------
class TestGlobalType:
    """GlobalType encodes a global's value type and mutability flag."""

    def test_immutable_global(self) -> None:
        gt = GlobalType(value_type=ValueType.I32, mutable=False)
        assert gt.value_type == ValueType.I32
        assert gt.mutable is False

    def test_mutable_global(self) -> None:
        gt = GlobalType(value_type=ValueType.F64, mutable=True)
        assert gt.mutable is True

    def test_frozen_cannot_mutate(self) -> None:
        gt = GlobalType(value_type=ValueType.I32, mutable=False)
        with pytest.raises(FrozenInstanceError):
            gt.mutable = True  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 12 — Import construction for each kind
# ---------------------------------------------------------------------------
class TestImport:
    """Import entries cover all four ExternalKind values."""

    def test_function_import(self) -> None:
        imp = Import(
            module_name="wasi_snapshot_preview1",
            name="fd_write",
            kind=ExternalKind.FUNCTION,
            type_info=3,
        )
        assert imp.kind == ExternalKind.FUNCTION
        assert imp.type_info == 3

    def test_table_import(self) -> None:
        imp = Import(
            module_name="env",
            name="table",
            kind=ExternalKind.TABLE,
            type_info=TableType(limits=Limits(min=0)),
        )
        assert imp.kind == ExternalKind.TABLE
        assert isinstance(imp.type_info, TableType)

    def test_memory_import(self) -> None:
        imp = Import(
            module_name="env",
            name="memory",
            kind=ExternalKind.MEMORY,
            type_info=MemoryType(limits=Limits(min=1)),
        )
        assert imp.kind == ExternalKind.MEMORY
        assert isinstance(imp.type_info, MemoryType)

    def test_global_import(self) -> None:
        imp = Import(
            module_name="env",
            name="__stack_pointer",
            kind=ExternalKind.GLOBAL,
            type_info=GlobalType(value_type=ValueType.I32, mutable=True),
        )
        assert imp.kind == ExternalKind.GLOBAL
        assert isinstance(imp.type_info, GlobalType)
        assert imp.type_info.mutable is True

    def test_frozen_cannot_mutate(self) -> None:
        imp = Import(
            module_name="env",
            name="x",
            kind=ExternalKind.FUNCTION,
            type_info=0,
        )
        with pytest.raises(FrozenInstanceError):
            imp.name = "y"  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 13 — Export construction
# ---------------------------------------------------------------------------
class TestExport:
    """Export entries name an index and tag its kind."""

    def test_function_export(self) -> None:
        exp = Export(name="main", kind=ExternalKind.FUNCTION, index=0)
        assert exp.name == "main"
        assert exp.kind == ExternalKind.FUNCTION
        assert exp.index == 0

    def test_memory_export(self) -> None:
        exp = Export(name="memory", kind=ExternalKind.MEMORY, index=0)
        assert exp.kind == ExternalKind.MEMORY

    def test_frozen_cannot_mutate(self) -> None:
        exp = Export(name="foo", kind=ExternalKind.FUNCTION, index=1)
        with pytest.raises(FrozenInstanceError):
            exp.index = 2  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 14 — Global with init_expr bytes
# ---------------------------------------------------------------------------
class TestGlobal:
    """Global pairs a GlobalType with a constant initializer expression."""

    def test_construction(self) -> None:
        # i32.const 42 = 0x41 0x2A 0x0B (end)
        g = Global(
            global_type=GlobalType(ValueType.I32, mutable=True),
            init_expr=bytes([0x41, 0x2A, 0x0B]),
        )
        assert g.global_type.value_type == ValueType.I32
        assert g.init_expr == bytes([0x41, 0x2A, 0x0B])

    def test_immutable_global(self) -> None:
        g = Global(
            global_type=GlobalType(ValueType.F64, mutable=False),
            init_expr=bytes([0x44, 0, 0, 0, 0, 0, 0, 0, 0, 0x0B]),
        )
        assert g.global_type.mutable is False

    def test_frozen_cannot_mutate(self) -> None:
        g = Global(
            global_type=GlobalType(ValueType.I32, mutable=False),
            init_expr=bytes([0x41, 0x00, 0x0B]),
        )
        with pytest.raises(FrozenInstanceError):
            g.init_expr = b""  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 15 — Element with function_indices tuple
# ---------------------------------------------------------------------------
class TestElement:
    """Element segment fills table entries with function references."""

    def test_construction(self) -> None:
        elem = Element(
            table_index=0,
            offset_expr=bytes([0x41, 0x00, 0x0B]),  # i32.const 0; end
            function_indices=(1, 2, 3),
        )
        assert elem.table_index == 0
        assert elem.function_indices == (1, 2, 3)
        assert len(elem.function_indices) == 3

    def test_empty_indices(self) -> None:
        elem = Element(
            table_index=0,
            offset_expr=bytes([0x41, 0x00, 0x0B]),
            function_indices=(),
        )
        assert elem.function_indices == ()

    def test_frozen_cannot_mutate(self) -> None:
        elem = Element(
            table_index=0,
            offset_expr=b"",
            function_indices=(0,),
        )
        with pytest.raises(FrozenInstanceError):
            elem.table_index = 1  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 16 — DataSegment construction
# ---------------------------------------------------------------------------
class TestDataSegment:
    """DataSegment writes raw bytes into linear memory at a given offset."""

    def test_construction(self) -> None:
        ds = DataSegment(
            memory_index=0,
            offset_expr=bytes([0x41, 0x00, 0x0B]),
            data=b"hello, wasm",
        )
        assert ds.memory_index == 0
        assert ds.data == b"hello, wasm"

    def test_empty_data(self) -> None:
        ds = DataSegment(memory_index=0, offset_expr=b"", data=b"")
        assert ds.data == b""

    def test_frozen_cannot_mutate(self) -> None:
        ds = DataSegment(memory_index=0, offset_expr=b"", data=b"x")
        with pytest.raises(FrozenInstanceError):
            ds.data = b"y"  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 17 — FunctionBody with locals and code
# ---------------------------------------------------------------------------
class TestFunctionBody:
    """FunctionBody holds local variable types and raw bytecode."""

    def test_construction(self) -> None:
        fb = FunctionBody(
            locals=(ValueType.I32, ValueType.I32),
            code=bytes([0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]),
        )
        assert len(fb.locals) == 2
        assert fb.locals[0] == ValueType.I32
        assert fb.code[-1] == 0x0B  # end opcode

    def test_no_locals(self) -> None:
        fb = FunctionBody(locals=(), code=bytes([0x0B]))
        assert fb.locals == ()

    def test_frozen_cannot_mutate(self) -> None:
        fb = FunctionBody(locals=(), code=b"")
        with pytest.raises(FrozenInstanceError):
            fb.locals = (ValueType.I32,)  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Test 18 — CustomSection construction
# ---------------------------------------------------------------------------
class TestCustomSection:
    """CustomSection carries arbitrary named data outside the WASM spec."""

    def test_construction(self) -> None:
        cs = CustomSection(name="name", data=b"\x00\x04main")
        assert cs.name == "name"
        assert cs.data == b"\x00\x04main"

    def test_empty_data(self) -> None:
        cs = CustomSection(name="producers", data=b"")
        assert cs.data == b""

    def test_frozen_cannot_mutate(self) -> None:
        cs = CustomSection(name="x", data=b"")
        with pytest.raises(FrozenInstanceError):
            cs.name = "y"  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Tests 19–20 — WasmModule
# ---------------------------------------------------------------------------
class TestWasmModule:
    """WasmModule starts empty and supports incremental population."""

    # Test 19 — starts empty
    def test_all_lists_start_empty(self) -> None:
        m = WasmModule()
        assert m.types == []
        assert m.imports == []
        assert m.functions == []
        assert m.tables == []
        assert m.memories == []
        assert m.globals == []
        assert m.exports == []
        assert m.elements == []
        assert m.code == []
        assert m.data == []
        assert m.customs == []

    def test_start_is_none(self) -> None:
        m = WasmModule()
        assert m.start is None

    # Test 20 — can be populated
    def test_append_type(self) -> None:
        m = WasmModule()
        ft = FuncType(params=(ValueType.I32,), results=())
        m.types.append(ft)
        assert len(m.types) == 1
        assert m.types[0] == ft

    def test_append_import(self) -> None:
        m = WasmModule()
        imp = Import(
            module_name="env",
            name="mem",
            kind=ExternalKind.MEMORY,
            type_info=MemoryType(limits=Limits(min=1)),
        )
        m.imports.append(imp)
        assert len(m.imports) == 1

    def test_append_function_index(self) -> None:
        m = WasmModule()
        m.functions.append(0)
        m.functions.append(1)
        assert m.functions == [0, 1]

    def test_set_start(self) -> None:
        m = WasmModule()
        m.start = 5
        assert m.start == 5

    def test_full_population(self) -> None:
        """A realistic module with one of every section type."""
        m = WasmModule()

        # Type section
        ft = FuncType(params=(ValueType.I32,), results=(ValueType.I32,))
        m.types.append(ft)

        # Import section
        m.imports.append(Import(
            module_name="env", name="mem",
            kind=ExternalKind.MEMORY,
            type_info=MemoryType(limits=Limits(min=1)),
        ))

        # Function section (type index 0)
        m.functions.append(0)

        # Table section
        m.tables.append(TableType(limits=Limits(min=1, max=10)))

        # Memory section
        m.memories.append(MemoryType(limits=Limits(min=1)))

        # Global section
        m.globals.append(Global(
            global_type=GlobalType(ValueType.I32, mutable=True),
            init_expr=bytes([0x41, 0x00, 0x0B]),
        ))

        # Export section
        m.exports.append(Export(name="main", kind=ExternalKind.FUNCTION, index=1))

        # Start
        m.start = 1

        # Elements
        m.elements.append(Element(
            table_index=0,
            offset_expr=bytes([0x41, 0x00, 0x0B]),
            function_indices=(1,),
        ))

        # Code
        m.code.append(FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B])))

        # Data
        m.data.append(DataSegment(
            memory_index=0,
            offset_expr=bytes([0x41, 0x00, 0x0B]),
            data=b"hello",
        ))

        # Custom
        m.customs.append(CustomSection(name="name", data=b""))

        assert len(m.types) == 1
        assert len(m.imports) == 1
        assert len(m.functions) == 1
        assert len(m.tables) == 1
        assert len(m.memories) == 1
        assert len(m.globals) == 1
        assert len(m.exports) == 1
        assert m.start == 1
        assert len(m.elements) == 1
        assert len(m.code) == 1
        assert len(m.data) == 1
        assert len(m.customs) == 1

    def test_module_is_mutable_not_frozen(self) -> None:
        """WasmModule fields can be reassigned (it is not frozen)."""
        m = WasmModule()
        m.start = 42
        assert m.start == 42
        m.start = None
        assert m.start is None

    def test_independent_instances(self) -> None:
        """Two WasmModule instances do not share the same list objects."""
        m1 = WasmModule()
        m2 = WasmModule()
        m1.functions.append(0)
        assert m2.functions == []
