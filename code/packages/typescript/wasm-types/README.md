# wasm-types (TypeScript)

Pure type definitions for the WebAssembly 1.0 type system. No parsing logic —
just data structures representing every type-level concept in WASM.

Used by `wasm-opcodes` and `wasm-module-parser` as the shared vocabulary for
everything that has a type in the WASM binary format.

## What it provides

### Enums / const objects

| Name           | Values                                    | Bytes                        |
|----------------|-------------------------------------------|------------------------------|
| `ValueType`    | `I32`, `I64`, `F32`, `F64`               | `0x7F`, `0x7E`, `0x7D`, `0x7C` |
| `BlockType`    | `EMPTY`                                   | `0x40`                       |
| `ExternalKind` | `FUNCTION`, `TABLE`, `MEMORY`, `GLOBAL`  | `0x00`–`0x03`                |
| `FUNCREF`      | (constant)                               | `0x70`                       |

### Immutable data structures (interfaces)

| Type            | Purpose                                                        |
|-----------------|----------------------------------------------------------------|
| `FuncType`      | Function signature: `params` and `results` arrays             |
| `Limits`        | Min/max size bounds for memories and tables                    |
| `MemoryType`    | Linear memory type (wraps `Limits`)                            |
| `TableType`     | Table type — `elementType` (always `FUNCREF`) + `Limits`      |
| `GlobalType`    | Global variable type — `valueType` + `mutable` flag           |
| `Import`        | Import declaration — two-part name, kind, and type info        |
| `Export`        | Export declaration — name, kind, and index                     |
| `Global`        | Module-defined global — `GlobalType` + raw `initExpr` bytes   |
| `Element`       | Table initializer segment                                      |
| `DataSegment`   | Linear-memory initializer segment                              |
| `FunctionBody`  | Function body — expanded `locals` + raw `code` bytes          |
| `CustomSection` | Named arbitrary byte blob (section id 0)                      |

### Mutable module container

| Type         | Purpose                                                          |
|--------------|------------------------------------------------------------------|
| `WasmModule` | Holds all twelve section arrays plus `start: number \| null`    |

### Factory

| Function       | Purpose                                            |
|----------------|----------------------------------------------------|
| `makeFuncType` | Construct a frozen `FuncType` from param/result lists |

## How it fits in the stack

```
wasm-leb128       ← integer encoding/decoding
    ↓
wasm-types        ← THIS PACKAGE: all type definitions
    ↓
wasm-opcodes      ← instruction set definitions (uses ValueType)
    ↓
wasm-module-parser← parses .wasm binary into a WasmModule
    ↓
wasm-simulator    ← executes WasmModule
```

## Usage

```typescript
import {
  ValueType,
  ExternalKind,
  makeFuncType,
  WasmModule,
} from "@coding-adventures/wasm-types";

// Create a function type: (i32, i32) → i32
const addType = makeFuncType(
  [ValueType.I32, ValueType.I32],
  [ValueType.I32]
);

// Build a module skeleton
const mod = new WasmModule();
mod.types.push(addType);
mod.functions.push(0);           // function 0 has type at index 0
mod.exports.push({
  name: "add",
  kind: ExternalKind.FUNCTION,
  index: 0,
});
```

## Dependencies

- `@coding-adventures/wasm-leb128` — LEB128 encoding (listed as a dependency
  for downstream packages that need both; wasm-types itself does not call it)

## Development

```bash
npm install
npx vitest run --coverage
```

All tests must pass with >80% coverage (currently 100%).
