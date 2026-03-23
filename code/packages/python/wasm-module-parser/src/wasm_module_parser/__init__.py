"""wasm-module-parser — Parses a raw .wasm binary into a structured WasmModule.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

This is the decoder layer of the WASM toolchain. It takes raw bytes and
produces a structured ``WasmModule`` object. It does NOT execute anything.

-------------------------------------------------------------------------------
THE BIG PICTURE: WHAT IS A .wasm FILE?
-------------------------------------------------------------------------------

A .wasm file is a WebAssembly binary module. WebAssembly (WASM) is a portable,
sandboxed bytecode format designed to run at near-native speed in web browsers
and, increasingly, on servers via runtimes like Wasmtime, Wasmer, and WASI.

Think of a .wasm file like a ZIP archive of structured sections:

    ┌─────────────────────────────────────────────────────────────────────┐
    │   .wasm file layout (each "block" is a section)                    │
    ├─────────────────────────────────────────────────────────────────────┤
    │  [magic 4 bytes][version 4 bytes]  ← always at the start           │
    │  [section_id u8][size LEB128][payload bytes]  ← section 1          │
    │  [section_id u8][size LEB128][payload bytes]  ← section 2          │
    │  ...                                                                │
    └─────────────────────────────────────────────────────────────────────┘

The magic bytes are  0x00 0x61 0x73 0x6D  which is "\0asm".
The version is 0x01 0x00 0x00 0x00 (little-endian 1).

Why this design?
  - Sections are independently parseable: a tool that only cares about exports
    can skip everything else.
  - LEB128 variable-length integers keep the binary compact: most counts are
    small and fit in a single byte.
  - IDs are stable: new section types can be added with new IDs (custom
    sections, ID=0, are an extension point).

-------------------------------------------------------------------------------
SECTION ID TABLE
-------------------------------------------------------------------------------

    ┌────────────┬──────────────────────┬─────────────────────────────────┐
    │ Section ID │ Section Name         │ Contains                        │
    ├────────────┼──────────────────────┼─────────────────────────────────┤
    │  0         │ Custom               │ name + arbitrary bytes          │
    │  1         │ Type                 │ list of FuncType                │
    │  2         │ Import               │ list of Import                  │
    │  3         │ Function             │ list of type indices (u32)      │
    │  4         │ Table                │ list of TableType               │
    │  5         │ Memory               │ list of MemoryType              │
    │  6         │ Global               │ list of Global                  │
    │  7         │ Export               │ list of Export                  │
    │  8         │ Start                │ single function index           │
    │  9         │ Element              │ list of Element                 │
    │ 10         │ Code                 │ list of FunctionBody            │
    │ 11         │ Data                 │ list of DataSegment             │
    └────────────┴──────────────────────┴─────────────────────────────────┘

Sections 1–11 must appear in order. Section 0 (Custom) may appear anywhere.
All sections are optional: a minimal valid module is just the 8-byte header.

-------------------------------------------------------------------------------
API
-------------------------------------------------------------------------------

    WasmModuleParser().parse(data: bytes) -> WasmModule
    WasmParseError(message: str, offset: int)     — raised on malformed input
"""

from __future__ import annotations

__version__ = "0.1.0"

from wasm_leb128 import decode_unsigned
from wasm_types import (
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
)

# ---------------------------------------------------------------------------
# CONSTANTS: MAGIC BYTES AND SECTION IDs
#
# The 4-byte magic is the ASCII string "\0asm" — it was chosen so that:
#   1. The null byte in position 0 prevents accidental text-file misdetection.
#   2. "asm" is a mnemonic for "assembly."
#
# Section IDs are 1-byte unsigned integers. The ordering (1–11) in the binary
# format matches the logical dependency order: you can't reference a type
# (section 1) before it's declared, can't have code (section 10) for functions
# (section 3) that haven't been listed yet, etc.
# ---------------------------------------------------------------------------

WASM_MAGIC: bytes = b"\x00asm"
WASM_VERSION: bytes = b"\x01\x00\x00\x00"

SECTION_CUSTOM: int = 0
SECTION_TYPE: int = 1
SECTION_IMPORT: int = 2
SECTION_FUNCTION: int = 3
SECTION_TABLE: int = 4
SECTION_MEMORY: int = 5
SECTION_GLOBAL: int = 6
SECTION_EXPORT: int = 7
SECTION_START: int = 8
SECTION_ELEMENT: int = 9
SECTION_CODE: int = 10
SECTION_DATA: int = 11


# ---------------------------------------------------------------------------
# PARSE ERROR
#
# Every parse failure carries the byte offset where the problem was detected.
# This makes it much easier to diagnose binary files with a hex editor.
# ---------------------------------------------------------------------------

class WasmParseError(Exception):
    """Raised when ``WasmModuleParser.parse`` encounters malformed input.

    Attributes:
        message: Human-readable description of what went wrong.
        offset:  Byte offset in the input data where the error was detected.

    Example:
        try:
            WasmModuleParser().parse(b"not wasm")
        except WasmParseError as e:
            print(e.message)   # "bad magic bytes at offset 0: ..."
            print(e.offset)    # 0
    """

    def __init__(self, message: str, offset: int) -> None:
        super().__init__(message)
        self.message = message
        self.offset = offset


# ---------------------------------------------------------------------------
# PARSER — CURSOR-BASED BYTE READER
#
# We use a simple index (``pos``) rather than file-like streams so that every
# helper can accept the raw ``data`` bytes and a ``pos`` integer and return
# ``(parsed_value, new_pos)``.  This is pure-functional and makes the logic
# easy to test in isolation.
#
# Pattern used throughout:
#
#   value, pos = _read_leb128(data, pos)
#   text, pos  = _read_name(data, pos)
#   ...
#
# If any read extends beyond ``len(data)``, we raise WasmParseError with the
# relevant offset.  The ``_check`` helper consolidates this guard.
# ---------------------------------------------------------------------------

def _check(data: bytes, offset: int, needed: int) -> None:
    """Raise WasmParseError if [offset, offset+needed) is out of bounds.

    Args:
        data:   The full binary buffer.
        offset: Starting position of the attempted read.
        needed: Number of bytes needed.

    Raises:
        WasmParseError if offset + needed > len(data).

    Example:
        # Works fine:
        _check(b'\\x00asm', 0, 4)

        # Raises:
        _check(b'\\x00asm', 3, 4)  # only 1 byte left, need 4
    """
    if offset + needed > len(data):
        msg = (
            f"unexpected end of input at offset {offset}: "
            f"need {needed} bytes, only {len(data) - offset} available"
        )
        raise WasmParseError(msg, offset)


def _read_leb128(data: bytes, pos: int) -> tuple[int, int]:
    """Read one unsigned LEB128 integer from data at pos.

    Delegates to ``wasm_leb128.decode_unsigned`` and translates
    ``LEB128Error`` into ``WasmParseError`` for consistent error types.

    Returns:
        (value, new_pos)

    Example:
        value, pos = _read_leb128(b'\\x03', 0)
        # value=3, pos=1
    """
    if pos >= len(data):
        msg = (
            f"unexpected end of input at offset {pos}: "
            "need at least 1 byte for LEB128"
        )
        raise WasmParseError(msg, pos)
    try:
        value, consumed = decode_unsigned(data, pos)
    except Exception as exc:
        raise WasmParseError(str(exc), pos) from exc
    return value, pos + consumed


def _read_name(data: bytes, pos: int) -> tuple[str, int]:
    """Read a length-prefixed UTF-8 name from data at pos.

    Format:
        <length: u32 LEB128>  <utf-8 bytes × length>

    WASM uses this for import module names, import field names, export names,
    and custom section names.

    Returns:
        (name_str, new_pos)

    Raises:
        WasmParseError if truncated or invalid UTF-8.

    Example:
        # Encode "hi": 0x02 0x68 0x69
        name, pos = _read_name(b'\\x02hi', 0)
        # name="hi", pos=3
    """
    length, pos = _read_leb128(data, pos)
    _check(data, pos, length)
    try:
        name = data[pos : pos + length].decode("utf-8")
    except UnicodeDecodeError as exc:
        raise WasmParseError(
            f"invalid UTF-8 in name at offset {pos}: {exc}", pos
        ) from exc
    return name, pos + length


def _read_limits(data: bytes, pos: int) -> tuple[Limits, int]:
    """Read a Limits struct from data at pos.

    Binary format:
        flags: u8   (0 = no max, 1 = has max)
        min:   u32 LEB128
        max:   u32 LEB128  (only present if flags == 1)

    Limits are used for both memories (pages) and tables (element slots).

    Returns:
        (Limits, new_pos)

    Example:
        # 0x01 0x01 0x04 → min=1, max=4
        lim, pos = _read_limits(bytes([0x01, 0x01, 0x04]), 0)
    """
    _check(data, pos, 1)
    flags = data[pos]
    pos += 1
    min_val, pos = _read_leb128(data, pos)
    max_val: int | None = None
    if flags & 1:
        max_val, pos = _read_leb128(data, pos)
    return Limits(min=min_val, max=max_val), pos


def _read_expr(data: bytes, pos: int) -> tuple[bytes, int]:
    """Read a constant expression (init_expr) from data at pos.

    A constant expression is a sequence of opcodes terminated by 0x0B (end).
    We capture the entire byte sequence including the 0x0B terminator.

    Why do we store the raw bytes?
        The parser's job is decoding structure, not interpreting semantics.
        A separate evaluator can interpret these bytes at instantiation time.
        Storing them raw keeps the parser simple and faithful to the spec.

    Returns:
        (expr_bytes, new_pos)

    Raises:
        WasmParseError if 0x0B is never found before end of data.
    """
    start = pos
    while True:
        if pos >= len(data):
            msg = f"unterminated init_expr at offset {start}: no 0x0B end opcode found"
            raise WasmParseError(msg, start)
        if data[pos] == 0x0B:
            pos += 1  # consume the 0x0B
            break
        pos += 1
    return data[start:pos], pos


# ---------------------------------------------------------------------------
# SECTION PARSERS
#
# Each parse_*_section function takes the raw section payload (the bytes
# between the section-size field and the next section) and returns the
# appropriate Python object(s).  The ``offset`` parameter is used only for
# error reporting — it is the absolute byte offset of the payload start in
# the original file.
# ---------------------------------------------------------------------------

def _parse_type_section(payload: bytes, base_offset: int) -> list[FuncType]:
    """Parse the Type section (ID 1).

    The Type section is a vector of function type descriptors. Each entry
    describes the parameter types and result types of one signature. Functions
    reference these by index rather than repeating the full type inline.

    Binary layout of one FuncType entry:
        0x60              — functype indicator byte (not part of the value)
        count:  LEB128    — number of params
        params: [valtype] — one byte per param
        count:  LEB128    — number of results
        results:[valtype] — one byte per result

    Why have a separate Type section?
        Many functions share signatures. Storing each type once and referencing
        by index compresses the binary. A module with 100 functions that all
        take (i32, i32) → i32 only encodes that type once.
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    types: list[FuncType] = []
    for i in range(count):
        if pos >= len(payload):
            raise WasmParseError(
                f"truncated type section: expected {count} types, got {i}",
                base_offset + pos,
            )
        # Each function type entry starts with 0x60 = functype marker.
        if payload[pos] != 0x60:
            raise WasmParseError(
                f"expected functype marker 0x60 at offset {base_offset + pos}, "
                f"got 0x{payload[pos]:02X}",
                base_offset + pos,
            )
        pos += 1  # consume 0x60

        # Read params
        param_count, pos = _read_leb128(payload, pos)
        params: list[ValueType] = []
        for _ in range(param_count):
            _check(payload, pos, 1)
            params.append(ValueType(payload[pos]))
            pos += 1

        # Read results
        result_count, pos = _read_leb128(payload, pos)
        results: list[ValueType] = []
        for _ in range(result_count):
            _check(payload, pos, 1)
            results.append(ValueType(payload[pos]))
            pos += 1

        types.append(FuncType(params=tuple(params), results=tuple(results)))

    return types


def _parse_import_section(payload: bytes, base_offset: int) -> list[Import]:
    """Parse the Import section (ID 2).

    Imports bring external entities into the module. A module can import
    functions, tables, memories, and globals. Imports are indexed before
    local definitions in their respective address spaces.

    Why import?
        - Access host APIs (WASI, JavaScript window/document, etc.)
        - Link multiple modules together
        - Share memory between modules

    Binary layout of one Import entry:
        module_name: length-prefixed UTF-8
        field_name:  length-prefixed UTF-8
        kind:        u8 (0=func, 1=table, 2=memory, 3=global)
        type_info:   depends on kind
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    imports: list[Import] = []
    for _ in range(count):
        module_name, pos = _read_name(payload, pos)
        field_name, pos = _read_name(payload, pos)
        _check(payload, pos, 1)
        kind_byte = payload[pos]
        pos += 1
        kind = ExternalKind(kind_byte)

        type_info: int | TableType | MemoryType | GlobalType
        if kind == ExternalKind.FUNCTION:
            # Function import: just a type section index.
            type_info, pos = _read_leb128(payload, pos)
        elif kind == ExternalKind.TABLE:
            # Table import: element_type byte + limits.
            _check(payload, pos, 1)
            _element_type = payload[pos]  # 0x70 = funcref (noqa: ignored)
            pos += 1
            lim, pos = _read_limits(payload, pos)
            type_info = TableType(element_type=_element_type, limits=lim)
        elif kind == ExternalKind.MEMORY:
            # Memory import: just limits.
            lim, pos = _read_limits(payload, pos)
            type_info = MemoryType(limits=lim)
        elif kind == ExternalKind.GLOBAL:
            # Global import: valtype + mutability byte.
            _check(payload, pos, 2)
            valtype = ValueType(payload[pos])
            mutable = bool(payload[pos + 1])
            pos += 2
            type_info = GlobalType(value_type=valtype, mutable=mutable)
        else:
            raise WasmParseError(
                f"unknown import kind 0x{kind_byte:02X} "
                f"at offset {base_offset + pos - 1}",
                base_offset + pos - 1,
            )

        imports.append(
            Import(
                module_name=module_name,
                name=field_name,
                kind=kind,
                type_info=type_info,
            )
        )
    return imports


def _parse_function_section(payload: bytes, base_offset: int) -> list[int]:
    """Parse the Function section (ID 3).

    The Function section is a vector of type indices. Entry i gives the type
    index (into the Type section) for the i-th locally-defined function.

    Why have a separate Function section?
        Separating type indices from function bodies allows tools to examine
        function signatures without parsing bytecode.

    Binary layout:
        count:         LEB128
        type_indices:  [u32 LEB128 × count]
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    functions: list[int] = []
    for _ in range(count):
        idx, pos = _read_leb128(payload, pos)
        functions.append(idx)
    return functions


def _parse_table_section(payload: bytes, base_offset: int) -> list[TableType]:
    """Parse the Table section (ID 4).

    A table is an array of opaque references (in WASM 1.0, always funcref
    = 0x70). Tables enable indirect function calls: the ``call_indirect``
    instruction looks up a function reference in a table by index at runtime.

    Why tables instead of direct function pointers?
        WASM is sandboxed. Function pointers in C/C++ compile to table indices.
        The runtime checks the table bounds on every ``call_indirect``, so a
        bug in sandboxed code cannot escape to an arbitrary function.

    Binary layout of one Table entry:
        element_type: u8 (must be 0x70 = funcref in WASM 1.0)
        limits:       flags + min [+ max]
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    tables: list[TableType] = []
    for _ in range(count):
        _check(payload, pos, 1)
        element_type = payload[pos]
        pos += 1
        lim, pos = _read_limits(payload, pos)
        tables.append(TableType(element_type=element_type, limits=lim))
    return tables


def _parse_memory_section(payload: bytes, base_offset: int) -> list[MemoryType]:
    """Parse the Memory section (ID 5).

    WASM 1.0 allows at most one linear memory per module. It is a flat
    byte-addressable array. The size is measured in 64-KiB pages (65536 bytes).
    The module declares initial and optional maximum page counts here.

    Why 64-KiB pages?
        Memory.grow is a coarse operation: each grow call increases memory by
        at least one page. 64 KiB is a practical granularity — small enough
        to avoid excessive waste, large enough that the implementation can use
        OS-level virtual memory mappings.

    Binary layout of one Memory entry:
        limits: flags + min [+ max]
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    memories: list[MemoryType] = []
    for _ in range(count):
        lim, pos = _read_limits(payload, pos)
        memories.append(MemoryType(limits=lim))
    return memories


def _parse_global_section(payload: bytes, base_offset: int) -> list[Global]:
    """Parse the Global section (ID 6).

    Globals are module-level variables. They have a type (i32/i64/f32/f64),
    a mutability flag, and a constant initializer expression.

    Common uses of globals in compiled WASM:
        - Stack pointer (__stack_pointer): mutable i32, tracks the C stack
        - Constants like __data_end, __heap_base: immutable i32

    Binary layout of one Global entry:
        valtype:   u8
        mutable:   u8 (0 = immutable, 1 = mutable)
        init_expr: bytes until 0x0B (inclusive)
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    globals_: list[Global] = []
    for _ in range(count):
        _check(payload, pos, 2)
        valtype = ValueType(payload[pos])
        mutable = bool(payload[pos + 1])
        pos += 2
        init_expr, pos = _read_expr(payload, pos)
        globals_.append(
            Global(
                global_type=GlobalType(value_type=valtype, mutable=mutable),
                init_expr=init_expr,
            )
        )
    return globals_


def _parse_export_section(payload: bytes, base_offset: int) -> list[Export]:
    """Parse the Export section (ID 7).

    Exports make module-internal entities (functions, tables, memories,
    globals) visible to the host environment or other modules.

    In WASM runtimes, the host calls exported functions by name (e.g.,
    ``_start`` for WASI programs, or ``main`` for C programs compiled to
    WASM).

    Binary layout of one Export entry:
        name:  length-prefixed UTF-8
        kind:  u8 (0=func, 1=table, 2=memory, 3=global)
        index: u32 LEB128 (index in the respective address space)
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    exports: list[Export] = []
    for _ in range(count):
        name, pos = _read_name(payload, pos)
        _check(payload, pos, 1)
        kind = ExternalKind(payload[pos])
        pos += 1
        index, pos = _read_leb128(payload, pos)
        exports.append(Export(name=name, kind=kind, index=index))
    return exports


def _parse_start_section(payload: bytes, base_offset: int) -> int:
    """Parse the Start section (ID 8).

    The Start section, if present, names a function to call automatically
    when the module is instantiated (before any explicit calls from the host).

    In practice, C/C++ programs compiled to WASM use the Start section to
    run global constructors, initialise the stack, and then call ``main``.

    Binary layout:
        function_index: u32 LEB128

    The function at that index must have type () → ().
    """
    pos = 0
    index, _pos = _read_leb128(payload, pos)
    return index


def _parse_element_section(payload: bytes, base_offset: int) -> list[Element]:
    """Parse the Element section (ID 9).

    The Element section initializes slots in a table with function references.
    It is essential for call_indirect: the table must be populated with the
    correct function references before any indirect calls happen.

    Example: a C function pointer table in WASM would appear here.

    Binary layout of one Element entry:
        table_index:      u32 LEB128 (always 0 in WASM 1.0)
        offset_expr:      bytes until 0x0B
        func_count:       u32 LEB128
        function_indices: [u32 LEB128 × func_count]
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    elements: list[Element] = []
    for _ in range(count):
        table_index, pos = _read_leb128(payload, pos)
        offset_expr, pos = _read_expr(payload, pos)
        func_count, pos = _read_leb128(payload, pos)
        indices: list[int] = []
        for _ in range(func_count):
            idx, pos = _read_leb128(payload, pos)
            indices.append(idx)
        elements.append(
            Element(
                table_index=table_index,
                offset_expr=offset_expr,
                function_indices=tuple(indices),
            )
        )
    return elements


def _parse_code_section(payload: bytes, base_offset: int) -> list[FunctionBody]:
    """Parse the Code section (ID 10).

    The Code section contains the bytecode bodies of all locally-defined
    functions, in the same order as the Function section. Entry i in the
    Code section corresponds to entry i in the Function section.

    Each body is self-contained: it starts with its own size field, so a
    parser or JIT compiler can skip or defer-parse individual function bodies.

    Binary layout of one Code entry:
        body_size:       u32 LEB128  (total bytes for this body entry)
        local_decl_count: u32 LEB128 (number of local-variable groups)
        local_decls:     [(count: LEB128, valtype: u8) × local_decl_count]
            — expand: e.g., (3, i32) means 3 i32 local variables
        code:            remaining bytes in body (bytecode, ends with 0x0B)

    Why group locals?
        If a function has 10 i32 locals, it's wasteful to store 10 × (1+1)
        bytes. The grouping (count, type) compresses runs of same-type locals.
        We expand them here for the consumer's convenience.
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    bodies: list[FunctionBody] = []
    for _ in range(count):
        # body_size includes everything in this entry AFTER the size field itself.
        body_size, pos = _read_leb128(payload, pos)
        body_start = pos
        _check(payload, pos, body_size)

        # Parse local variable declarations.
        local_decl_count, pos = _read_leb128(payload, pos)
        local_types: list[ValueType] = []
        for _ in range(local_decl_count):
            run_count, pos = _read_leb128(payload, pos)
            _check(payload, pos, 1)
            valtype = ValueType(payload[pos])
            pos += 1
            # Expand the (count, type) run into individual ValueType entries.
            local_types.extend([valtype] * run_count)

        # The remaining bytes in the body are the raw bytecode.
        code_end = body_start + body_size
        code = payload[pos:code_end]
        pos = code_end

        bodies.append(FunctionBody(locals=tuple(local_types), code=bytes(code)))
    return bodies


def _parse_data_section(payload: bytes, base_offset: int) -> list[DataSegment]:
    """Parse the Data section (ID 11).

    The Data section initializes regions of linear memory with constant byte
    strings. This is how a compiled C program loads its .rodata and .data
    sections into WASM memory at startup.

    Example:
        A C string literal like ``"hello, world"`` ends up as a DataSegment
        that writes those bytes into memory at the address the linker chose.

    Binary layout of one Data entry:
        memory_index: u32 LEB128 (always 0 in WASM 1.0)
        offset_expr:  bytes until 0x0B
        byte_count:   u32 LEB128
        data_bytes:   [u8 × byte_count]
    """
    pos = 0
    count, pos = _read_leb128(payload, pos)
    segments: list[DataSegment] = []
    for _ in range(count):
        memory_index, pos = _read_leb128(payload, pos)
        offset_expr, pos = _read_expr(payload, pos)
        byte_count, pos = _read_leb128(payload, pos)
        _check(payload, pos, byte_count)
        data = payload[pos : pos + byte_count]
        pos += byte_count
        segments.append(
            DataSegment(
                memory_index=memory_index,
                offset_expr=offset_expr,
                data=bytes(data),
            )
        )
    return segments


def _parse_custom_section(payload: bytes, base_offset: int) -> CustomSection:
    """Parse a Custom section (ID 0).

    Custom sections carry non-standard extension data. The WASM runtime ignores
    any custom section it does not recognise, so they are a safe extension point.

    Common custom sections:
        "name":      WASM Name section — human-readable function/local names
        ".debug_*":  DWARF debug info (used by Emscripten, wasm-pack)
        "producers": Toolchain metadata (which compiler was used, versions)

    Binary layout:
        name_len:   u32 LEB128
        name_bytes: UTF-8
        data:       remaining bytes in the section payload
    """
    pos = 0
    name, pos = _read_name(payload, pos)
    data = payload[pos:]
    return CustomSection(name=name, data=bytes(data))


# ---------------------------------------------------------------------------
# MAIN PARSER CLASS
# ---------------------------------------------------------------------------

class WasmModuleParser:
    """Parses a raw .wasm binary file into a structured ``WasmModule``.

    This class implements the decoder layer of the WASM toolchain. It reads
    bytes sequentially, validates the header, then dispatches each section
    to the appropriate section parser.

    The parser is stateless: each call to ``parse`` is independent.

    Usage:
        parser = WasmModuleParser()
        with open("program.wasm", "rb") as f:
            module = parser.parse(f.read())

        # Inspect the parsed module:
        for ft in module.types:
            print(ft.params, "->", ft.results)
        for exp in module.exports:
            print(exp.name, exp.kind, exp.index)

    Raises:
        WasmParseError: On any malformed input (bad magic, wrong version,
                        truncated section, unknown section kind, etc.).
    """

    def parse(self, data: bytes) -> WasmModule:
        """Parse a complete .wasm binary.

        Args:
            data: The raw bytes of a .wasm file.

        Returns:
            A populated ``WasmModule`` object.

        Raises:
            WasmParseError: If the binary is malformed.

        Example:
            # Minimal valid module (just the 8-byte header):
            module = WasmModuleParser().parse(b'\\x00asm\\x01\\x00\\x00\\x00')
            assert module.types == []
            assert module.exports == []
        """
        pos = 0

        # ------------------------------------------------------------------
        # STEP 1: VALIDATE THE HEADER
        #
        # The header is exactly 8 bytes:
        #   [0x00 0x61 0x73 0x6D]  ← magic "\0asm"
        #   [0x01 0x00 0x00 0x00]  ← version 1 (little-endian u32)
        #
        # If the file is shorter than 8 bytes, or the magic/version don't
        # match, we raise immediately.
        #
        # Byte layout of the header:
        #
        #   offset 0  1  2  3  4  5  6  7
        #   value  00 61 73 6D 01 00 00 00
        #          ↑──────────↑ ↑──────────↑
        #          magic "\0asm"  version 1
        # ------------------------------------------------------------------
        if len(data) < 8:
            raise WasmParseError(
                f"truncated header: need 8 bytes, got {len(data)}", 0
            )

        if data[0:4] != WASM_MAGIC:
            raise WasmParseError(
                f"bad magic bytes at offset 0: expected \\x00asm, "
                f"got {data[0:4]!r}",
                0,
            )

        if data[4:8] != WASM_VERSION:
            raise WasmParseError(
                f"unsupported version at offset 4: expected \\x01\\x00\\x00\\x00, "
                f"got {data[4:8]!r}",
                4,
            )

        pos = 8

        # ------------------------------------------------------------------
        # STEP 2: READ SECTIONS ONE BY ONE
        #
        # After the header, the file is a sequence of sections. Each section
        # has this envelope:
        #
        #   section_id:   u8            (1 byte)
        #   section_size: u32 LEB128    (variable, usually 1–5 bytes)
        #   payload:      bytes         (section_size bytes)
        #
        # We read the id, size, and payload, then dispatch to the appropriate
        # section parser.  We track ``last_non_custom_id`` to enforce the
        # section ordering rule: sections 1–11 must appear in ascending ID
        # order (custom sections, ID=0, may appear anywhere).
        # ------------------------------------------------------------------
        module = WasmModule()
        last_non_custom_id: int = 0

        while pos < len(data):
            # Read section ID (1 byte).
            _check(data, pos, 1)
            section_id = data[pos]
            pos += 1

            # Read section size (LEB128).
            section_size, pos = _read_leb128(data, pos)

            # Extract the section payload slice.
            _check(data, pos, section_size)
            payload = data[pos : pos + section_size]
            payload_base = pos
            pos += section_size

            # Enforce section ordering for non-custom sections.
            if section_id != SECTION_CUSTOM:
                if section_id < last_non_custom_id:
                    raise WasmParseError(
                        f"section ordering violation: section {section_id} "
                        f"appears after section {last_non_custom_id}",
                        payload_base,
                    )
                last_non_custom_id = section_id

            # Dispatch to the appropriate parser.
            if section_id == SECTION_CUSTOM:
                cs = _parse_custom_section(payload, payload_base)
                module.customs.append(cs)

            elif section_id == SECTION_TYPE:
                module.types = _parse_type_section(payload, payload_base)

            elif section_id == SECTION_IMPORT:
                module.imports = _parse_import_section(payload, payload_base)

            elif section_id == SECTION_FUNCTION:
                module.functions = _parse_function_section(payload, payload_base)

            elif section_id == SECTION_TABLE:
                module.tables = _parse_table_section(payload, payload_base)

            elif section_id == SECTION_MEMORY:
                module.memories = _parse_memory_section(payload, payload_base)

            elif section_id == SECTION_GLOBAL:
                module.globals = _parse_global_section(payload, payload_base)

            elif section_id == SECTION_EXPORT:
                module.exports = _parse_export_section(payload, payload_base)

            elif section_id == SECTION_START:
                module.start = _parse_start_section(payload, payload_base)

            elif section_id == SECTION_ELEMENT:
                module.elements = _parse_element_section(payload, payload_base)

            elif section_id == SECTION_CODE:
                module.code = _parse_code_section(payload, payload_base)

            elif section_id == SECTION_DATA:
                module.data = _parse_data_section(payload, payload_base)

            else:
                # Unknown section ID — raise. (Valid modules have IDs 0–11.)
                raise WasmParseError(
                    f"unknown section ID {section_id} at offset {payload_base - 2}",
                    payload_base - 2,
                )

        return module
