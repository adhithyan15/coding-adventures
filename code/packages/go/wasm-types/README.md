# wasm-types (Go)

Pure type definitions for the WASM 1.0 type system. No parsing logic — just
data structures representing every type-level concept in WebAssembly. Used by
`wasm-opcodes` and `wasm-module-parser`.

## What This Package Does

WebAssembly modules are structured as a sequence of typed sections. This
package provides Go structs and constants that mirror those section types
exactly. Byte values of constants match the WASM binary encoding, so a
parser can use them directly without translation.

```
┌────────────┬──────────────────────┬────────────────────────────────────┐
│ Section ID │ Section Name         │ Go type (this package)             │
├────────────┼──────────────────────┼────────────────────────────────────┤
│     1      │ Type section         │ []FuncType                         │
│     2      │ Import section       │ []Import                           │
│     3      │ Function section     │ []uint32  (type indices)           │
│     4      │ Table section        │ []TableType                        │
│     5      │ Memory section       │ []MemoryType                       │
│     6      │ Global section       │ []Global                           │
│     7      │ Export section       │ []Export                           │
│     8      │ Start section        │ *uint32   (nil = absent)           │
│     9      │ Element section      │ []Element                          │
│    10      │ Code section         │ []FunctionBody                     │
│    11      │ Data section         │ []DataSegment                      │
│     0      │ Custom sections      │ []CustomSection                    │
└────────────┴──────────────────────┴────────────────────────────────────┘
```

## Types

### Constants (byte-valued, matching WASM binary encoding)

**ValueType** (`type ValueType byte`):

| Constant       | Value | Meaning             |
|----------------|-------|---------------------|
| `ValueTypeI32` | 0x7F  | 32-bit integer      |
| `ValueTypeI64` | 0x7E  | 64-bit integer      |
| `ValueTypeF32` | 0x7D  | 32-bit float        |
| `ValueTypeF64` | 0x7C  | 64-bit float        |

**ExternalKind** (`type ExternalKind byte`):

| Constant               | Value | Meaning         |
|------------------------|-------|-----------------|
| `ExternalKindFunction` | 0x00  | callable function |
| `ExternalKindTable`    | 0x01  | reference table   |
| `ExternalKindMemory`   | 0x02  | linear memory     |
| `ExternalKindGlobal`   | 0x03  | global variable   |

**BlockType** (`type BlockType byte`):

| Constant         | Value | Meaning               |
|------------------|-------|-----------------------|
| `BlockTypeEmpty` | 0x40  | block has no results  |

**Other**:

| Constant             | Value | Meaning                |
|----------------------|-------|------------------------|
| `ElementTypeFuncRef` | 0x70  | funcref (table element)|

### Structs

| Struct         | Key Fields                                              |
|----------------|---------------------------------------------------------|
| `FuncType`     | `Params []ValueType`, `Results []ValueType`             |
| `Limits`       | `Min uint32`, `Max uint32`, `HasMax bool`               |
| `MemoryType`   | `Limits Limits`                                         |
| `TableType`    | `ElementType byte`, `Limits Limits`                     |
| `GlobalType`   | `ValueType ValueType`, `Mutable bool`                   |
| `Import`       | `ModuleName string`, `Name string`, `Kind ExternalKind`, `TypeInfo any` |
| `Export`       | `Name string`, `Kind ExternalKind`, `Index uint32`      |
| `Global`       | `GlobalType GlobalType`, `InitExpr []byte`              |
| `Element`      | `TableIndex uint32`, `OffsetExpr []byte`, `FunctionIndices []uint32` |
| `DataSegment`  | `MemoryIndex uint32`, `OffsetExpr []byte`, `Data []byte` |
| `FunctionBody` | `Locals []ValueType`, `Code []byte`                     |
| `CustomSection`| `Name string`, `Data []byte`                            |
| `WasmModule`   | all 12 section fields as slices + `Start *uint32`       |

## Usage

```go
package main

import wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"

func main() {
    // Describe a function that takes (i32) and returns (i64)
    sig := wasmtypes.FuncType{
        Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
        Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
    }

    // Build a module incrementally
    m := wasmtypes.WasmModule{}
    m.Types = append(m.Types, sig)
    m.Memories = append(m.Memories, wasmtypes.MemoryType{
        Limits: wasmtypes.Limits{Min: 1},
    })
    m.Exports = append(m.Exports, wasmtypes.Export{
        Name:  "memory",
        Kind:  wasmtypes.ExternalKindMemory,
        Index: 0,
    })

    // Import type assertion for Import.TypeInfo
    imp := wasmtypes.Import{
        ModuleName: "env",
        Name:       "memory",
        Kind:       wasmtypes.ExternalKindMemory,
        TypeInfo:   wasmtypes.MemoryType{Limits: wasmtypes.Limits{Min: 1}},
    }
    if mt, ok := imp.TypeInfo.(wasmtypes.MemoryType); ok {
        _ = mt.Limits.Min
    }
}
```

## Dependencies

- `wasm-leb128` — LEB128 encoding (used by downstream parsers)

## Development

```bash
go test ./... -v -cover
go vet ./...
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
