# wasm-module-parser

Parse a raw `.wasm` binary into a structured `WasmModule`. Takes bytes,
produces structured data. No execution.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
ground-up computing stack.

## What It Does

This package reads the binary WebAssembly (`.wasm`) format and decodes every
section into typed, inspectable data structures. It covers all 12 section types
in the WASM 1.0 specification:

| Section ID | Name     | What it contains                            |
|------------|----------|---------------------------------------------|
| 0          | Custom   | Named byte blobs (debug info, metadata)     |
| 1          | Type     | Function signatures (params + results)      |
| 2          | Import   | Host-provided functions/tables/memories/globals |
| 3          | Function | Type-section indices for local functions    |
| 4          | Table    | Indirect-call tables (funcref arrays)       |
| 5          | Memory   | Linear memory declarations                  |
| 6          | Global   | Module-level global variables               |
| 7          | Export   | Host-visible definitions                    |
| 8          | Start    | Entry-point function index                  |
| 9          | Element  | Table initializer segments                  |
| 10         | Code     | Function bodies with locals and bytecode    |
| 11         | Data     | Memory initializer segments                 |

## Installation

```bash
npm install @coding-adventures/wasm-module-parser
```

## Usage

```typescript
import { WasmModuleParser, WasmParseError } from "@coding-adventures/wasm-module-parser";
import { readFileSync } from "fs";

const parser = new WasmModuleParser();

try {
  const bytes = new Uint8Array(readFileSync("example.wasm"));
  const module = parser.parse(bytes);

  // Inspect types
  for (const t of module.types) {
    console.log(`FuncType: (${t.params}) → (${t.results})`);
  }

  // Inspect exports
  for (const e of module.exports) {
    console.log(`Export: ${e.name} (kind=${e.kind}, index=${e.index})`);
  }

} catch (e) {
  if (e instanceof WasmParseError) {
    console.error(`Parse failed at byte offset 0x${e.offset.toString(16)}: ${e.message}`);
  }
}
```

## API

### `WasmModuleParser`

```typescript
class WasmModuleParser {
  parse(data: Uint8Array): WasmModule  // throws WasmParseError on malformed input
}
```

The parser is stateless between calls. Reuse the same instance for multiple files.

### `WasmParseError`

```typescript
class WasmParseError extends Error {
  readonly offset: number;  // byte offset where the problem was detected
}
```

### `WasmModule` (from `@coding-adventures/wasm-types`)

```typescript
class WasmModule {
  types:     FuncType[]       // type section
  imports:   Import[]         // import section
  functions: number[]         // function section (type indices)
  tables:    TableType[]      // table section
  memories:  MemoryType[]     // memory section
  globals:   Global[]         // global section
  exports:   Export[]         // export section
  start:     number | null    // start section
  elements:  Element[]        // element section
  code:      FunctionBody[]   // code section
  data:      DataSegment[]    // data section
  customs:   CustomSection[]  // custom sections
}
```

## WASM Binary Format Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Magic:   0x00 0x61 0x73 0x6D  ("\0asm")  — bytes 0–3       │
│  Version: 0x01 0x00 0x00 0x00  (1)        — bytes 4–7       │
├─────────────────────────────────────────────────────────────┤
│  Section: id:u8 + size:u32leb + payload:bytes               │
│  Section: id:u8 + size:u32leb + payload:bytes               │
│  ...                                                        │
└─────────────────────────────────────────────────────────────┘
```

All integers in the payload are ULEB128-encoded (variable-length).

## Dependencies

- [`@coding-adventures/wasm-leb128`](../wasm-leb128) — ULEB128/SLEB128 decode/encode
- [`@coding-adventures/wasm-types`](../wasm-types) — `WasmModule` and all type definitions
- [`@coding-adventures/wasm-opcodes`](../wasm-opcodes) — opcode table (available for consumers)

## Development

```bash
npm install
npx vitest run --coverage
```

## Layer in the Stack

```
wasm-module-parser  ← you are here (parses binary → WasmModule)
wasm-types          ← WasmModule, FuncType, Import, Export, ... (data model)
wasm-leb128         ← LEB128 variable-length integer encoding
wasm-opcodes        ← opcode table for all 183 WASM 1.0 instructions
```
