# wasm-types (Python)

Pure type definitions for the WASM 1.0 type system. No parsing logic — just
data structures representing every type-level concept in WebAssembly. Used by
`wasm-opcodes` and `wasm-module-parser`.

## What This Package Does

WebAssembly modules are structured as a sequence of typed sections. This
package provides Python data structures that mirror those section types
exactly. Byte values of enum constants match the WASM binary encoding, so
a parser can use them directly.

```
┌────────────┬──────────────────────┬────────────────────────────────────┐
│ Section ID │ Section Name         │ Python type (this package)         │
├────────────┼──────────────────────┼────────────────────────────────────┤
│     1      │ Type section         │ list[FuncType]                     │
│     2      │ Import section       │ list[Import]                       │
│     3      │ Function section     │ list[int]  (type indices)          │
│     4      │ Table section        │ list[TableType]                    │
│     5      │ Memory section       │ list[MemoryType]                   │
│     6      │ Global section       │ list[Global]                       │
│     7      │ Export section       │ list[Export]                       │
│     8      │ Start section        │ int | None                         │
│     9      │ Element section      │ list[Element]                      │
│    10      │ Code section         │ list[FunctionBody]                 │
│    11      │ Data section         │ list[DataSegment]                  │
│     0      │ Custom sections      │ list[CustomSection]                │
└────────────┴──────────────────────┴────────────────────────────────────┘
```

## Types

### Enums

| Type           | Members                            | Byte values        |
|----------------|------------------------------------|--------------------|
| `ValueType`    | I32, I64, F32, F64                 | 0x7F, 0x7E, 0x7D, 0x7C |
| `ExternalKind` | FUNCTION, TABLE, MEMORY, GLOBAL    | 0x00, 0x01, 0x02, 0x03 |
| `BlockType`    | EMPTY                              | 0x40               |

### Frozen Dataclasses (immutable)

| Type           | Fields                                                    |
|----------------|-----------------------------------------------------------|
| `FuncType`     | `params: tuple[ValueType, ...]`, `results: tuple[ValueType, ...]` |
| `Limits`       | `min: int`, `max: int \| None`                            |
| `MemoryType`   | `limits: Limits`                                          |
| `TableType`    | `element_type: int = 0x70`, `limits: Limits`              |
| `GlobalType`   | `value_type: ValueType`, `mutable: bool`                  |
| `Import`       | `module_name: str`, `name: str`, `kind: ExternalKind`, `type_info` |
| `Export`       | `name: str`, `kind: ExternalKind`, `index: int`           |
| `Global`       | `global_type: GlobalType`, `init_expr: bytes`             |
| `Element`      | `table_index: int`, `offset_expr: bytes`, `function_indices: tuple[int, ...]` |
| `DataSegment`  | `memory_index: int`, `offset_expr: bytes`, `data: bytes`  |
| `FunctionBody` | `locals: tuple[ValueType, ...]`, `code: bytes`            |
| `CustomSection`| `name: str`, `data: bytes`                                |

### Mutable Container

| Type         | Fields                                        |
|--------------|-----------------------------------------------|
| `WasmModule` | `types`, `imports`, `functions`, `tables`, `memories`, `globals`, `exports`, `start`, `elements`, `code`, `data`, `customs` |

## Usage

```python
from wasm_types import (
    ValueType, ExternalKind, BlockType,
    FuncType, Limits, MemoryType, TableType, GlobalType,
    Import, Export, Global, Element, DataSegment,
    FunctionBody, CustomSection, WasmModule,
)

# Describe a function that takes (i32) and returns (i64)
sig = FuncType(params=(ValueType.I32,), results=(ValueType.I64,))

# Describe a linear memory with 1 initial page and no max
mem = MemoryType(limits=Limits(min=1))

# Build a module incrementally
module = WasmModule()
module.types.append(sig)
module.memories.append(mem)
module.exports.append(Export(name="memory", kind=ExternalKind.MEMORY, index=0))

# Frozen types are usable as dict keys
type_cache: dict[FuncType, int] = {}
type_cache[sig] = 0
```

## Dependencies

- `wasm-leb128` — LEB128 encoding (used by downstream parsers)

## Development

```bash
# Unix/macOS
bash BUILD

# Windows
bash BUILD_windows
```

## How It Fits in the Stack

```
wasm-leb128          — LEB128 integer encoding/decoding
wasm-types           — THIS PACKAGE: type system data structures
wasm-opcodes         — instruction set (depends on wasm-types)
wasm-module-parser   — binary parser (depends on wasm-types, wasm-opcodes)
wasm-validator       — type checker (depends on all above)
wasm-interpreter     — execution engine (depends on all above)
```
