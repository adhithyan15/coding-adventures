"""test_validator.py --- Tests for WASM module structural validation.

Covers: valid module, memory limit checks, duplicate exports, bad type indices,
start function validation, table limits, element segment validation, and
the public validate() API.
"""

from __future__ import annotations

import pytest
from wasm_types import (
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

from wasm_validator import ValidatedModule, validate
from wasm_validator.validator import (
    ValidationError,
    ValidationErrorKind,
    validate_structure,
)


# ===========================================================================
# Helper: build a minimal valid module
# ===========================================================================


def _minimal_module(**overrides) -> WasmModule:
    """Build a minimal valid WasmModule with overrides for specific sections."""
    defaults = dict(
        types=[FuncType(params=(ValueType.I32,), results=(ValueType.I32,))],
        imports=[],
        functions=[0],
        tables=[],
        memories=[],
        globals=[],
        exports=[
            Export(name="func", kind=ExternalKind.FUNCTION, index=0),
        ],
        start=None,
        elements=[],
        code=[FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B]))],
        data=[],
        customs=[],
    )
    defaults.update(overrides)
    return WasmModule(**defaults)


# ===========================================================================
# Valid module
# ===========================================================================


class TestValidModule:
    def test_minimal_valid(self) -> None:
        """A minimal module with one function should validate."""
        module = _minimal_module()
        result = validate(module)
        assert isinstance(result, ValidatedModule)
        assert len(result.func_types) == 1

    def test_valid_with_memory(self) -> None:
        module = _minimal_module(memories=[MemoryType(limits=Limits(min=1, max=10))])
        result = validate(module)
        assert result is not None

    def test_valid_with_table(self) -> None:
        module = _minimal_module(tables=[TableType(element_type=0x70, limits=Limits(min=1, max=10))])
        result = validate(module)
        assert result is not None

    def test_validated_module_has_func_locals(self) -> None:
        module = _minimal_module()
        result = validate(module)
        assert len(result.func_locals) == 1
        # Locals = params + declared locals
        assert ValueType.I32 in result.func_locals[0]


# ===========================================================================
# Memory limit validation
# ===========================================================================


class TestMemoryLimits:
    def test_memory_max_exceeded(self) -> None:
        module = _minimal_module(
            memories=[MemoryType(limits=Limits(min=1, max=100000))]
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.MEMORY_LIMIT_EXCEEDED

    def test_memory_min_exceeds_max(self) -> None:
        module = _minimal_module(
            memories=[MemoryType(limits=Limits(min=10, max=5))]
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.MEMORY_LIMIT_ORDER

    def test_memory_no_max_is_valid(self) -> None:
        module = _minimal_module(
            memories=[MemoryType(limits=Limits(min=1, max=None))]
        )
        result = validate(module)
        assert result is not None


# ===========================================================================
# Multiple memories/tables (WASM 1.0 restriction)
# ===========================================================================


class TestMultipleEntities:
    def test_multiple_memories(self) -> None:
        module = _minimal_module(memories=[
            MemoryType(limits=Limits(min=1, max=None)),
            MemoryType(limits=Limits(min=1, max=None)),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.MULTIPLE_MEMORIES

    def test_multiple_tables(self) -> None:
        module = _minimal_module(tables=[
            TableType(element_type=0x70, limits=Limits(min=1, max=None)),
            TableType(element_type=0x70, limits=Limits(min=1, max=None)),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.MULTIPLE_TABLES


# ===========================================================================
# Table limit validation
# ===========================================================================


class TestTableLimits:
    def test_table_min_exceeds_max(self) -> None:
        module = _minimal_module(
            tables=[TableType(element_type=0x70, limits=Limits(min=10, max=5))]
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.TABLE_LIMIT_ORDER


# ===========================================================================
# Export validation
# ===========================================================================


class TestExportValidation:
    def test_duplicate_export_name(self) -> None:
        module = _minimal_module(exports=[
            Export(name="func", kind=ExternalKind.FUNCTION, index=0),
            Export(name="func", kind=ExternalKind.FUNCTION, index=0),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.DUPLICATE_EXPORT_NAME

    def test_export_index_out_of_range(self) -> None:
        module = _minimal_module(exports=[
            Export(name="missing", kind=ExternalKind.FUNCTION, index=99),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE

    def test_export_table_out_of_range(self) -> None:
        module = _minimal_module(exports=[
            Export(name="tbl", kind=ExternalKind.TABLE, index=0),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE

    def test_export_memory_out_of_range(self) -> None:
        module = _minimal_module(exports=[
            Export(name="mem", kind=ExternalKind.MEMORY, index=0),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE

    def test_export_global_out_of_range(self) -> None:
        module = _minimal_module(exports=[
            Export(name="glob", kind=ExternalKind.GLOBAL, index=0),
        ])
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE


# ===========================================================================
# Type index validation
# ===========================================================================


class TestTypeIndexValidation:
    def test_invalid_type_index_in_function(self) -> None:
        module = _minimal_module(
            functions=[99],  # points to nonexistent type
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.INVALID_TYPE_INDEX

    def test_function_code_count_mismatch(self) -> None:
        module = _minimal_module(
            functions=[0, 0],  # 2 function declarations
            code=[FunctionBody(locals=(), code=bytes([0x0B]))],  # only 1 body
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.INVALID_FUNC_INDEX


# ===========================================================================
# Start function validation
# ===========================================================================


class TestStartFunction:
    def test_start_out_of_range(self) -> None:
        module = _minimal_module(start=99)
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.INVALID_FUNC_INDEX

    def test_start_wrong_signature(self) -> None:
        """Start function must have type () -> ()."""
        module = _minimal_module(start=0)
        # func at index 0 has type (i32) -> (i32), which is invalid for start
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.START_FUNCTION_BAD_TYPE

    def test_start_valid(self) -> None:
        """A start function with () -> () should be valid."""
        module = _minimal_module(
            types=[
                FuncType(params=(), results=()),
                FuncType(params=(ValueType.I32,), results=(ValueType.I32,)),
            ],
            functions=[0, 1],
            code=[
                FunctionBody(locals=(), code=bytes([0x0B])),
                FunctionBody(locals=(), code=bytes([0x20, 0x00, 0x0B])),
            ],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
            start=0,
        )
        result = validate(module)
        assert result is not None


# ===========================================================================
# Element segment validation
# ===========================================================================


class TestElements:
    def test_element_bad_func_index(self) -> None:
        module = _minimal_module(
            tables=[TableType(element_type=0x70, limits=Limits(min=10, max=None))],
            elements=[
                Element(
                    table_index=0,
                    offset_expr=bytes([0x41, 0x00, 0x0B]),
                    function_indices=[99],  # out of range
                ),
            ],
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.INVALID_FUNC_INDEX


# ===========================================================================
# Import validation
# ===========================================================================


class TestImportValidation:
    def test_import_bad_type_index(self) -> None:
        module = _minimal_module(
            imports=[
                Import(
                    module_name="env",
                    name="func",
                    kind=ExternalKind.FUNCTION,
                    type_info=99,
                ),
            ],
        )
        with pytest.raises(ValidationError) as exc_info:
            validate(module)
        assert exc_info.value.kind == ValidationErrorKind.INVALID_TYPE_INDEX

    def test_import_function_valid(self) -> None:
        module = _minimal_module(
            imports=[
                Import(
                    module_name="env",
                    name="func",
                    kind=ExternalKind.FUNCTION,
                    type_info=0,
                ),
            ],
            exports=[Export(name="f", kind=ExternalKind.FUNCTION, index=0)],
        )
        result = validate(module)
        # imported func + module func = 2 total
        assert len(result.func_types) == 2

    def test_import_memory(self) -> None:
        module = _minimal_module(
            imports=[
                Import(
                    module_name="env",
                    name="mem",
                    kind=ExternalKind.MEMORY,
                    type_info=MemoryType(limits=Limits(min=1, max=None)),
                ),
            ],
        )
        result = validate(module)
        assert result is not None

    def test_import_table(self) -> None:
        module = _minimal_module(
            imports=[
                Import(
                    module_name="env",
                    name="tbl",
                    kind=ExternalKind.TABLE,
                    type_info=TableType(element_type=0x70, limits=Limits(min=1, max=None)),
                ),
            ],
        )
        result = validate(module)
        assert result is not None

    def test_import_global(self) -> None:
        module = _minimal_module(
            imports=[
                Import(
                    module_name="env",
                    name="g",
                    kind=ExternalKind.GLOBAL,
                    type_info=GlobalType(value_type=ValueType.I32, mutable=False),
                ),
            ],
        )
        result = validate(module)
        assert result is not None
