"""Tests for wasm-module-parser.

These tests verify every section type the parser handles, as well as the
most important error paths. Each test constructs a minimal binary by hand,
feeds it to WasmModuleParser, and checks the structured output.

Binary construction helpers
----------------------------
The ``make_wasm`` helper below assembles a complete .wasm binary from
(section_id, payload) pairs, writing LEB128-encoded section sizes. This
mirrors exactly what a WASM compiler produces.

Coverage targets
-----------------
- Parse: header only, each section type, multiple sections
- Errors: bad magic, wrong version, truncated header, truncated section
- Round-trip: build binary → parse → verify all fields
"""

from __future__ import annotations

import pytest
from wasm_types import (
    ExternalKind,
    FuncType,
    GlobalType,
    Limits,
    MemoryType,
    TableType,
    ValueType,
    WasmModule,
)

from wasm_module_parser import WasmModuleParser, WasmParseError, __version__

# ---------------------------------------------------------------------------
# BINARY CONSTRUCTION HELPERS
#
# These helpers let tests express WASM binaries in a human-readable way:
#
#   make_wasm([
#       (1, type_section_payload),
#       (7, export_section_payload),
#   ])
#
# This produces the 8-byte header followed by section envelopes.
# ---------------------------------------------------------------------------

WASM_HEADER: bytes = b"\x00asm\x01\x00\x00\x00"


def _leb128(value: int) -> bytes:
    """Encode a non-negative integer as unsigned LEB128.

    This duplicates wasm_leb128.encode_unsigned inline so tests don't need
    to import a second package just for binary construction.

    Example:
        _leb128(0)    → b'\\x00'
        _leb128(128)  → b'\\x80\\x01'
        _leb128(300)  → b'\\xac\\x02'
    """
    out: list[int] = []
    while True:
        payload = value & 0x7F
        value >>= 7
        if value != 0:
            out.append(payload | 0x80)
        else:
            out.append(payload)
            break
    return bytes(out)


def make_wasm(sections: list[tuple[int, bytes]]) -> bytes:
    """Build a minimal valid .wasm binary from (section_id, payload) pairs.

    Args:
        sections: List of (section_id, payload_bytes) tuples. The size field
                  is computed automatically from len(payload_bytes).

    Returns:
        Complete .wasm binary starting with the 8-byte header.

    Example:
        # Module with one export:
        payload = b'\\x01\\x04main\\x00\\x00'
        wasm = make_wasm([(7, payload)])
    """
    body = b""
    for section_id, payload in sections:
        body += bytes([section_id]) + _leb128(len(payload)) + payload
    return WASM_HEADER + body


def _make_functype(params: list[int], results: list[int]) -> bytes:
    """Encode one FuncType entry as WASM bytes (0x60 + params + results)."""
    return (
        b"\x60"
        + _leb128(len(params))
        + bytes(params)
        + _leb128(len(results))
        + bytes(results)
    )


def _make_name(s: str) -> bytes:
    """Encode a WASM name (LEB128 length + UTF-8 bytes)."""
    encoded = s.encode("utf-8")
    return _leb128(len(encoded)) + encoded


def _make_limits(min_val: int, max_val: int | None = None) -> bytes:
    """Encode a WASM Limits (flags + min [+ max])."""
    if max_val is None:
        return b"\x00" + _leb128(min_val)
    return b"\x01" + _leb128(min_val) + _leb128(max_val)


def _make_init_expr_i32(n: int) -> bytes:
    """Encode an i32.const N init_expr (0x41 <leb128> 0x0B)."""
    # LEB128-encode the signed value (for i32.const; simple for positive n)
    return b"\x41" + _leb128(n) + b"\x0B"


# ---------------------------------------------------------------------------
# TEST CLASSES
# ---------------------------------------------------------------------------


class TestVersion:
    """Package is importable and has the expected version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestParserCreation:
    """WasmModuleParser can be instantiated."""

    def test_create_parser(self) -> None:
        parser = WasmModuleParser()
        assert parser is not None


# ---------------------------------------------------------------------------
# Test 1: Minimal module (header only)
# ---------------------------------------------------------------------------

class TestMinimalModule:
    """Parse a module that contains only the 8-byte header.

    A minimal valid WASM module is just:
        0x00 0x61 0x73 0x6D  — magic
        0x01 0x00 0x00 0x00  — version

    No sections. All WasmModule fields should be empty/None.
    """

    def test_parse_minimal(self) -> None:
        module = WasmModuleParser().parse(WASM_HEADER)
        assert isinstance(module, WasmModule)
        assert module.types == []
        assert module.imports == []
        assert module.functions == []
        assert module.tables == []
        assert module.memories == []
        assert module.globals == []
        assert module.exports == []
        assert module.start is None
        assert module.elements == []
        assert module.code == []
        assert module.data == []
        assert module.customs == []


# ---------------------------------------------------------------------------
# Test 2: Type section — (i32, i32) → i32
# ---------------------------------------------------------------------------

class TestTypeSection:
    """Parse a module containing a single function type: (i32, i32) → i32.

    Binary layout of the type section payload:
        0x01                — 1 type entry
        0x60                — functype marker
        0x02                — 2 params
        0x7F 0x7F           — i32, i32
        0x01                — 1 result
        0x7F                — i32
    """

    def test_parse_type_i32_i32_to_i32(self) -> None:
        type_payload = (
            _leb128(1)
            + _make_functype([0x7F, 0x7F], [0x7F])
        )
        module = WasmModuleParser().parse(make_wasm([(1, type_payload)]))
        assert len(module.types) == 1
        ft = module.types[0]
        assert ft.params == (ValueType.I32, ValueType.I32)
        assert ft.results == (ValueType.I32,)

    def test_parse_type_empty_signature(self) -> None:
        """Void → void function type."""
        type_payload = _leb128(1) + _make_functype([], [])
        module = WasmModuleParser().parse(make_wasm([(1, type_payload)]))
        assert len(module.types) == 1
        ft = module.types[0]
        assert ft.params == ()
        assert ft.results == ()

    def test_parse_multiple_types(self) -> None:
        """Two types: (i32) → i64 and (f32, f64) → ()."""
        type_payload = (
            _leb128(2)
            + _make_functype([0x7F], [0x7E])       # (i32) → i64
            + _make_functype([0x7D, 0x7C], [])     # (f32, f64) → ()
        )
        module = WasmModuleParser().parse(make_wasm([(1, type_payload)]))
        assert len(module.types) == 2
        assert module.types[0].params == (ValueType.I32,)
        assert module.types[0].results == (ValueType.I64,)
        assert module.types[1].params == (ValueType.F32, ValueType.F64)
        assert module.types[1].results == ()


# ---------------------------------------------------------------------------
# Test 3: Function section
# ---------------------------------------------------------------------------

class TestFunctionSection:
    """Parse a module with a Function section (type indices for local funcs)."""

    def test_parse_function_section(self) -> None:
        """One function referencing type index 0."""
        func_payload = _leb128(1) + _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(3, func_payload)]))
        assert module.functions == [0]

    def test_parse_multiple_functions(self) -> None:
        func_payload = _leb128(3) + _leb128(0) + _leb128(1) + _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(3, func_payload)]))
        assert module.functions == [0, 1, 0]


# ---------------------------------------------------------------------------
# Test 4: Export section
# ---------------------------------------------------------------------------

class TestExportSection:
    """Parse a module with an Export section."""

    def test_parse_export_function(self) -> None:
        """Export a function named 'main' at index 0."""
        export_payload = (
            _leb128(1)
            + _make_name("main")
            + b"\x00"       # ExternalKind.FUNCTION
            + _leb128(0)    # index 0
        )
        module = WasmModuleParser().parse(make_wasm([(7, export_payload)]))
        assert len(module.exports) == 1
        exp = module.exports[0]
        assert exp.name == "main"
        assert exp.kind == ExternalKind.FUNCTION
        assert exp.index == 0

    def test_parse_export_memory(self) -> None:
        """Export memory named 'memory' at index 0."""
        export_payload = (
            _leb128(1)
            + _make_name("memory")
            + b"\x02"       # ExternalKind.MEMORY
            + _leb128(0)
        )
        module = WasmModuleParser().parse(make_wasm([(7, export_payload)]))
        assert len(module.exports) == 1
        exp = module.exports[0]
        assert exp.name == "memory"
        assert exp.kind == ExternalKind.MEMORY

    def test_parse_multiple_exports(self) -> None:
        """Two exports: function and global."""
        export_payload = (
            _leb128(2)
            + _make_name("add")  + b"\x00" + _leb128(0)
            + _make_name("g")    + b"\x03" + _leb128(1)
        )
        module = WasmModuleParser().parse(make_wasm([(7, export_payload)]))
        assert len(module.exports) == 2
        assert module.exports[0].kind == ExternalKind.FUNCTION
        assert module.exports[1].kind == ExternalKind.GLOBAL


# ---------------------------------------------------------------------------
# Test 5: Code section
# ---------------------------------------------------------------------------

class TestCodeSection:
    """Parse a module with a Code section containing a simple function body."""

    def test_parse_code_no_locals(self) -> None:
        """Function body with no locals and trivial bytecode: just 0x0B (end)."""
        #  body_size:        2 bytes (1 for local_decl_count + 1 for end)
        #  local_decl_count: 0
        #  code:             0x0B
        body = _leb128(0) + b"\x0B"   # 0 local decls + end
        code_payload = _leb128(1) + _leb128(len(body)) + body
        module = WasmModuleParser().parse(make_wasm([(10, code_payload)]))
        assert len(module.code) == 1
        fb = module.code[0]
        assert fb.locals == ()
        assert fb.code == b"\x0B"

    def test_parse_code_with_locals(self) -> None:
        """Function body with two i32 locals and an add instruction.

        Body encoding:
            local_decl_count: 1  (one group)
            local_decl:       (2, i32)  — 2 i32 locals
            code:             0x20 0x00   local.get 0
                              0x20 0x01   local.get 1
                              0x6A        i32.add
                              0x0B        end
        """
        code_bytes = bytes([0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B])
        # 1 local decl group: 2 × i32
        locals_enc = _leb128(1) + _leb128(2) + b"\x7F"
        body = locals_enc + code_bytes
        code_payload = _leb128(1) + _leb128(len(body)) + body
        module = WasmModuleParser().parse(make_wasm([(10, code_payload)]))
        assert len(module.code) == 1
        fb = module.code[0]
        assert fb.locals == (ValueType.I32, ValueType.I32)
        assert fb.code == code_bytes

    def test_parse_code_multiple_local_groups(self) -> None:
        """Function body with two local-decl groups: (1, i32) and (1, f64)."""
        code_bytes = b"\x0B"
        locals_enc = _leb128(2) + _leb128(1) + b"\x7F" + _leb128(1) + b"\x7C"
        body = locals_enc + code_bytes
        code_payload = _leb128(1) + _leb128(len(body)) + body
        module = WasmModuleParser().parse(make_wasm([(10, code_payload)]))
        fb = module.code[0]
        assert fb.locals == (ValueType.I32, ValueType.F64)


# ---------------------------------------------------------------------------
# Test 6: Import section
# ---------------------------------------------------------------------------

class TestImportSection:
    """Parse a module with an Import section."""

    def test_parse_function_import(self) -> None:
        """Import a function: env::add with type index 0."""
        imp_payload = (
            _leb128(1)
            + _make_name("env")
            + _make_name("add")
            + b"\x00"        # ExternalKind.FUNCTION
            + _leb128(0)     # type index 0
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        assert len(module.imports) == 1
        imp = module.imports[0]
        assert imp.module_name == "env"
        assert imp.name == "add"
        assert imp.kind == ExternalKind.FUNCTION
        assert imp.type_info == 0

    def test_parse_memory_import(self) -> None:
        """Import a memory with min=1, no max."""
        imp_payload = (
            _leb128(1)
            + _make_name("env")
            + _make_name("memory")
            + b"\x02"               # ExternalKind.MEMORY
            + _make_limits(1)       # limits: no max, min=1
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        imp = module.imports[0]
        assert imp.kind == ExternalKind.MEMORY
        assert isinstance(imp.type_info, MemoryType)
        assert imp.type_info.limits.min == 1
        assert imp.type_info.limits.max is None

    def test_parse_global_import(self) -> None:
        """Import an immutable i32 global."""
        imp_payload = (
            _leb128(1)
            + _make_name("env")
            + _make_name("g")
            + b"\x03"    # ExternalKind.GLOBAL
            + b"\x7F"    # ValueType.I32
            + b"\x00"    # immutable
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        imp = module.imports[0]
        assert imp.kind == ExternalKind.GLOBAL
        assert isinstance(imp.type_info, GlobalType)
        assert imp.type_info.value_type == ValueType.I32
        assert imp.type_info.mutable is False

    def test_parse_table_import(self) -> None:
        """Import a table with funcref element type and min=0."""
        imp_payload = (
            _leb128(1)
            + _make_name("env")
            + _make_name("tbl")
            + b"\x01"           # ExternalKind.TABLE
            + b"\x70"           # element_type = funcref
            + _make_limits(0)   # min=0, no max
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        imp = module.imports[0]
        assert imp.kind == ExternalKind.TABLE
        assert isinstance(imp.type_info, TableType)
        assert imp.type_info.element_type == 0x70
        assert imp.type_info.limits.min == 0


# ---------------------------------------------------------------------------
# Test 7: Memory section
# ---------------------------------------------------------------------------

class TestMemorySection:
    """Parse a module with a Memory section."""

    def test_parse_memory_no_max(self) -> None:
        """One memory with min=1, no max."""
        mem_payload = _leb128(1) + _make_limits(1)
        module = WasmModuleParser().parse(make_wasm([(5, mem_payload)]))
        assert len(module.memories) == 1
        mem = module.memories[0]
        assert mem.limits.min == 1
        assert mem.limits.max is None

    def test_parse_memory_with_max(self) -> None:
        """One memory with min=1, max=4."""
        mem_payload = _leb128(1) + _make_limits(1, 4)
        module = WasmModuleParser().parse(make_wasm([(5, mem_payload)]))
        mem = module.memories[0]
        assert mem.limits.min == 1
        assert mem.limits.max == 4


# ---------------------------------------------------------------------------
# Test 8: Table section
# ---------------------------------------------------------------------------

class TestTableSection:
    """Parse a module with a Table section."""

    def test_parse_table(self) -> None:
        """One funcref table with min=0, max=10."""
        tbl_payload = (
            _leb128(1)
            + b"\x70"           # element_type = funcref
            + _make_limits(0, 10)
        )
        module = WasmModuleParser().parse(make_wasm([(4, tbl_payload)]))
        assert len(module.tables) == 1
        tbl = module.tables[0]
        assert tbl.element_type == 0x70
        assert tbl.limits.min == 0
        assert tbl.limits.max == 10


# ---------------------------------------------------------------------------
# Test 9: Global section
# ---------------------------------------------------------------------------

class TestGlobalSection:
    """Parse a module with a Global section."""

    def test_parse_global_const_i32(self) -> None:
        """Immutable i32 global initialized to 42 via i32.const 42."""
        # init_expr: 0x41 0x2A 0x0B  (i32.const 42; end)
        glob_payload = (
            _leb128(1)
            + b"\x7F"   # ValueType.I32
            + b"\x00"   # immutable
            + b"\x41\x2A\x0B"  # i32.const 42; end
        )
        module = WasmModuleParser().parse(make_wasm([(6, glob_payload)]))
        assert len(module.globals) == 1
        g = module.globals[0]
        assert g.global_type.value_type == ValueType.I32
        assert g.global_type.mutable is False
        assert g.init_expr == b"\x41\x2A\x0B"

    def test_parse_global_mutable_i32(self) -> None:
        """Mutable i32 global initialized to 0."""
        glob_payload = (
            _leb128(1)
            + b"\x7F"   # ValueType.I32
            + b"\x01"   # mutable
            + b"\x41\x00\x0B"  # i32.const 0; end
        )
        module = WasmModuleParser().parse(make_wasm([(6, glob_payload)]))
        g = module.globals[0]
        assert g.global_type.mutable is True


# ---------------------------------------------------------------------------
# Test 10: Data section
# ---------------------------------------------------------------------------

class TestDataSection:
    """Parse a module with a Data section."""

    def test_parse_data_segment(self) -> None:
        """One data segment writing b'hello' at memory offset 0."""
        data_bytes = b"hello"
        data_payload = (
            _leb128(1)
            + _leb128(0)                   # memory_index = 0
            + _make_init_expr_i32(0)       # offset = 0
            + _leb128(len(data_bytes))
            + data_bytes
        )
        module = WasmModuleParser().parse(make_wasm([(11, data_payload)]))
        assert len(module.data) == 1
        ds = module.data[0]
        assert ds.memory_index == 0
        assert ds.data == b"hello"
        assert ds.offset_expr == b"\x41\x00\x0B"

    def test_parse_data_segment_nonzero_offset(self) -> None:
        """Data segment at offset 256 (0x41 0x80 0x02 0x0B for i32.const 256)."""
        # i32.const 256: 0x41 + LEB128(256) = 0x41 0x80 0x02 + 0x0B
        data_bytes = b"world"
        data_payload = (
            _leb128(1)
            + _leb128(0)
            + b"\x41\x80\x02\x0B"   # i32.const 256; end
            + _leb128(len(data_bytes))
            + data_bytes
        )
        module = WasmModuleParser().parse(make_wasm([(11, data_payload)]))
        ds = module.data[0]
        assert ds.data == b"world"


# ---------------------------------------------------------------------------
# Test 11: Element section
# ---------------------------------------------------------------------------

class TestElementSection:
    """Parse a module with an Element section."""

    def test_parse_element_segment(self) -> None:
        """One element segment: table 0, offset 0, functions [1, 2, 3]."""
        elem_payload = (
            _leb128(1)
            + _leb128(0)                  # table_index = 0
            + _make_init_expr_i32(0)      # offset = 0
            + _leb128(3)                  # 3 function indices
            + _leb128(1) + _leb128(2) + _leb128(3)
        )
        module = WasmModuleParser().parse(make_wasm([(9, elem_payload)]))
        assert len(module.elements) == 1
        elem = module.elements[0]
        assert elem.table_index == 0
        assert elem.offset_expr == b"\x41\x00\x0B"
        assert elem.function_indices == (1, 2, 3)


# ---------------------------------------------------------------------------
# Test 12: Start section
# ---------------------------------------------------------------------------

class TestStartSection:
    """Parse a module with a Start section."""

    def test_parse_start_section(self) -> None:
        """Start function at index 5."""
        start_payload = _leb128(5)
        module = WasmModuleParser().parse(make_wasm([(8, start_payload)]))
        assert module.start == 5

    def test_no_start_section(self) -> None:
        """Module without a Start section has start=None."""
        module = WasmModuleParser().parse(WASM_HEADER)
        assert module.start is None


# ---------------------------------------------------------------------------
# Test 13: Custom section
# ---------------------------------------------------------------------------

class TestCustomSection:
    """Parse a module with one or more Custom sections."""

    def test_parse_custom_section(self) -> None:
        """Custom section named 'name' with payload b'\\x00\\x04main'."""
        name_bytes = _make_name("name")
        data = b"\x00\x04main"
        custom_payload = name_bytes + data
        module = WasmModuleParser().parse(make_wasm([(0, custom_payload)]))
        assert len(module.customs) == 1
        cs = module.customs[0]
        assert cs.name == "name"
        assert cs.data == data

    def test_custom_section_before_type(self) -> None:
        """Custom sections may appear before standard sections."""
        custom_payload = _make_name("pre") + b"\xDE\xAD"
        type_payload = _leb128(0)   # 0 types
        module = WasmModuleParser().parse(
            make_wasm([(0, custom_payload), (1, type_payload)])
        )
        assert len(module.customs) == 1
        assert module.customs[0].name == "pre"
        assert module.types == []

    def test_multiple_custom_sections(self) -> None:
        """Two custom sections are both collected."""
        c1 = _make_name("a") + b"\x01"
        c2 = _make_name("b") + b"\x02\x03"
        module = WasmModuleParser().parse(make_wasm([(0, c1), (0, c2)]))
        assert len(module.customs) == 2
        assert module.customs[0].name == "a"
        assert module.customs[1].name == "b"


# ---------------------------------------------------------------------------
# Test 14: Multiple sections combined
# ---------------------------------------------------------------------------

class TestMultipleSections:
    """Parse a module with several sections at once."""

    def test_type_plus_function_plus_export(self) -> None:
        """A module with one function: (i32, i32) → i32, exported as 'add'."""
        type_payload = _leb128(1) + _make_functype([0x7F, 0x7F], [0x7F])
        func_payload = _leb128(1) + _leb128(0)
        export_payload = _leb128(1) + _make_name("add") + b"\x00" + _leb128(0)
        code_bytes = b"\x0B"
        body = _leb128(0) + code_bytes
        code_payload = _leb128(1) + _leb128(len(body)) + body

        module = WasmModuleParser().parse(
            make_wasm([
                (1, type_payload),
                (3, func_payload),
                (7, export_payload),
                (10, code_payload),
            ])
        )
        assert len(module.types) == 1
        assert module.functions == [0]
        assert len(module.exports) == 1
        assert module.exports[0].name == "add"
        assert len(module.code) == 1


# ---------------------------------------------------------------------------
# Test 15: Error — bad magic bytes
# ---------------------------------------------------------------------------

class TestBadMagic:
    """Raise WasmParseError on wrong magic bytes."""

    def test_bad_magic(self) -> None:
        bad = b"WASM\x01\x00\x00\x00"
        with pytest.raises(WasmParseError) as exc_info:
            WasmModuleParser().parse(bad)
        err = exc_info.value
        assert err.offset == 0
        assert "magic" in err.message.lower()

    def test_empty_input(self) -> None:
        with pytest.raises(WasmParseError) as exc_info:
            WasmModuleParser().parse(b"")
        assert exc_info.value.offset == 0


# ---------------------------------------------------------------------------
# Test 16: Error — wrong version
# ---------------------------------------------------------------------------

class TestWrongVersion:
    """Raise WasmParseError on wrong version field."""

    def test_wrong_version(self) -> None:
        bad = b"\x00asm\x02\x00\x00\x00"   # version 2 instead of 1
        with pytest.raises(WasmParseError) as exc_info:
            WasmModuleParser().parse(bad)
        err = exc_info.value
        assert err.offset == 4
        assert "version" in err.message.lower()


# ---------------------------------------------------------------------------
# Test 17: Error — truncated header
# ---------------------------------------------------------------------------

class TestTruncatedHeader:
    """Raise WasmParseError on header shorter than 8 bytes."""

    def test_truncated_header_4_bytes(self) -> None:
        with pytest.raises(WasmParseError) as exc_info:
            WasmModuleParser().parse(b"\x00asm")
        assert exc_info.value.offset == 0

    def test_truncated_header_0_bytes(self) -> None:
        with pytest.raises(WasmParseError):
            WasmModuleParser().parse(b"")

    def test_truncated_header_7_bytes(self) -> None:
        with pytest.raises(WasmParseError):
            WasmModuleParser().parse(b"\x00asm\x01\x00\x00")


# ---------------------------------------------------------------------------
# Test 18: Error — truncated section payload
# ---------------------------------------------------------------------------

class TestTruncatedSectionPayload:
    """Raise WasmParseError when a section payload is shorter than declared."""

    def test_truncated_type_section(self) -> None:
        """Section declares 10 bytes but only 2 are present."""
        # Section header: id=1, size=10, but only 2 bytes of payload follow.
        truncated = WASM_HEADER + bytes([1]) + _leb128(10) + b"\x01\x60"
        with pytest.raises(WasmParseError):
            WasmModuleParser().parse(truncated)

    def test_truncated_section_size_field(self) -> None:
        """Section ID is present but the LEB128 size is truncated."""
        # A LEB128 with continuation bit set and no following byte.
        truncated = WASM_HEADER + bytes([1, 0x80])
        with pytest.raises(WasmParseError):
            WasmModuleParser().parse(truncated)


# ---------------------------------------------------------------------------
# Test 19: Round-trip — manually build binary and verify all fields
# ---------------------------------------------------------------------------

class TestRoundTrip:
    """Build a complete multi-section module, parse it, verify every field."""

    def test_full_round_trip(self) -> None:
        """
        Construct a module with:
          - 1 type: (i32, i32) → i32
          - 1 import: env::log (func, type 0)
          - 1 local function: type 0
          - 1 table: funcref, min=0, max=5
          - 1 memory: min=1, max=2
          - 1 global: immutable i32 = 0
          - 1 export: 'add' → function 1 (import is index 0, local is index 1)
          - 1 start: function index 0
          - 1 element: table 0, offset 0, [1]
          - 1 code body: no locals, just 0x0B
          - 1 data: memory 0, offset 0, b'hi'
          - 1 custom: name='test', data=b'\\xCA\\xFE'

        Then parse and verify every field.
        """
        # Type section: (i32, i32) → i32
        type_payload = _leb128(1) + _make_functype([0x7F, 0x7F], [0x7F])

        # Import section: env::log, func, type 0
        import_payload = (
            _leb128(1)
            + _make_name("env") + _make_name("log")
            + b"\x00" + _leb128(0)
        )

        # Function section: local function uses type 0
        func_payload = _leb128(1) + _leb128(0)

        # Table section: funcref, min=0, max=5
        table_payload = _leb128(1) + b"\x70" + _make_limits(0, 5)

        # Memory section: min=1, max=2
        mem_payload = _leb128(1) + _make_limits(1, 2)

        # Global section: immutable i32 = 0
        glob_payload = (
            _leb128(1)
            + b"\x7F\x00"              # i32, immutable
            + b"\x41\x00\x0B"         # i32.const 0; end
        )

        # Export section: add → function 1
        export_payload = (
            _leb128(1)
            + _make_name("add") + b"\x00" + _leb128(1)
        )

        # Start section: function 0
        start_payload = _leb128(0)

        # Element section: table 0, offset 0, [1]
        elem_payload = (
            _leb128(1) + _leb128(0)
            + b"\x41\x00\x0B"
            + _leb128(1) + _leb128(1)
        )

        # Code section: one body, no locals, end
        body = _leb128(0) + b"\x0B"
        code_payload = _leb128(1) + _leb128(len(body)) + body

        # Data section: memory 0, offset 0, b'hi'
        data_payload = (
            _leb128(1) + _leb128(0)
            + b"\x41\x00\x0B"
            + _leb128(2) + b"hi"
        )

        # Custom section: name='test', data=b'\xCA\xFE'
        custom_payload = _make_name("test") + b"\xCA\xFE"

        binary = make_wasm([
            (1, type_payload),
            (2, import_payload),
            (3, func_payload),
            (4, table_payload),
            (5, mem_payload),
            (6, glob_payload),
            (7, export_payload),
            (8, start_payload),
            (9, elem_payload),
            (10, code_payload),
            (11, data_payload),
            (0, custom_payload),   # custom after data is valid
        ])

        module = WasmModuleParser().parse(binary)

        # Verify types
        assert len(module.types) == 1
        assert module.types[0] == FuncType(
            params=(ValueType.I32, ValueType.I32),
            results=(ValueType.I32,),
        )

        # Verify imports
        assert len(module.imports) == 1
        imp = module.imports[0]
        assert imp.module_name == "env"
        assert imp.name == "log"
        assert imp.kind == ExternalKind.FUNCTION
        assert imp.type_info == 0

        # Verify functions
        assert module.functions == [0]

        # Verify tables
        assert len(module.tables) == 1
        tbl = module.tables[0]
        assert tbl.element_type == 0x70
        assert tbl.limits == Limits(min=0, max=5)

        # Verify memories
        assert len(module.memories) == 1
        assert module.memories[0].limits == Limits(min=1, max=2)

        # Verify globals
        assert len(module.globals) == 1
        g = module.globals[0]
        assert g.global_type == GlobalType(value_type=ValueType.I32, mutable=False)
        assert g.init_expr == b"\x41\x00\x0B"

        # Verify exports
        assert len(module.exports) == 1
        exp = module.exports[0]
        assert exp.name == "add"
        assert exp.kind == ExternalKind.FUNCTION
        assert exp.index == 1

        # Verify start
        assert module.start == 0

        # Verify elements
        assert len(module.elements) == 1
        elem = module.elements[0]
        assert elem.table_index == 0
        assert elem.function_indices == (1,)

        # Verify code
        assert len(module.code) == 1
        fb = module.code[0]
        assert fb.locals == ()
        assert fb.code == b"\x0B"

        # Verify data
        assert len(module.data) == 1
        ds = module.data[0]
        assert ds.memory_index == 0
        assert ds.data == b"hi"

        # Verify customs
        assert len(module.customs) == 1
        cs = module.customs[0]
        assert cs.name == "test"
        assert cs.data == b"\xCA\xFE"


# ---------------------------------------------------------------------------
# Additional edge cases for coverage
# ---------------------------------------------------------------------------

class TestEdgeCases:
    """Additional edge cases to push coverage above 90%."""

    def test_empty_type_section(self) -> None:
        """Type section with count=0 is valid (no types)."""
        type_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(1, type_payload)]))
        assert module.types == []

    def test_empty_function_section(self) -> None:
        func_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(3, func_payload)]))
        assert module.functions == []

    def test_empty_code_section(self) -> None:
        code_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(10, code_payload)]))
        assert module.code == []

    def test_empty_data_section(self) -> None:
        data_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(11, data_payload)]))
        assert module.data == []

    def test_empty_element_section(self) -> None:
        elem_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(9, elem_payload)]))
        assert module.elements == []

    def test_empty_import_section(self) -> None:
        imp_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        assert module.imports == []

    def test_empty_export_section(self) -> None:
        exp_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(7, exp_payload)]))
        assert module.exports == []

    def test_empty_table_section(self) -> None:
        tbl_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(4, tbl_payload)]))
        assert module.tables == []

    def test_empty_memory_section(self) -> None:
        mem_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(5, mem_payload)]))
        assert module.memories == []

    def test_empty_global_section(self) -> None:
        glob_payload = _leb128(0)
        module = WasmModuleParser().parse(make_wasm([(6, glob_payload)]))
        assert module.globals == []

    def test_parse_error_has_message_and_offset(self) -> None:
        """WasmParseError carries both message and offset attributes."""
        try:
            WasmModuleParser().parse(b"NOTW")
        except WasmParseError as e:
            assert isinstance(e.message, str)
            assert isinstance(e.offset, int)
            assert len(e.message) > 0

    def test_wasm_parse_error_str(self) -> None:
        """WasmParseError.__str__ returns the message."""
        err = WasmParseError("bad bytes", 42)
        assert str(err) == "bad bytes"
        assert err.offset == 42

    def test_type_section_bad_marker(self) -> None:
        """Type section entry that starts with 0x40 (not 0x60) is an error."""
        type_payload = _leb128(1) + b"\x40\x00\x00"   # bad marker
        with pytest.raises(WasmParseError):
            WasmModuleParser().parse(make_wasm([(1, type_payload)]))

    def test_large_leb128_index(self) -> None:
        """Function section with a multi-byte LEB128 type index (128)."""
        func_payload = _leb128(1) + _leb128(128)
        module = WasmModuleParser().parse(make_wasm([(3, func_payload)]))
        assert module.functions == [128]

    def test_global_import_mutable(self) -> None:
        """Import a mutable i64 global."""
        imp_payload = (
            _leb128(1)
            + _make_name("env") + _make_name("gv")
            + b"\x03"    # ExternalKind.GLOBAL
            + b"\x7E"    # ValueType.I64
            + b"\x01"    # mutable
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        gt = module.imports[0].type_info
        assert isinstance(gt, GlobalType)
        assert gt.value_type == ValueType.I64
        assert gt.mutable is True

    def test_custom_section_empty_data(self) -> None:
        """Custom section with a name but no trailing data bytes."""
        custom_payload = _make_name("empty")   # no data bytes
        module = WasmModuleParser().parse(make_wasm([(0, custom_payload)]))
        cs = module.customs[0]
        assert cs.name == "empty"
        assert cs.data == b""

    def test_memory_with_max_limits(self) -> None:
        """Memory import with explicit max reads max correctly."""
        imp_payload = (
            _leb128(1)
            + _make_name("env") + _make_name("mem")
            + b"\x02"
            + _make_limits(2, 8)
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        mt = module.imports[0].type_info
        assert isinstance(mt, MemoryType)
        assert mt.limits.min == 2
        assert mt.limits.max == 8

    def test_table_import_no_max(self) -> None:
        """Table import without max."""
        imp_payload = (
            _leb128(1)
            + _make_name("env") + _make_name("t")
            + b"\x01"           # ExternalKind.TABLE
            + b"\x70"
            + _make_limits(1)   # min=1, no max
        )
        module = WasmModuleParser().parse(make_wasm([(2, imp_payload)]))
        tt = module.imports[0].type_info
        assert isinstance(tt, TableType)
        assert tt.limits.min == 1
        assert tt.limits.max is None
