# W01 — WebAssembly Runtime

## Overview

A complete WebAssembly 1.0 runtime built from **seven composable packages**, each
independently testable and publishable. The runtime can parse real `.wasm` binary
modules, validate them, and execute all ~182 WASM 1.0 instructions. A pluggable
host interface (WASI-like) provides I/O for both browser and non-browser environments.

This is distinct from the existing `wasm-simulator` (spec 07c), which is a minimal
6-instruction teaching tool. The runtime is a production-style execution engine that
handles the full WASM 1.0 specification.

## Layer Position

```
                     ┌─────────────────────────────────────────┐
                     │         W01 — WASM Runtime              │
                     │                                         │
                     │  wasm-runtime (orchestrator)            │
                     │    ├── wasm-module-parser                │
                     │    │     ├── wasm-leb128                 │
                     │    │     ├── wasm-types                  │
                     │    │     └── wasm-opcodes                │
                     │    ├── wasm-validator                    │
                     │    └── wasm-execution                    │
                     │          ├── instructions (all ~182)     │
                     │          ├── linear memory               │
                     │          ├── tables                      │
                     │          └── host interface (WASI)       │
                     └─────────────────────────────────────────┘
                                       │
                          ┌────────────┴────────────┐
                          │                         │
                    "Real" execution          "Simulated" execution
                    (direct interpret)        (future: WASM → RISC-V
                                              → pipeline → gates)
```

The runtime sits at the same layer as the ISA simulators (Layer 7) but provides a
higher-level execution model. The "real" path interprets WASM directly. The "simulated"
path (future `wasm-compiler-backend` package) will compile WASM to RISC-V and execute
on the full simulated CPU, traceable down to logic gates.

## Package Decomposition

Seven packages, ordered by dependency:

```
wasm-leb128          (no dependencies)
     │
wasm-types           (depends on: wasm-leb128)
     │
wasm-opcodes         (depends on: wasm-types)
     │
wasm-module-parser   (depends on: wasm-leb128, wasm-types, wasm-opcodes)
     │
     ├── wasm-validator   (depends on: wasm-types, wasm-opcodes, wasm-module-parser)
     │
     └── wasm-execution   (depends on: wasm-types, wasm-opcodes, wasm-module-parser)
              │
         wasm-runtime     (depends on: ALL of the above)
```

Each package is implemented in all 6 languages: Python, Go, TypeScript, Ruby, Rust,
and Elixir.

---

## Package 1: `wasm-leb128`

### What Is LEB128?

LEB128 (Little-Endian Base 128) is a variable-length encoding for integers. WASM uses
it for all integer values in the binary format — section sizes, type indices, function
indices, instruction immediates, and more.

The idea: use 7 bits of each byte for data and 1 bit (the high bit) as a continuation
flag. If the high bit is 1, read another byte. If 0, this is the last byte.

```
Value 624485 in unsigned LEB128:

  624485 = 0b 0010_0110_0001_0000_1110_0101

  Split into 7-bit groups (little-endian):
    0b 1100101  = 0x65  → byte 0: 0xE5 (high bit = 1, more bytes)
    0b 0001000  = 0x08  → byte 1: 0x88 (high bit = 1, more bytes)
    0b 0100110  = 0x26  → byte 2: 0x26 (high bit = 0, last byte)

  Encoded: [0xE5, 0x88, 0x26]
```

Signed LEB128 uses the same scheme but with sign extension on the final byte.

### Public API

```python
def decode_unsigned(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode unsigned LEB128 integer.

    Args:
        data: The byte sequence to decode from.
        offset: Starting position in data.

    Returns:
        (value, bytes_consumed) — the decoded integer and how many bytes were read.

    Raises:
        LEB128Error: If the encoding is malformed (unterminated, overflow).

    Example:
        >>> decode_unsigned(bytes([0xE5, 0x88, 0x26]), 0)
        (624485, 3)
    """

def decode_signed(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode signed LEB128 integer (two's complement).

    Example:
        >>> decode_signed(bytes([0x7E]), 0)  # -2 in signed LEB128
        (-2, 1)
    """

def encode_unsigned(value: int) -> bytes:
    """Encode non-negative integer as unsigned LEB128.

    Example:
        >>> encode_unsigned(624485)
        b'\\xe5\\x88\\x26'
    """

def encode_signed(value: int) -> bytes:
    """Encode integer as signed LEB128 (two's complement).

    Example:
        >>> encode_signed(-2)
        b'\\x7e'
    """
```

### Test Strategy

| Test | Input | Expected |
|------|-------|----------|
| Zero | `[0x00]` | unsigned=0, signed=0 |
| One byte unsigned | `[0x03]` | 3 |
| One byte signed negative | `[0x7E]` | -2 |
| Multi-byte | `[0xE5, 0x88, 0x26]` | 624485 |
| Max u32 | `[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]` | 4294967295 |
| Max i32 positive | `[0xFF, 0xFF, 0xFF, 0xFF, 0x07]` | 2147483647 |
| Min i32 negative | `[0x80, 0x80, 0x80, 0x80, 0x78]` | -2147483648 |
| Round-trip | encode then decode | original value |
| Unterminated | `[0x80, 0x80]` (no terminator) | error |

---

## Package 2: `wasm-types`

### What Are WASM Types?

WebAssembly has a small, well-defined type system. Every value on the stack, every
local variable, every function parameter and return value has a type. Types are central
to both validation and execution.

### Type Definitions

```python
from enum import IntEnum

class ValueType(IntEnum):
    """The four value types in WASM 1.0.

    These are the only types that can appear on the operand stack or in
    local/global variables. Each has a specific byte encoding in the binary format.
    """
    I32 = 0x7F   # 32-bit integer (signed or unsigned, depending on instruction)
    I64 = 0x7E   # 64-bit integer
    F32 = 0x7D   # 32-bit IEEE 754 float
    F64 = 0x7C   # 64-bit IEEE 754 float


@dataclass(frozen=True)
class FuncType:
    """A function signature: the types of its parameters and return values.

    In the binary format, encoded as: 0x60 <params as vec(valtype)> <results as vec(valtype)>

    Example: A function (i32, i32) → (i32) has params=[I32, I32], results=[I32].
    """
    params: tuple[ValueType, ...]
    results: tuple[ValueType, ...]


@dataclass(frozen=True)
class Limits:
    """Size constraints for memories and tables.

    min: The initial size (in pages for memory, in elements for tables).
    max: The maximum allowed size, or None if unbounded.

    A memory page is 65536 bytes (64 KiB).
    """
    min: int
    max: int | None = None


@dataclass(frozen=True)
class MemoryType:
    """Describes a linear memory's size constraints.

    limits.min and limits.max are in units of pages (64 KiB each).
    WASM 1.0 allows at most 1 memory per module.
    """
    limits: Limits


@dataclass(frozen=True)
class TableType:
    """Describes a table's element type and size constraints.

    In WASM 1.0, the element type is always funcref (0x70) — tables
    store function references for indirect calls (call_indirect).
    """
    element_type: int   # 0x70 = funcref
    limits: Limits


@dataclass(frozen=True)
class GlobalType:
    """A global variable's type and mutability.

    mutable=False means the global is a constant (set once during init).
    mutable=True means it can be modified with global.set.
    """
    value_type: ValueType
    mutable: bool


class BlockType:
    """The type of a structured control flow block (block/loop/if).

    Three forms:
    - Empty: 0x40 — the block produces no values
    - Single value: a ValueType byte — the block produces one value
    - Type index: a signed LEB128 — references a FuncType for multi-value
    """
    EMPTY = 0x40


class ExternalKind(IntEnum):
    """The kind of entity being imported or exported.

    Used in Import and Export sections.
    """
    FUNCTION = 0x00
    TABLE    = 0x01
    MEMORY   = 0x02
    GLOBAL   = 0x03
```

### Supporting Data Structures

```python
@dataclass(frozen=True)
class Import:
    """An imported entity from another module.

    module_name: The module to import from (e.g., "wasi_snapshot_preview1").
    name: The entity name within that module (e.g., "fd_write").
    kind: What kind of entity (function, table, memory, global).
    type_info: The type descriptor (FuncType index, TableType, MemoryType, or GlobalType).
    """
    module_name: str
    name: str
    kind: ExternalKind
    type_info: int | TableType | MemoryType | GlobalType


@dataclass(frozen=True)
class Export:
    """An exported entity from this module.

    name: The export name (e.g., "main", "memory").
    kind: What kind of entity.
    index: The index into the corresponding index space.
    """
    name: str
    kind: ExternalKind
    index: int


@dataclass(frozen=True)
class Global:
    """A global variable with its type and initialization expression.

    The init_expr is a constant expression (typically a single i32.const or global.get).
    """
    global_type: GlobalType
    init_expr: bytes


@dataclass(frozen=True)
class Element:
    """A table initializer segment.

    Copies function references into a table at instantiation time.
    """
    table_index: int
    offset_expr: bytes
    function_indices: tuple[int, ...]


@dataclass(frozen=True)
class DataSegment:
    """A memory initializer segment.

    Copies bytes into linear memory at instantiation time.
    """
    memory_index: int
    offset_expr: bytes
    data: bytes


@dataclass(frozen=True)
class FunctionBody:
    """A function's compiled body from the Code section.

    locals: Declared local variable types (not including params — those come from FuncType).
    code: Raw bytecode of the function body (instructions ending with 'end').
    """
    locals: tuple[ValueType, ...]
    code: bytes


@dataclass(frozen=True)
class CustomSection:
    """An uninterpreted custom section (used for debug info, names, etc.)."""
    name: str
    data: bytes
```

---

## Package 3: `wasm-opcodes`

### Complete WASM 1.0 Opcode Table

Every WASM instruction has a single-byte opcode (some prefixed with 0xFC for newer
extensions, but WASM 1.0 core uses single bytes). Each opcode has metadata describing
its immediates, stack effect, and category.

```python
@dataclass(frozen=True)
class OpcodeInfo:
    """Metadata for a single WASM opcode.

    name: Human-readable instruction name (e.g., "i32.add").
    opcode: The byte value (e.g., 0x6A).
    category: Instruction category (e.g., "numeric_i32", "control", "memory").
    immediates: List of immediate operand types (e.g., ["blocktype"], ["memarg"]).
    stack_pop: Number of values consumed from the stack.
    stack_push: Number of values pushed onto the stack.
    """
    name: str
    opcode: int
    category: str
    immediates: tuple[str, ...]
    stack_pop: int
    stack_push: int
```

### Control Instructions (0x00–0x11)

Control instructions manage the flow of execution. WASM uses **structured control
flow** — there are no arbitrary gotos. Instead, `block`, `loop`, and `if` create
labeled scopes that `br` (branch) can jump to.

| Opcode | Name | Immediates | Pop | Push | Notes |
|--------|------|-----------|-----|------|-------|
| 0x00 | unreachable | — | 0 | 0 | Always traps |
| 0x01 | nop | — | 0 | 0 | No operation |
| 0x02 | block | blocktype | 0 | 0 | Start a block (br jumps to end) |
| 0x03 | loop | blocktype | 0 | 0 | Start a loop (br jumps to start) |
| 0x04 | if | blocktype | 1 | 0 | Pop condition, enter then/else |
| 0x05 | else | — | 0 | 0 | Start else branch |
| 0x0B | end | — | 0 | 0 | End block/loop/if/function |
| 0x0C | br | labelidx | 0 | 0 | Branch to enclosing label |
| 0x0D | br_if | labelidx | 1 | 0 | Branch if top != 0 |
| 0x0E | br_table | vec(labelidx) + labelidx | 1 | 0 | Indexed branch |
| 0x0F | return | — | varies | 0 | Return from function |
| 0x10 | call | funcidx | varies | varies | Call function by index |
| 0x11 | call_indirect | typeidx + tableidx | varies | varies | Indirect call via table |

**How structured control flow works:**

```
block (result i32)      ;; label L0 — br 0 jumps to END
  i32.const 42
  br 0                  ;; jump to L0's end, carrying 42
  i32.const 99          ;; unreachable
end                     ;; ← br 0 lands here; stack has [42]

loop (result void)      ;; label L0 — br 0 jumps to START
  ;; loop body
  br 0                  ;; jump back to loop start
end
```

### Parametric Instructions (0x1A–0x1B)

| Opcode | Name | Pop | Push | Notes |
|--------|------|-----|------|-------|
| 0x1A | drop | 1 | 0 | Discard top value |
| 0x1B | select | 3 | 1 | Pop (c, b, a): push a if c≠0, else b |

### Variable Instructions (0x20–0x24)

| Opcode | Name | Immediates | Pop | Push | Notes |
|--------|------|-----------|-----|------|-------|
| 0x20 | local.get | localidx | 0 | 1 | Push local variable |
| 0x21 | local.set | localidx | 1 | 0 | Pop into local variable |
| 0x22 | local.tee | localidx | 1 | 1 | Set local, keep value on stack |
| 0x23 | global.get | globalidx | 0 | 1 | Push global variable |
| 0x24 | global.set | globalidx | 1 | 0 | Pop into mutable global |

### Memory Instructions (0x28–0x40)

Memory instructions access linear memory. Each load/store has a **memarg** immediate
consisting of an alignment hint and an offset, both encoded as unsigned LEB128.

The effective address is: `stack_operand + offset`.

**Loads:**

| Opcode | Name | Width | Sign Extension |
|--------|------|-------|---------------|
| 0x28 | i32.load | 4 bytes | — |
| 0x29 | i64.load | 8 bytes | — |
| 0x2A | f32.load | 4 bytes | — |
| 0x2B | f64.load | 8 bytes | — |
| 0x2C | i32.load8_s | 1 byte | sign-extend to i32 |
| 0x2D | i32.load8_u | 1 byte | zero-extend to i32 |
| 0x2E | i32.load16_s | 2 bytes | sign-extend to i32 |
| 0x2F | i32.load16_u | 2 bytes | zero-extend to i32 |
| 0x30 | i64.load8_s | 1 byte | sign-extend to i64 |
| 0x31 | i64.load8_u | 1 byte | zero-extend to i64 |
| 0x32 | i64.load16_s | 2 bytes | sign-extend to i64 |
| 0x33 | i64.load16_u | 2 bytes | zero-extend to i64 |
| 0x34 | i64.load32_s | 4 bytes | sign-extend to i64 |
| 0x35 | i64.load32_u | 4 bytes | zero-extend to i64 |

**Stores:**

| Opcode | Name | Width |
|--------|------|-------|
| 0x36 | i32.store | 4 bytes |
| 0x37 | i64.store | 8 bytes |
| 0x38 | f32.store | 4 bytes |
| 0x39 | f64.store | 8 bytes |
| 0x3A | i32.store8 | 1 byte (truncate) |
| 0x3B | i32.store16 | 2 bytes (truncate) |
| 0x3C | i64.store8 | 1 byte (truncate) |
| 0x3D | i64.store16 | 2 bytes (truncate) |
| 0x3E | i64.store32 | 4 bytes (truncate) |

**Memory management:**

| Opcode | Name | Pop | Push | Notes |
|--------|------|-----|------|-------|
| 0x3F | memory.size | 0 | 1 | Push current size in pages |
| 0x40 | memory.grow | 1 | 1 | Grow by N pages, push old size (or -1 on failure) |

### Numeric Instructions — i32 (0x41, 0x45–0x4F, 0x67–0x78)

**Constants:**

| Opcode | Name | Immediates | Push |
|--------|------|-----------|------|
| 0x41 | i32.const | i32 (signed LEB128) | 1 |

**Comparisons (all pop 2, push 1 i32 boolean):**

| Opcode | Name | Operation |
|--------|------|-----------|
| 0x45 | i32.eqz | a == 0 (unary, pops 1) |
| 0x46 | i32.eq | a == b |
| 0x47 | i32.ne | a ≠ b |
| 0x48 | i32.lt_s | a < b (signed) |
| 0x49 | i32.lt_u | a < b (unsigned) |
| 0x4A | i32.gt_s | a > b (signed) |
| 0x4B | i32.gt_u | a > b (unsigned) |
| 0x4C | i32.le_s | a ≤ b (signed) |
| 0x4D | i32.le_u | a ≤ b (unsigned) |
| 0x4E | i32.ge_s | a ≥ b (signed) |
| 0x4F | i32.ge_u | a ≥ b (unsigned) |

**Unary operations (pop 1, push 1):**

| Opcode | Name | Operation |
|--------|------|-----------|
| 0x67 | i32.clz | Count leading zeros |
| 0x68 | i32.ctz | Count trailing zeros |
| 0x69 | i32.popcnt | Count set bits |

**Binary operations (pop 2, push 1):**

| Opcode | Name | Operation |
|--------|------|-----------|
| 0x6A | i32.add | a + b (wrapping) |
| 0x6B | i32.sub | a - b (wrapping) |
| 0x6C | i32.mul | a × b (wrapping) |
| 0x6D | i32.div_s | a ÷ b (signed, traps on zero/overflow) |
| 0x6E | i32.div_u | a ÷ b (unsigned, traps on zero) |
| 0x6F | i32.rem_s | a mod b (signed, traps on zero) |
| 0x70 | i32.rem_u | a mod b (unsigned, traps on zero) |
| 0x71 | i32.and | a & b |
| 0x72 | i32.or | a \| b |
| 0x73 | i32.xor | a ^ b |
| 0x74 | i32.shl | a << (b mod 32) |
| 0x75 | i32.shr_s | a >> (b mod 32) (arithmetic) |
| 0x76 | i32.shr_u | a >> (b mod 32) (logical) |
| 0x77 | i32.rotl | rotate left by (b mod 32) |
| 0x78 | i32.rotr | rotate right by (b mod 32) |

### Numeric Instructions — i64 (0x42, 0x50–0x5A, 0x79–0x8A)

Mirror of i32 but operating on 64-bit integers. Same operation names with `i64` prefix.

| Opcode | Name | Notes |
|--------|------|-------|
| 0x42 | i64.const | Immediate is i64 signed LEB128 |
| 0x50 | i64.eqz | Unary (pop 1 i64, push 1 i32) |
| 0x51–0x5A | i64.eq through i64.ge_u | Pop 2 i64, push 1 i32 |
| 0x79 | i64.clz | Pop 1 i64, push 1 i64 |
| 0x7A | i64.ctz | |
| 0x7B | i64.popcnt | |
| 0x7C–0x8A | i64.add through i64.rotr | Pop 2 i64, push 1 i64 |

### Numeric Instructions — f32 (0x43, 0x5B–0x60, 0x8B–0x98)

IEEE 754 single-precision floating point operations.

| Opcode | Name | Notes |
|--------|------|-------|
| 0x43 | f32.const | 4-byte IEEE 754 immediate |
| 0x5B | f32.eq | Pop 2 f32, push 1 i32 |
| 0x5C | f32.ne | |
| 0x5D | f32.lt | |
| 0x5E | f32.gt | |
| 0x5F | f32.le | |
| 0x60 | f32.ge | |
| 0x8B | f32.abs | Unary |
| 0x8C | f32.neg | Unary |
| 0x8D | f32.ceil | Unary |
| 0x8E | f32.floor | Unary |
| 0x8F | f32.trunc | Unary |
| 0x90 | f32.nearest | Unary |
| 0x91 | f32.sqrt | Unary |
| 0x92 | f32.add | Binary |
| 0x93 | f32.sub | Binary |
| 0x94 | f32.mul | Binary |
| 0x95 | f32.div | Binary |
| 0x96 | f32.min | Binary |
| 0x97 | f32.max | Binary |
| 0x98 | f32.copysign | Binary |

### Numeric Instructions — f64 (0x44, 0x61–0x66, 0x99–0xA6)

Mirror of f32 but operating on 64-bit double-precision floats.

| Opcode | Name | Notes |
|--------|------|-------|
| 0x44 | f64.const | 8-byte IEEE 754 immediate |
| 0x61–0x66 | f64.eq through f64.ge | Pop 2 f64, push 1 i32 |
| 0x99–0xA6 | f64.abs through f64.copysign | Same structure as f32 |

### Conversion Instructions (0xA7–0xBF)

These instructions convert between types. They are the "bridge" between the four
value types.

| Opcode | Name | Pop → Push | Notes |
|--------|------|-----------|-------|
| 0xA7 | i32.wrap_i64 | i64 → i32 | Keep low 32 bits |
| 0xA8 | i32.trunc_f32_s | f32 → i32 | Truncate, signed (traps on NaN/overflow) |
| 0xA9 | i32.trunc_f32_u | f32 → i32 | Truncate, unsigned |
| 0xAA | i32.trunc_f64_s | f64 → i32 | Truncate, signed |
| 0xAB | i32.trunc_f64_u | f64 → i32 | Truncate, unsigned |
| 0xAC | i64.extend_i32_s | i32 → i64 | Sign-extend |
| 0xAD | i64.extend_i32_u | i32 → i64 | Zero-extend |
| 0xAE | i64.trunc_f32_s | f32 → i64 | Truncate, signed |
| 0xAF | i64.trunc_f32_u | f32 → i64 | Truncate, unsigned |
| 0xB0 | i64.trunc_f64_s | f64 → i64 | Truncate, signed |
| 0xB1 | i64.trunc_f64_u | f64 → i64 | Truncate, unsigned |
| 0xB2 | f32.convert_i32_s | i32 → f32 | Signed int to float |
| 0xB3 | f32.convert_i32_u | i32 → f32 | Unsigned int to float |
| 0xB4 | f32.convert_i64_s | i64 → f32 | Signed long to float |
| 0xB5 | f32.convert_i64_u | i64 → f32 | Unsigned long to float |
| 0xB6 | f32.demote_f64 | f64 → f32 | Double to single |
| 0xB7 | f64.convert_i32_s | i32 → f64 | Signed int to double |
| 0xB8 | f64.convert_i32_u | i32 → f64 | Unsigned int to double |
| 0xB9 | f64.convert_i64_s | i64 → f64 | Signed long to double |
| 0xBA | f64.convert_i64_u | i64 → f64 | Unsigned long to double |
| 0xBB | f64.promote_f32 | f32 → f64 | Single to double |
| 0xBC | i32.reinterpret_f32 | f32 → i32 | Reinterpret bits |
| 0xBD | i64.reinterpret_f64 | f64 → i64 | Reinterpret bits |
| 0xBE | f32.reinterpret_i32 | i32 → f32 | Reinterpret bits |
| 0xBF | f64.reinterpret_i64 | i64 → f64 | Reinterpret bits |

---

## Package 4: `wasm-module-parser`

### Binary Format

A `.wasm` file starts with an 8-byte header, followed by a sequence of sections.

```
┌──────────────────────────────────────────────────┐
│ Magic:   0x00 0x61 0x73 0x6D  ("\0asm")          │
│ Version: 0x01 0x00 0x00 0x00  (version 1)        │
├──────────────────────────────────────────────────┤
│ Section 1: id(u8) + size(u32 LEB128) + payload   │
│ Section 2: id(u8) + size(u32 LEB128) + payload   │
│ ...                                               │
│ Section N: id(u8) + size(u32 LEB128) + payload   │
└──────────────────────────────────────────────────┘
```

Sections must appear in order of their IDs (except custom sections, which can appear
anywhere). Not all sections are required.

### Section Parsing

Each section has its own internal format. The parser reads each section's payload
and populates the corresponding field in `WasmModule`.

**Type Section (ID 1):**
```
count: u32 LEB128
for each type:
  0x60 (function type marker)
  param_count: u32 LEB128
  param_types: [valtype × param_count]
  result_count: u32 LEB128
  result_types: [valtype × result_count]
```

**Import Section (ID 2):**
```
count: u32 LEB128
for each import:
  module_name: name (u32 len + UTF-8 bytes)
  entity_name: name
  kind: u8 (0=func, 1=table, 2=memory, 3=global)
  type descriptor (depends on kind)
```

**Function Section (ID 3):**
```
count: u32 LEB128
type_indices: [u32 LEB128 × count]
```

**Table Section (ID 4):**
```
count: u32 LEB128
for each table:
  element_type: u8 (0x70 = funcref)
  limits: (flags:u8 + min:u32 [+ max:u32])
```

**Memory Section (ID 5):**
```
count: u32 LEB128
for each memory:
  limits: (flags:u8 + min:u32 [+ max:u32])
```

**Global Section (ID 6):**
```
count: u32 LEB128
for each global:
  value_type: u8
  mutable: u8 (0=const, 1=var)
  init_expr: expression (instructions ending with 0x0B)
```

**Export Section (ID 7):**
```
count: u32 LEB128
for each export:
  name: name (u32 len + UTF-8 bytes)
  kind: u8
  index: u32 LEB128
```

**Start Section (ID 8):**
```
function_index: u32 LEB128
```

**Element Section (ID 9):**
```
count: u32 LEB128
for each element:
  table_index: u32 LEB128
  offset_expr: expression
  func_count: u32 LEB128
  function_indices: [u32 LEB128 × func_count]
```

**Code Section (ID 10):**
```
count: u32 LEB128
for each function body:
  body_size: u32 LEB128
  local_count: u32 LEB128
  locals: [(count:u32, type:valtype) × local_count]
  code: bytes (remaining bytes, ending with 0x0B)
```

**Data Section (ID 11):**
```
count: u32 LEB128
for each segment:
  memory_index: u32 LEB128
  offset_expr: expression
  byte_count: u32 LEB128
  bytes: [u8 × byte_count]
```

### Public API

```python
class WasmModuleParser:
    def parse(self, data: bytes) -> WasmModule:
        """Parse a complete .wasm binary into a WasmModule.

        Raises WasmParseError with byte offset on malformed input.
        """

class WasmParseError(Exception):
    """Raised when .wasm binary is malformed."""
    def __init__(self, message: str, offset: int): ...
```

### Test Strategy

- Parse minimal valid module (just header, no sections)
- Parse module with each section type individually
- Parse complete module with multiple sections
- Verify section ordering validation
- Error cases: bad magic, bad version, truncated section, invalid section ID
- Round-trip: build binary manually → parse → verify fields match

---

## Package 5: `wasm-validator`

### Validation Algorithm

Validation ensures a parsed module is well-formed before execution. The core
algorithm uses **abstract interpretation** — it simulates execution with *types*
instead of values.

```
For each function in the module:
  1. Initialize type stack from function params
  2. Walk each instruction:
     - Check that stack has the required input types
     - Pop input types, push output types
     - For control flow: maintain a control stack tracking block types
  3. At function end: verify stack matches declared return types
```

### Validation Checks

| Category | Check | Error if violated |
|----------|-------|-------------------|
| Types | All type indices < len(module.types) | "type index out of bounds" |
| Functions | function_type_indices[i] < len(module.types) | "function type index out of bounds" |
| Stack | Instruction inputs match stack types | "type mismatch: expected i32, got f64" |
| Stack | Function end stack matches return type | "stack height mismatch at function end" |
| Control | br/br_if label index within enclosing blocks | "branch target out of range" |
| Control | Blocks properly nested | "mismatched block/end" |
| Memory | Memory index < len(module.memories) | "memory index out of bounds" |
| Globals | global.set only on mutable globals | "cannot set immutable global" |
| Start | Start function type is [] → [] | "start function has wrong type" |
| Imports | Imported types match declarations | "import type mismatch" |

### Public API

```python
@dataclass
class ValidationError:
    message: str
    function_index: int | None = None
    instruction_offset: int | None = None

@dataclass
class ValidationResult:
    is_valid: bool
    errors: list[ValidationError]

class WasmValidator:
    def validate(self, module: WasmModule) -> ValidationResult:
        """Validate a parsed WASM module.

        Returns ValidationResult with is_valid=True if the module passes
        all checks, or is_valid=False with a list of specific errors.
        """
```

---

## Package 6: `wasm-execution`

### Execution Model

The execution engine interprets validated WASM modules. It manages four runtime
data structures:

```
┌─────────────────────────────────────────────────────┐
│                  WasmExecutionEngine                 │
│                                                      │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────┐  │
│  │  Value    │  │  Label    │  │  Call Stack       │  │
│  │  Stack    │  │  Stack    │  │  (WasmFrames)     │  │
│  │          │  │          │  │                    │  │
│  │  [i32:42]│  │  [Label0]│  │  [Frame0, Frame1]│  │
│  │  [i64:7] │  │  [Label1]│  │                    │  │
│  │  [f32:…] │  │          │  │                    │  │
│  └──────────┘  └───────────┘  └──────────────────┘  │
│                                                      │
│  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │  Linear Memory   │  │  Tables                  │  │
│  │  (page-based)    │  │  (function references)   │  │
│  │                  │  │                          │  │
│  │  [0x00...0xFF]   │  │  [func_0, func_3, null] │  │
│  └──────────────────┘  └──────────────────────────┘  │
│                                                      │
│  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │  Globals          │  │  Host Interface          │  │
│  │  [i32:0, f64:3.14]│  │  (WASI / custom)         │  │
│  └──────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### WasmFrame (Call Stack Entry)

```python
@dataclass
class WasmFrame:
    """A single activation record on the call stack.

    Created when a function is called, destroyed when it returns.
    """
    function_index: int
    locals: list[Value]             # params + declared locals
    pc: int                         # byte offset into function body
    label_stack: list[Label]        # active block/loop/if labels
    stack_height_at_entry: int      # for restoring stack on return
```

### Label (Structured Control Flow)

```python
@dataclass
class Label:
    """Tracks one level of structured control flow nesting.

    When 'br N' executes, it unwinds to the Nth label from the top of
    the label stack. For blocks, the branch target is the END instruction
    (forward jump). For loops, the branch target is the LOOP start
    (backward jump — this is how loops repeat).
    """
    arity: int              # how many values this block produces
    target_pc: int          # where to jump on branch
    stack_height: int       # stack height when this block started
    is_loop: bool           # True for loop (br goes to start), False for block (br goes to end)
```

### Linear Memory

```python
class LinearMemory:
    """Byte-addressable linear memory with page-based growth.

    WASM memory is a contiguous, mutable array of raw bytes. It grows
    in units of pages (each page = 65536 bytes = 64 KiB).

    All loads and stores use little-endian byte order.
    """
    PAGE_SIZE = 65536   # 64 KiB

    def __init__(self, initial_pages: int, max_pages: int | None = None): ...
    def load(self, offset: int, width: int) -> bytes: ...
    def store(self, offset: int, data: bytes) -> None: ...
    def grow(self, delta_pages: int) -> int: ...    # returns old size or -1
    def size(self) -> int: ...                       # current size in pages
```

### Table

```python
class Table:
    """A table of function references for indirect calls.

    In WASM 1.0, tables hold funcref values — indices into the function
    index space, or null. Used by call_indirect to support dynamic dispatch.
    """
    def __init__(self, initial_size: int, max_size: int | None = None): ...
    def get(self, index: int) -> int | None: ...
    def set(self, index: int, func_index: int | None) -> None: ...
```

### Host Interface

```python
class HostInterface(Protocol):
    """Protocol for external host functions.

    The runtime calls this when WASM code invokes an imported function.
    Implementations provide the bridge to the outside world — file I/O,
    console output, system calls, browser APIs, etc.
    """
    def call_import(
        self, module_name: str, func_name: str, args: list
    ) -> list: ...

    def resolve_global(
        self, module_name: str, name: str
    ) -> tuple[ValueType, bool, int | float]: ...

    def resolve_memory(
        self, module_name: str, name: str
    ) -> LinearMemory: ...

    def resolve_table(
        self, module_name: str, name: str
    ) -> Table: ...


class WasiHost(HostInterface):
    """Minimal WASI (WebAssembly System Interface) host.

    Implements the wasi_snapshot_preview1 interface with these syscalls:
    - fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) — write to fd
    - fd_read(fd, iovs_ptr, iovs_len, nread_ptr) — read from fd
    - proc_exit(code) — terminate execution
    - args_sizes_get(argc_ptr, argv_buf_size_ptr) — query CLI args
    - args_get(argv_ptr, argv_buf_ptr) — get CLI args
    - environ_sizes_get(count_ptr, buf_size_ptr) — query env vars
    - environ_get(environ_ptr, environ_buf_ptr) — get env vars

    In browser environments (TypeScript), fd_write maps to console.log
    and fd_read is unsupported.
    """
    def __init__(
        self,
        stdout=sys.stdout,
        stderr=sys.stderr,
        stdin=sys.stdin,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
    ): ...
```

### Interpreter Loop

```python
class WasmExecutionEngine:
    """The core WASM interpreter.

    Executes instructions from a validated, instantiated module.
    Uses a dispatch table (opcode → handler function) for fast execution.
    """

    def __init__(self, instance: WasmInstance): ...

    def call_function(self, func_index: int, args: list) -> list:
        """Call a function by index with the given arguments.

        Creates a new WasmFrame, pushes args as locals, and runs
        the interpreter loop until the function returns.
        """

    def _step(self) -> bool:
        """Execute one instruction. Returns False when function returns."""
        opcode = self._read_byte()
        handler = self._dispatch_table[opcode]
        handler(self)
        return not self._returned
```

---

## Package 7: `wasm-runtime`

### Top-Level API

```python
class WasmRuntime:
    """Complete WebAssembly runtime — parse, validate, instantiate, execute.

    This is the user-facing entry point that composes all other wasm-* packages.

    Usage:
        runtime = WasmRuntime(host=WasiHost())
        module = runtime.load(wasm_bytes)
        result = runtime.validate(module)
        if result.is_valid:
            instance = runtime.instantiate(module)
            return_values = runtime.call(instance, "add", [1, 2])

    Or the convenience method:
        runtime.load_and_run(wasm_bytes)  # parse + validate + instantiate + call _start
    """

    def __init__(self, host: HostInterface | None = None): ...

    def load(self, wasm_bytes: bytes) -> WasmModule:
        """Parse .wasm binary into a WasmModule."""

    def validate(self, module: WasmModule) -> ValidationResult:
        """Validate a parsed module."""

    def instantiate(self, module: WasmModule) -> WasmInstance:
        """Create an executable instance from a validated module.

        1. Allocate linear memory (from memory section + data segments)
        2. Allocate tables (from table section + element segments)
        3. Initialize globals (from global section)
        4. Resolve imports (via host interface)
        5. Call start function (if present)
        """

    def call(self, instance: WasmInstance, name: str, args: list) -> list:
        """Call an exported function by name."""

    def load_and_run(
        self, wasm_bytes: bytes, entry: str = "_start", args: list | None = None
    ) -> list:
        """Convenience: parse, validate, instantiate, and call entry point."""


@dataclass
class WasmInstance:
    """A runtime instance of a WASM module — ready to execute.

    Contains all allocated runtime state: memory, tables, globals, and
    the resolved function index space (imports + module functions).
    """
    module: WasmModule
    memory: LinearMemory | None
    tables: list[Table]
    globals: list    # list of current global values
    functions: list  # combined import functions + module functions
    host: HostInterface | None
```

---

## Implementation Languages

Each package is implemented in all 6 languages:

| Language | Package naming | Module naming | Test framework |
|----------|---------------|---------------|---------------|
| Python | `wasm-leb128/` (kebab-case dir) | `wasm_leb128` (snake_case import) | pytest |
| Go | `wasm-leb128/` | `wasmleb128` (single word) | `go test` |
| TypeScript | `wasm-leb128/` | `@coding-adventures/wasm-leb128` | vitest |
| Ruby | `wasm_leb128/` (snake_case dir) | `coding_adventures_wasm_leb128` | minitest |
| Rust | `wasm-leb128/` | `wasm_leb128` | `cargo test` |
| Elixir | `wasm_leb128/` | `CodingAdventures.WasmLeb128` | ExUnit |

---

## Future: WASM-to-RISC-V Compiler Backend

The runtime's module representation and validator are shared between two execution paths:

1. **Direct interpretation** (this spec) — the `wasm-execution` engine runs instructions directly
2. **Compiled execution** (future `wasm-compiler-backend` package) — translates WASM functions to RISC-V machine code, which runs on the simulated CPU with full pipeline/cache/gate tracing

The compiler backend would:
- Map WASM stack operations to RISC-V register allocations
- Translate WASM linear memory ops to RISC-V load/store
- Convert WASM structured control flow to RISC-V branches
- Output binary compatible with the existing `riscv-simulator`

This creates the full trace: WASM bytecode → RISC-V instructions → pipeline stages → cache hits/misses → ALU → logic gates.

---

## Test Strategy

### Per-Package Unit Tests

Each package has comprehensive tests (>80% coverage target):

- `wasm-leb128`: Encode/decode round-trips, boundary values, error cases
- `wasm-types`: Construction, equality, serialization of all type structures
- `wasm-opcodes`: Lookup by name, lookup by byte, metadata correctness
- `wasm-module-parser`: Parse each section type, full modules, error cases
- `wasm-validator`: Valid modules pass, each validation rule catches its error
- `wasm-execution`: Each instruction category has its own test file
- `wasm-runtime`: Integration tests with real `.wasm` binaries

### Cross-Language Consistency

All 6 language implementations share the same test fixtures:
- Hand-crafted `.wasm` binary files in a shared `fixtures/` directory
- JSON test vectors for LEB128 encode/decode values
- JSON test vectors for instruction execution (input stack → expected output stack)

### End-to-End: Hello World

The ultimate integration test across all languages:

```
1. Build a .wasm module that imports wasi_snapshot_preview1.fd_write
2. The module's _start function writes "Hello World\n" to fd 1 (stdout)
3. Load the module into WasmRuntime with WasiHost
4. Call load_and_run()
5. Assert WasiHost captured "Hello World\n" on stdout
```
