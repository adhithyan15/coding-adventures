"""wasm-types — WASM 1.0 type system: pure data structures for every type-level concept.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

-------------------------------------------------------------------------------
BACKGROUND: WHERE TYPES LIVE IN THE WASM BINARY FORMAT
-------------------------------------------------------------------------------

A WebAssembly module is a sequence of sections, each identified by a 1-byte
section ID. The binary layout looks like this:

    ┌─────────────────────────────────────────────────────────────────────┐
    │  WebAssembly module binary (.wasm)                                  │
    ├─────────────────────────────────────────────────────────────────────┤
    │  Magic:  0x00 0x61 0x73 0x6D  ("\\0asm")                           │
    │  Version: 0x01 0x00 0x00 0x00  (1)                                 │
    ├────────────┬────────────────────────────────────────────────────────┤
    │ Section ID │ Section Name         │ Contains                        │
    ├────────────┼──────────────────────┼─────────────────────────────────┤
    │  1         │ Type section         │ list[FuncType]                  │
    │  2         │ Import section       │ list[Import]                    │
    │  3         │ Function section     │ list[int]  (type indices)       │
    │  4         │ Table section        │ list[TableType]                 │
    │  5         │ Memory section       │ list[MemoryType]                │
    │  6         │ Global section       │ list[Global]                    │
    │  7         │ Export section       │ list[Export]                    │
    │  8         │ Start section        │ int | None  (function index)    │
    │  9         │ Element section      │ list[Element]                   │
    │ 10         │ Code section         │ list[FunctionBody]              │
    │ 11         │ Data section         │ list[DataSegment]               │
    │  0         │ Custom section       │ list[CustomSection]             │
    └────────────┴──────────────────────┴─────────────────────────────────┘

The types in this package correspond 1-to-1 with that table. The byte values
of each enum member match the WASM binary encoding specification exactly, so
the parser can use them directly for encoding and decoding.

-------------------------------------------------------------------------------
WASM TYPE SYSTEM OVERVIEW
-------------------------------------------------------------------------------

WASM is a stack machine with a strict static type system. Every value on the
operand stack has one of four types:

    i32  — 32-bit integer  (used for booleans, pointers, and 32-bit arithmetic)
    i64  — 64-bit integer
    f32  — 32-bit IEEE 754 float
    f64  — 64-bit IEEE 754 float

These map to the ValueType enum. Their byte values are chosen so that they fit
in the negative range of a signed LEB128 byte (0x40–0x7F), making them compact
in the binary format.

    Encoding layout of ValueType bytes (all fit in 1 LEB128 byte):

    ┌─────────────────────────────────────────┐
    │  Binary byte │ Type  │ Signed LEB128     │
    ├──────────────┼───────┼───────────────────┤
    │    0x7F      │  i32  │  -1               │
    │    0x7E      │  i64  │  -2               │
    │    0x7D      │  f32  │  -3               │
    │    0x7C      │  f64  │  -4               │
    └─────────────────────────────────────────┘

-------------------------------------------------------------------------------
DESIGN CHOICES
-------------------------------------------------------------------------------

1.  All "record" types (FuncType, Limits, etc.) are frozen dataclasses. This
    means they are immutable once constructed and are safe to use as dict keys
    or in sets. It also makes equality testing structural by default.

2.  Tuple fields (params, results, function_indices) are used instead of lists
    to enforce immutability at the collection level. A frozen dataclass cannot
    prevent mutation of a list field, but a tuple is inherently immutable.

3.  WasmModule is deliberately *mutable*. A parser fills it in incrementally
    as it reads section after section. Making WasmModule frozen would require
    building the entire module in one step, which is awkward for streaming
    parsers.

4.  BlockType is not a simple enum. The WASM spec allows a block to produce:
    - No value:      encoded as 0x40
    - One value:     encoded as a ValueType byte
    - Multiple values: encoded as a type index (LEB128 signed integer >= 0)
    We model this with a class that has a single constant plus the convention
    that callers use ValueType directly for single-result blocks and integers
    for type-indexed multi-value blocks.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum

__version__ = "0.1.0"


# ---------------------------------------------------------------------------
# VALUE TYPE
#
# ValueType represents the primitive types in the WASM execution model.
# These are the types that can appear on the operand stack, in function
# signatures, in local variable declarations, and in global variable types.
#
# The numeric values come directly from the WASM binary format spec (section
# "Value Types" in the binary encoding chapter). They are chosen to be
# negative numbers in signed LEB128 interpretation — this is how the spec
# distinguishes them from non-negative type indices.
#
# Binary encoding diagram:
#
#    i32  →  0x7F  →  0111_1111  →  -1 as signed 7-bit value
#    i64  →  0x7E  →  0111_1110  →  -2
#    f32  →  0x7D  →  0111_1101  →  -3
#    f64  →  0x7C  →  0111_1100  →  -4
#
# Example usage in WASM:
#    ;; A function that takes (i32, i64) and returns f64
#    (func (param i32) (param i64) (result f64) ...)
#    ;; Encoded in binary type section as:
#    ;; 0x60  — functype indicator
#    ;; 0x02  — 2 params
#    ;; 0x7F  — i32
#    ;; 0x7E  — i64
#    ;; 0x01  — 1 result
#    ;; 0x7C  — f64
# ---------------------------------------------------------------------------
class ValueType(IntEnum):
    """The four primitive value types of the WASM 1.0 type system.

    Each member's integer value equals its WASM binary encoding byte.

    Example:
        >>> ValueType.I32 == 0x7F
        True
        >>> int(ValueType.F64)
        124
        >>> bytes([ValueType.I32, ValueType.I64])
        b'\\x7f~'
    """

    I32 = 0x7F  # 32-bit integer — also used for booleans and memory addresses
    I64 = 0x7E  # 64-bit integer
    F32 = 0x7D  # 32-bit IEEE 754 floating-point
    F64 = 0x7C  # 64-bit IEEE 754 floating-point


# ---------------------------------------------------------------------------
# BLOCK TYPE
#
# A "block" in WASM (block, loop, if) can produce zero or more results.
# The BlockType encoding handles three cases:
#
#   1. EMPTY (0x40): the block produces no values.
#      Most blocks fall into this category.
#
#   2. A ValueType byte (0x7C–0x7F): the block produces exactly one value.
#      Callers use a ValueType directly for this case.
#
#   3. A signed LEB128 integer >= 0: the block's result type is described
#      by the function type at that index in the type section.
#      This is the "multi-value" extension from WASM 1.1 but its encoding
#      was part of the original 1.0 spec as a forward-compatibility hook.
#
# Encoding layout:
#
#    ┌────────────┬──────────────────────────────────────────────────────┐
#    │ Byte range │ Meaning                                               │
#    ├────────────┼──────────────────────────────────────────────────────┤
#    │   0x40     │ empty block type (EMPTY constant below)              │
#    │  0x7C–0x7F │ single-result block (use ValueType enum directly)    │
#    │  >= 0x00   │ type index (multi-value, use a plain int)            │
#    └────────────┴──────────────────────────────────────────────────────┘
#
# This package only defines EMPTY as a constant. The caller is responsible
# for handling the union:
#   BlockTypeValue = int | ValueType  (where int 0x40 means "EMPTY")
# ---------------------------------------------------------------------------
class BlockType(IntEnum):
    """Block type encoding for WASM structured control flow.

    EMPTY (0x40) means the block produces no values. For single-result
    blocks, use a ValueType directly. For multi-value blocks, use a
    non-negative integer type index.

    Example:
        >>> BlockType.EMPTY == 0x40
        True
        >>> # Single-result block — use ValueType directly:
        >>> block_type = ValueType.I32
        >>> # Multi-value block — use a type index:
        >>> block_type_idx = 3  # refers to type section entry #3
    """

    EMPTY = 0x40  # block produces no values (most common case)


# ---------------------------------------------------------------------------
# EXTERNAL KIND
#
# ExternalKind identifies what kind of entity an import or export refers to.
# Used in both the Import section and Export section of a WASM module.
#
# Binary encoding: a single byte 0x00–0x03.
#
#    ┌──────┬──────────┬──────────────────────────────────────────────────┐
#    │ Byte │ Kind     │ What it refers to                                │
#    ├──────┼──────────┼──────────────────────────────────────────────────┤
#    │ 0x00 │ FUNCTION │ A function (identified by type index)            │
#    │ 0x01 │ TABLE    │ A table (funcref entries)                        │
#    │ 0x02 │ MEMORY   │ Linear memory (byte-addressed, page-granular)    │
#    │ 0x03 │ GLOBAL   │ A global variable                                │
#    └──────┴──────────┴──────────────────────────────────────────────────┘
#
# Example: an import like `(import "env" "memory" (memory 1))` has kind=MEMORY.
# ---------------------------------------------------------------------------
class ExternalKind(IntEnum):
    """The four kinds of importable/exportable WASM entities.

    Byte values match the WASM binary encoding (Import and Export sections).

    Example:
        >>> ExternalKind.FUNCTION == 0x00
        True
        >>> ExternalKind.GLOBAL == 3
        True
    """

    FUNCTION = 0x00  # a callable function
    TABLE = 0x01     # a table of opaque references (funcref)
    MEMORY = 0x02    # linear memory (64 KiB pages)
    GLOBAL = 0x03    # a mutable or immutable global variable


# ---------------------------------------------------------------------------
# FUNC TYPE
#
# FuncType describes a function's signature: the types of its parameters
# and the types of its results. In WASM 1.0, a function has at most one
# result value (the multi-value extension was added in 1.1).
#
# Binary encoding in the Type section:
#
#    0x60             — functype indicator byte
#    <LEB128 count>   — number of params
#    <param bytes>    — one ValueType byte per param
#    <LEB128 count>   — number of results
#    <result bytes>   — one ValueType byte per result
#
# Example — (i32, i64) -> f64:
#
#    0x60  0x02  0x7F 0x7E  0x01  0x7C
#    ^^^^  ^^^^  ^^^^^^^^^^  ^^^^  ^^^^
#    func  2 params i32,i64  1 res  f64
#
# The type section stores a vector of FuncType entries indexed from 0.
# The Function section then stores a vector of indices into this type table,
# giving each function its signature without repeating the full type.
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class FuncType:
    """A WASM function type: parameter types and result types.

    Attributes:
        params:  Tuple of ValueType for each parameter (left to right).
        results: Tuple of ValueType for each result value (usually 0 or 1).

    Immutable (frozen=True) so it can be used as a dict key or in a set.

    Example:
        >>> ft = FuncType(params=(ValueType.I32,), results=(ValueType.I64,))
        >>> ft.params
        (ValueType.I32,)
        >>> ft == FuncType(params=(ValueType.I32,), results=(ValueType.I64,))
        True
        >>> FuncType(params=(), results=())  # void -> void
        FuncType(params=(), results=())
    """

    params: tuple[ValueType, ...]
    results: tuple[ValueType, ...]


# ---------------------------------------------------------------------------
# LIMITS
#
# Limits specify a minimum (and optional maximum) count for tables and
# memories. For memories the unit is 64-KiB pages; for tables the unit is
# element slots.
#
# Binary encoding:
#
#    0x00  <min>           — only minimum (no maximum)
#    0x01  <min>  <max>    — minimum and maximum
#
# Where <min> and <max> are LEB128-encoded unsigned integers.
#
# Example — a memory that starts with 1 page and can grow to 4 pages:
#    0x01  0x01  0x04
#    ^^^^  ^^^^  ^^^^
#    has-max  1    4
#
# A memory with only a minimum (unbounded growth):
#    0x00  0x01
#    ^^^^  ^^^^
#    no-max  1
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Limits:
    """Size constraints for WASM memories and tables.

    Attributes:
        min: Minimum size (pages for memory, elements for tables). Required.
        max: Maximum size, or None if unbounded.

    Example:
        >>> Limits(min=1)          # at least 1 page, no upper limit
        Limits(min=1, max=None)
        >>> Limits(min=0, max=10)  # 0 to 10 elements
        Limits(min=0, max=10)
        >>> Limits(min=1).max is None
        True
    """

    min: int
    max: int | None = None


# ---------------------------------------------------------------------------
# MEMORY TYPE
#
# A MemoryType describes the linear memory of a WASM module. WASM 1.0 allows
# at most one memory per module, though it may be imported or defined locally.
# The memory is a flat array of bytes, addressed by i32 values, and measured
# in 64-KiB pages (page = 65536 bytes).
#
# Binary encoding (Memory section):
#
#    <limits>     — the Limits struct encoded as described above
#
# Example (1 initial page, max 2 pages):
#    0x01  0x01  0x02
#    ^^^^  ^^^^  ^^^^
#    has-max  1    2
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class MemoryType:
    """Describes a WASM linear memory (pages of 64 KiB each).

    Attributes:
        limits: Minimum and optional maximum size in 64-KiB pages.

    Example:
        >>> mt = MemoryType(limits=Limits(min=1, max=4))
        >>> mt.limits.min
        1
        >>> mt.limits.max
        4
    """

    limits: Limits


# ---------------------------------------------------------------------------
# TABLE TYPE
#
# A table is an array of opaque references — in WASM 1.0, always funcref
# (0x70). Tables are used by the call_indirect instruction to dispatch
# through a function pointer stored in the table.
#
# Binary encoding (Table section):
#
#    <element_type>  <limits>
#    0x70            ...
#
# The element_type 0x70 = funcref is the only valid element type in WASM 1.0.
# WASM proposals add externref (0x6F) and other types, but this package
# covers only the 1.0 spec.
#
# Example — table with 1 to 16 function references:
#    0x70  0x01  0x01  0x10
#    ^^^^  ^^^^  ^^^^  ^^^^
#    funcref  limits min=1 max=16
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class TableType:
    """Describes a WASM table (array of opaque references).

    Attributes:
        element_type: Reference type stored in the table. 0x70 = funcref
                      (the only valid value in WASM 1.0).
        limits:       Minimum and optional maximum number of elements.

    Example:
        >>> tt = TableType(limits=Limits(min=0, max=100))
        >>> tt.element_type
        112
        >>> hex(tt.element_type)
        '0x70'
    """

    element_type: int = 0x70  # funcref — the only type allowed in WASM 1.0
    limits: Limits = field(default_factory=lambda: Limits(min=0))


# ---------------------------------------------------------------------------
# GLOBAL TYPE
#
# A GlobalType describes a global variable: its value type and whether
# it is mutable.
#
# Binary encoding:
#
#    <value_type>  <mutability>
#    0x7F          0x01
#    ^^^^          ^^^^
#    i32           mutable (0x00 = immutable, 0x01 = mutable)
#
# Example — immutable i32 global (e.g., a constant like __data_end):
#    0x7F  0x00
#
# Example — mutable i32 global (e.g., a stack pointer):
#    0x7F  0x01
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class GlobalType:
    """Describes a WASM global variable: its type and mutability.

    Attributes:
        value_type: The type of the global's value (i32, i64, f32, f64).
        mutable:    True if the global can be modified by set_global;
                    False if it is a constant (read-only) global.

    Example:
        >>> gt = GlobalType(value_type=ValueType.I32, mutable=False)
        >>> gt.mutable
        False
        >>> GlobalType(value_type=ValueType.F64, mutable=True)
        GlobalType(value_type=ValueType.F64, mutable=True)
    """

    value_type: ValueType
    mutable: bool


# ---------------------------------------------------------------------------
# IMPORT
#
# An import brings an external entity into the module's namespace.
# WASM modules can import functions, tables, memories, and globals.
#
# Binary encoding (Import section entry):
#
#    <module_name_len>  <module_name_bytes>  — LEB128 length + UTF-8
#    <name_len>         <name_bytes>         — LEB128 length + UTF-8
#    <kind>                                  — 1 byte (ExternalKind)
#    <type_info>                             — depends on kind:
#        FUNCTION: LEB128 type index (int)
#        TABLE:    TableType
#        MEMORY:   MemoryType
#        GLOBAL:   GlobalType
#
# Example (WASI import of fd_write):
#    module_name = "wasi_snapshot_preview1"
#    name        = "fd_write"
#    kind        = ExternalKind.FUNCTION
#    type_info   = 3  (index into type section)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Import:
    """A WASM import entry: an entity brought in from the host environment.

    Attributes:
        module_name: The module namespace string (e.g., "env",
                     "wasi_snapshot_preview1").
        name:        The entity name within that module (e.g., "memory", "fd_write").
        kind:        What kind of entity is being imported (ExternalKind).
        type_info:   For FUNCTION: int (type index). For TABLE: TableType.
                     For MEMORY: MemoryType. For GLOBAL: GlobalType.

    Example:
        >>> imp = Import(
        ...     module_name="env",
        ...     name="memory",
        ...     kind=ExternalKind.MEMORY,
        ...     type_info=MemoryType(limits=Limits(min=1)),
        ... )
        >>> imp.kind
        ExternalKind.MEMORY
    """

    module_name: str
    name: str
    kind: ExternalKind
    type_info: int | TableType | MemoryType | GlobalType


# ---------------------------------------------------------------------------
# EXPORT
#
# An export makes a module's internal entity available to the host or other
# modules. An export has a name, a kind, and an index into the appropriate
# module section.
#
# Binary encoding (Export section entry):
#
#    <name_len>  <name_bytes>  <kind>  <index>
#    LEB128      UTF-8         1 byte  LEB128
#
# The index refers to:
#    FUNCTION: index into function address space (imports first, then locals)
#    TABLE:    index into table space
#    MEMORY:   index into memory space (always 0 in WASM 1.0)
#    GLOBAL:   index into global space
#
# Example — exporting the "main" function at index 5:
#    name  = "main"
#    kind  = ExternalKind.FUNCTION
#    index = 5
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Export:
    """A WASM export entry: makes an internal entity accessible externally.

    Attributes:
        name:  The export name (visible to the host or linking tools).
        kind:  What kind of entity is exported (ExternalKind).
        index: Index into the appropriate address space for that kind.

    Example:
        >>> exp = Export(name="main", kind=ExternalKind.FUNCTION, index=0)
        >>> exp.name
        'main'
        >>> exp.index
        0
    """

    name: str
    kind: ExternalKind
    index: int


# ---------------------------------------------------------------------------
# GLOBAL
#
# A module-defined global variable. Combines a GlobalType (type + mutability)
# with an initializer expression. The initializer is a constant expression
# encoded as a byte sequence (terminated by 0x0B = end opcode).
#
# Binary encoding (Global section entry):
#
#    <global_type>   — GlobalType (value_type + mutability)
#    <init_expr>     — constant expression bytes (ends with 0x0B)
#
# Common initializer expressions:
#    i32.const N: 0x41 <leb128(N)> 0x0B
#    i64.const N: 0x42 <leb128(N)> 0x0B
#    f32.const N: 0x43 <4 bytes>   0x0B
#    f64.const N: 0x44 <8 bytes>   0x0B
#    global.get K: 0x23 <leb128(K)> 0x0B
#
# Example — mutable i32 global initialized to 42:
#    global_type = GlobalType(ValueType.I32, mutable=True)
#    init_expr   = bytes([0x41, 0x2A, 0x0B])   # i32.const 42; end
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Global:
    """A WASM module-defined global variable with its initializer.

    Attributes:
        global_type: The type and mutability of the global.
        init_expr:   Constant initializer expression bytes (ends with 0x0B).

    Example:
        >>> g = Global(
        ...     global_type=GlobalType(ValueType.I32, mutable=True),
        ...     init_expr=bytes([0x41, 0x2A, 0x0B]),  # i32.const 42
        ... )
        >>> g.global_type.mutable
        True
        >>> g.init_expr[0]
        65
    """

    global_type: GlobalType
    init_expr: bytes


# ---------------------------------------------------------------------------
# ELEMENT
#
# An element segment initializes entries in a table. It specifies which table
# to initialize, an offset expression that computes where to start, and a
# list of function indices to write into the table.
#
# Binary encoding (Element section entry):
#
#    <table_index>   — LEB128 (always 0 in WASM 1.0)
#    <offset_expr>   — constant expression bytes (i32.const + end)
#    <count>         — LEB128 number of function indices
#    <func_indices>  — LEB128 per function index
#
# Example — fill table[0..3] starting at offset 0 with functions 1,2,3:
#    table_index    = 0
#    offset_expr    = bytes([0x41, 0x00, 0x0B])  # i32.const 0; end
#    function_indices = (1, 2, 3)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Element:
    """A WASM element segment: initializes table entries with function refs.

    Attributes:
        table_index:      Index of the table to initialize (always 0 in 1.0).
        offset_expr:      Constant expression giving the start offset in the table.
        function_indices: Tuple of function indices to write, starting at offset.

    Example:
        >>> elem = Element(
        ...     table_index=0,
        ...     offset_expr=bytes([0x41, 0x00, 0x0B]),
        ...     function_indices=(1, 2, 3),
        ... )
        >>> len(elem.function_indices)
        3
    """

    table_index: int
    offset_expr: bytes
    function_indices: tuple[int, ...]


# ---------------------------------------------------------------------------
# DATA SEGMENT
#
# A data segment initializes a region of linear memory with a byte string.
# It specifies which memory (always 0 in WASM 1.0), an offset expression,
# and the bytes to write.
#
# Binary encoding (Data section entry):
#
#    <memory_index>  — LEB128 (always 0 in WASM 1.0)
#    <offset_expr>   — constant expression bytes
#    <count>         — LEB128 byte length
#    <data>          — raw bytes
#
# Example — write "hello" at memory[0x100]:
#    memory_index = 0
#    offset_expr  = bytes([0x41, 0x80, 0x02, 0x0B])  # i32.const 256; end
#    data         = b"hello"
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class DataSegment:
    """A WASM data segment: initializes a region of linear memory.

    Attributes:
        memory_index: Index of the memory to initialize (always 0 in 1.0).
        offset_expr:  Constant expression giving the byte offset in memory.
        data:         The raw bytes to copy into memory at that offset.

    Example:
        >>> ds = DataSegment(
        ...     memory_index=0,
        ...     offset_expr=bytes([0x41, 0x00, 0x0B]),
        ...     data=b"hello",
        ... )
        >>> ds.data
        b'hello'
    """

    memory_index: int
    offset_expr: bytes
    data: bytes


# ---------------------------------------------------------------------------
# FUNCTION BODY
#
# A function body is the executable content of a locally-defined function.
# It contains a list of local variable declarations (beyond the parameters,
# which are declared in FuncType) and the raw bytecode instructions.
#
# Binary encoding (Code section entry):
#
#    <body_size>   — LEB128 total byte size of this entry
#    <local_count> — LEB128 number of local variable groups
#    <locals>      — each group: (count: LEB128, type: ValueType byte)
#    <code>        — raw bytecode bytes (ends with 0x0B = end opcode)
#
# The locals field here stores one ValueType per local (already expanded).
# The raw binary groups (count, type) are only relevant during parsing.
#
# Example — function body with 2 i32 locals and a simple add:
#    locals = (ValueType.I32, ValueType.I32)
#    code   = bytes([0x20, 0x00,   # local.get 0
#                    0x20, 0x01,   # local.get 1
#                    0x6A,          # i32.add
#                    0x0B])         # end
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class FunctionBody:
    """The body of a WASM function: local variable types and bytecode.

    Attributes:
        locals: Tuple of ValueType for each local variable (not including params).
        code:   Raw bytecode bytes for the function (ends with 0x0B = end).

    Example:
        >>> fb = FunctionBody(
        ...     locals=(ValueType.I32,),
        ...     code=bytes([0x20, 0x00, 0x0B]),  # local.get 0; end
        ... )
        >>> len(fb.locals)
        1
        >>> fb.code[-1]
        11
    """

    locals: tuple[ValueType, ...]
    code: bytes


# ---------------------------------------------------------------------------
# CUSTOM SECTION
#
# Custom sections carry non-standard data outside the core WASM spec.
# The WASM runtime ignores sections it doesn't understand, making custom
# sections a safe extension point.
#
# Common custom section uses:
#   - "name":       WASM name section — maps function indices to debug names
#   - ".debug_info": DWARF debug information (Emscripten, wasm-pack)
#   - "producers":  Toolchain metadata (which compiler was used)
#
# Binary encoding:
#
#    0x00              — custom section ID
#    <section_size>    — LEB128
#    <name_len>        — LEB128
#    <name_bytes>      — UTF-8
#    <data>            — arbitrary bytes (section_size - name_len - 1 bytes)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class CustomSection:
    """A WASM custom section: named blob of arbitrary data.

    Attributes:
        name: The section's name (e.g., "name", "producers").
        data: The raw payload bytes following the name.

    Example:
        >>> cs = CustomSection(name="name", data=b"\\x00\\x04main")
        >>> cs.name
        'name'
        >>> len(cs.data)
        6
    """

    name: str
    data: bytes


# ---------------------------------------------------------------------------
# WASM MODULE
#
# WasmModule is the top-level container for a parsed WASM binary. It holds
# one list per section type. Fields start empty; a parser fills them in
# section by section as it reads through the binary.
#
# The module is deliberately mutable (a regular dataclass, not frozen).
# This makes it easy for a streaming parser to build up the module
# incrementally without knowing the final sizes up front.
#
# Field naming follows the WASM spec section names:
#
#    types     ← Type section     (list of FuncType)
#    imports   ← Import section   (list of Import)
#    functions ← Function section (list of int — type indices for local funcs)
#    tables    ← Table section    (list of TableType)
#    memories  ← Memory section   (list of MemoryType)
#    globals   ← Global section   (list of Global)
#    exports   ← Export section   (list of Export)
#    start     ← Start section    (int index, or None if absent)
#    elements  ← Element section  (list of Element)
#    code      ← Code section     (list of FunctionBody)
#    data      ← Data section     (list of DataSegment)
#    customs   ← Custom sections  (list of CustomSection)
#
# The function address space merges imported and local functions:
#   index 0 .. len(imports with kind=FUNCTION)-1  → imported functions
#   index len(imports) .. len(imports)+len(functions)-1  → local functions
#
# Example usage:
#    module = WasmModule()
#    module.types.append(FuncType(params=(ValueType.I32,), results=()))
#    module.functions.append(0)  # local function uses type index 0
# ---------------------------------------------------------------------------
@dataclass
class WasmModule:
    """Top-level container for a parsed WASM 1.0 module.

    All fields are lists; they start empty and are populated by a parser.
    This class is intentionally mutable — parsers fill it section by section.

    Attributes:
        types:     FuncType entries from the Type section.
        imports:   Import entries from the Import section.
        functions: Type indices (ints) for each locally-defined function.
        tables:    TableType entries from the Table section.
        memories:  MemoryType entries from the Memory section.
        globals:   Global entries from the Global section.
        exports:   Export entries from the Export section.
        start:     Index of the start function, or None if absent.
        elements:  Element entries from the Element section.
        code:      FunctionBody entries from the Code section.
        data:      DataSegment entries from the Data section.
        customs:   CustomSection entries (may appear multiple times).

    Example:
        >>> m = WasmModule()
        >>> m.types
        []
        >>> m.types.append(FuncType(params=(), results=()))
        >>> len(m.types)
        1
        >>> m.start is None
        True
    """

    types: list[FuncType] = field(default_factory=list)
    imports: list[Import] = field(default_factory=list)
    functions: list[int] = field(default_factory=list)
    tables: list[TableType] = field(default_factory=list)
    memories: list[MemoryType] = field(default_factory=list)
    globals: list[Global] = field(default_factory=list)
    exports: list[Export] = field(default_factory=list)
    start: int | None = None
    elements: list[Element] = field(default_factory=list)
    code: list[FunctionBody] = field(default_factory=list)
    data: list[DataSegment] = field(default_factory=list)
    customs: list[CustomSection] = field(default_factory=list)
