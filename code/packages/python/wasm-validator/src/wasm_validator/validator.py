"""validator.py --- WebAssembly 1.0 module validator.

Validates WASM module structure: checks index bounds, memory/table limits,
export uniqueness, and start function signatures. Function body type checking
is a simplified pass that validates basic structure.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Any

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

MAX_MEMORY_PAGES = 65536


# ===========================================================================
# Validation Error
# ===========================================================================


class ValidationErrorKind(StrEnum):
    INVALID_TYPE_INDEX = "invalid_type_index"
    INVALID_FUNC_INDEX = "invalid_func_index"
    INVALID_TABLE_INDEX = "invalid_table_index"
    INVALID_MEMORY_INDEX = "invalid_memory_index"
    INVALID_GLOBAL_INDEX = "invalid_global_index"
    INVALID_LOCAL_INDEX = "invalid_local_index"
    INVALID_LABEL_INDEX = "invalid_label_index"
    INVALID_ELEMENT_INDEX = "invalid_element_index"
    MULTIPLE_MEMORIES = "multiple_memories"
    MULTIPLE_TABLES = "multiple_tables"
    MEMORY_LIMIT_EXCEEDED = "memory_limit_exceeded"
    MEMORY_LIMIT_ORDER = "memory_limit_order"
    TABLE_LIMIT_ORDER = "table_limit_order"
    DUPLICATE_EXPORT_NAME = "duplicate_export_name"
    EXPORT_INDEX_OUT_OF_RANGE = "export_index_out_of_range"
    START_FUNCTION_BAD_TYPE = "start_function_bad_type"
    IMMUTABLE_GLOBAL_WRITE = "immutable_global_write"
    INIT_EXPR_INVALID = "init_expr_invalid"
    TYPE_MISMATCH = "type_mismatch"
    STACK_UNDERFLOW = "stack_underflow"
    STACK_HEIGHT_MISMATCH = "stack_height_mismatch"
    RETURN_TYPE_MISMATCH = "return_type_mismatch"
    CALL_INDIRECT_TYPE_MISMATCH = "call_indirect_type_mismatch"


class ValidationError(Exception):
    """A WASM validation error with an error kind."""

    def __init__(self, kind: ValidationErrorKind, message: str) -> None:
        super().__init__(message)
        self.kind = kind


# ===========================================================================
# Validated Module
# ===========================================================================


@dataclass(frozen=True)
class ValidatedModule:
    """A validated WASM module with resolved type information."""

    module: WasmModule
    func_types: tuple[FuncType, ...]
    func_locals: tuple[tuple[ValueType, ...], ...]


# ===========================================================================
# Index Spaces
# ===========================================================================


@dataclass(frozen=True)
class IndexSpaces:
    """Combined index spaces for a WASM module."""

    func_types: tuple[FuncType, ...]
    num_imported_funcs: int
    table_types: tuple[TableType, ...]
    num_imported_tables: int
    memory_types: tuple[MemoryType, ...]
    num_imported_memories: int
    global_types: tuple[GlobalType, ...]
    num_imported_globals: int
    num_types: int


# ===========================================================================
# Public API
# ===========================================================================


def validate(module: WasmModule) -> ValidatedModule:
    """Validate a WASM module and return a ValidatedModule."""
    index_spaces = validate_structure(module)

    func_locals_list: list[tuple[ValueType, ...]] = []
    for i, body in enumerate(module.code):
        func_idx = index_spaces.num_imported_funcs + i
        func_type = index_spaces.func_types[func_idx]
        locals_tuple = tuple(func_type.params) + tuple(body.locals)
        func_locals_list.append(locals_tuple)

    return ValidatedModule(
        module=module,
        func_types=index_spaces.func_types,
        func_locals=tuple(func_locals_list),
    )


def validate_structure(module: WasmModule) -> IndexSpaces:
    """Validate module structure and return index spaces."""
    index_spaces = _build_index_spaces(module)

    if len(index_spaces.table_types) > 1:
        raise ValidationError(
            ValidationErrorKind.MULTIPLE_TABLES,
            f"WASM 1.0 allows at most one table, found {len(index_spaces.table_types)}",
        )

    if len(index_spaces.memory_types) > 1:
        raise ValidationError(
            ValidationErrorKind.MULTIPLE_MEMORIES,
            f"WASM 1.0 allows at most one memory, found {len(index_spaces.memory_types)}",
        )

    for mem_type in index_spaces.memory_types:
        _validate_memory_limits(mem_type.limits)

    for table_type in index_spaces.table_types:
        _validate_table_limits(table_type.limits)

    _validate_exports(module, index_spaces)
    _validate_start_function(module, index_spaces)

    for element in module.elements:
        for func_idx in element.function_indices:
            _ensure_index(
                func_idx,
                len(index_spaces.func_types),
                ValidationErrorKind.INVALID_FUNC_INDEX,
                f"Element references function index {func_idx}, but only {len(index_spaces.func_types)} exist",
            )

    return index_spaces


# ===========================================================================
# Internal helpers
# ===========================================================================


def _build_index_spaces(module: WasmModule) -> IndexSpaces:
    """Build the combined index spaces from imports and module definitions."""
    if len(module.functions) != len(module.code):
        raise ValidationError(
            ValidationErrorKind.INVALID_FUNC_INDEX,
            f"Function section declares {len(module.functions)} functions, "
            f"but code section contains {len(module.code)} bodies",
        )

    func_types: list[FuncType] = []
    table_types: list[TableType] = []
    memory_types: list[MemoryType] = []
    global_types: list[GlobalType] = []
    num_imported_funcs = 0
    num_imported_tables = 0
    num_imported_memories = 0
    num_imported_globals = 0

    for entry in module.imports:
        if entry.kind == ExternalKind.FUNCTION:
            type_index = entry.type_info
            if not isinstance(type_index, int):
                raise ValidationError(
                    ValidationErrorKind.INVALID_TYPE_INDEX,
                    f"Import '{entry.module_name}.{entry.name}' has non-integer type_info",
                )
            _ensure_index(
                type_index,
                len(module.types),
                ValidationErrorKind.INVALID_TYPE_INDEX,
                f"Import '{entry.module_name}.{entry.name}' references type {type_index}, "
                f"but only {len(module.types)} type(s) exist",
            )
            func_types.append(module.types[type_index])
            num_imported_funcs += 1
        elif entry.kind == ExternalKind.TABLE:
            table_types.append(entry.type_info)  # type: ignore[arg-type]
            num_imported_tables += 1
        elif entry.kind == ExternalKind.MEMORY:
            memory_types.append(entry.type_info)  # type: ignore[arg-type]
            num_imported_memories += 1
        elif entry.kind == ExternalKind.GLOBAL:
            global_types.append(entry.type_info)  # type: ignore[arg-type]
            num_imported_globals += 1

    for type_index in module.functions:
        _ensure_index(
            type_index,
            len(module.types),
            ValidationErrorKind.INVALID_TYPE_INDEX,
            f"Local function references type index {type_index}, "
            f"but only {len(module.types)} type(s) exist",
        )
        func_types.append(module.types[type_index])

    table_types.extend(module.tables)
    memory_types.extend(module.memories)
    global_types.extend(g.global_type for g in module.globals)

    return IndexSpaces(
        func_types=tuple(func_types),
        num_imported_funcs=num_imported_funcs,
        table_types=tuple(table_types),
        num_imported_tables=num_imported_tables,
        memory_types=tuple(memory_types),
        num_imported_memories=num_imported_memories,
        global_types=tuple(global_types),
        num_imported_globals=num_imported_globals,
        num_types=len(module.types),
    )


def _validate_exports(module: WasmModule, index_spaces: IndexSpaces) -> None:
    seen: set[str] = set()
    for exp in module.exports:
        if exp.name in seen:
            raise ValidationError(
                ValidationErrorKind.DUPLICATE_EXPORT_NAME,
                f"Duplicate export name '{exp.name}'",
            )
        seen.add(exp.name)

        upper = (
            len(index_spaces.func_types) if exp.kind == ExternalKind.FUNCTION
            else len(index_spaces.table_types) if exp.kind == ExternalKind.TABLE
            else len(index_spaces.memory_types) if exp.kind == ExternalKind.MEMORY
            else len(index_spaces.global_types) if exp.kind == ExternalKind.GLOBAL
            else 0
        )
        if exp.index < 0 or exp.index >= upper:
            raise ValidationError(
                ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE,
                f"Export '{exp.name}' references index {exp.index}, "
                f"but only {upper} definition(s) exist",
            )


def _validate_start_function(module: WasmModule, index_spaces: IndexSpaces) -> None:
    if module.start is None:
        return
    _ensure_index(
        module.start,
        len(index_spaces.func_types),
        ValidationErrorKind.INVALID_FUNC_INDEX,
        f"Start function index {module.start} out of range",
    )
    start_type = index_spaces.func_types[module.start]
    if len(start_type.params) != 0 or len(start_type.results) != 0:
        raise ValidationError(
            ValidationErrorKind.START_FUNCTION_BAD_TYPE,
            f"Start function must have type () -> (), got {start_type}",
        )


def _validate_memory_limits(limits: Limits) -> None:
    if limits.max is not None and limits.max > MAX_MEMORY_PAGES:
        raise ValidationError(
            ValidationErrorKind.MEMORY_LIMIT_EXCEEDED,
            f"Memory maximum {limits.max} exceeds the WASM 1.0 limit of {MAX_MEMORY_PAGES} pages",
        )
    if limits.max is not None and limits.min > limits.max:
        raise ValidationError(
            ValidationErrorKind.MEMORY_LIMIT_ORDER,
            f"Memory minimum {limits.min} exceeds maximum {limits.max}",
        )


def _validate_table_limits(limits: Limits) -> None:
    if limits.max is not None and limits.min > limits.max:
        raise ValidationError(
            ValidationErrorKind.TABLE_LIMIT_ORDER,
            f"Table minimum {limits.min} exceeds maximum {limits.max}",
        )


def _ensure_index(
    index: int,
    length: int,
    kind: ValidationErrorKind,
    message: str,
) -> None:
    if not isinstance(index, int) or index < 0 or index >= length:
        raise ValidationError(kind, message)
