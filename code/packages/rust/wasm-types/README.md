# wasm-types

Pure type definitions for the WebAssembly 1.0 (MVP) type system.

This crate contains **no parsing logic**. It defines the Rust data structures
that represent a decoded WASM module's type information. Higher-level crates
(`wasm-opcodes`, `wasm-module-parser`) depend on these definitions.

## What it provides

### Value types

The four numeric types that every WASM value, local, and stack slot holds:

| Type   | Byte (binary) | Description                          |
|--------|---------------|--------------------------------------|
| `I32`  | `0x7F`        | 32-bit integer (also used for bools) |
| `I64`  | `0x7E`        | 64-bit integer                       |
| `F32`  | `0x7D`        | 32-bit IEEE 754 float                |
| `F64`  | `0x7C`        | 64-bit IEEE 754 float                |

### Module-level types

| Type              | Description                                              |
|-------------------|----------------------------------------------------------|
| `BlockType`       | Result type of `block`/`loop`/`if` instructions         |
| `ExternalKind`    | Function / Table / Memory / Global (for imports/exports) |
| `FuncType`        | Function signature: `params -> results`                 |
| `Limits`          | min/max size for memories and tables                    |
| `MemoryType`      | Linear memory declaration                               |
| `TableType`       | Function reference table declaration                    |
| `GlobalType`      | Global variable type + mutability                       |
| `Import`          | An import from the host environment                     |
| `ImportTypeInfo`  | Type-specific info for each import kind                 |
| `Export`          | An export to the host environment                       |
| `Global`          | Module-defined global with init expression              |
| `Element`         | Table initialization segment                            |
| `DataSegment`     | Memory initialization segment                           |
| `FunctionBody`    | Local variables + raw bytecode for one function         |
| `CustomSection`   | Named tool metadata section                             |
| `WasmModule`      | Top-level container for all decoded sections            |

## How it fits in the stack

```
wasm-leb128          ← LEB128 integer decoding
wasm-types           ← THIS CRATE: type definitions (no parsing)
wasm-opcodes         ← instruction set definitions (depends on wasm-types)
wasm-module-parser   ← binary → WasmModule (depends on wasm-types + wasm-leb128)
wasm-simulator       ← execution engine (depends on all above)
```

## Usage

```rust
use wasm_types::{ValueType, FuncType, Limits, MemoryType, WasmModule, FUNCREF};

// Describe a function: (i32, i64) -> f32
let sig = FuncType {
    params: vec![ValueType::I32, ValueType::I64],
    results: vec![ValueType::F32],
};

// Describe a memory: at least 1 page, at most 4 pages (4 * 64KiB = 256KiB)
let mem = MemoryType {
    limits: Limits { min: 1, max: Some(4) },
};

// Start with an empty module and add to it
let mut module = WasmModule::default();
module.types.push(sig);
module.memories.push(mem);
```

## WASM binary encoding quick reference

```
ValueType bytes:   I32=0x7F  I64=0x7E  F32=0x7D  F64=0x7C
ExternalKind:      Func=0x00  Table=0x01  Mem=0x02  Global=0x03
BlockType empty:   0x40
FuncType tag:      0x60
FuncRef tag:       0x70
Limits flags:      0x00 (min only)  0x01 (min + max)
```

## Dependencies

- `wasm-leb128` — LEB128 encoding/decoding (listed as a dependency for downstream use)

## Development

```bash
# Run tests
cd code/packages/rust/wasm-types
cargo test -p wasm-types -- --nocapture

# Lint
cargo clippy -p wasm-types

# Or use the BUILD script
bash BUILD
```
