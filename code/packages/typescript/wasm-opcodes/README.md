# @coding-adventures/wasm-opcodes

Complete WASM 1.0 opcode lookup table with metadata for all 172 instructions.

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up implementation of the computing stack from transistors to
operating systems.

## What it does

WebAssembly (WASM) is a binary instruction format. Every instruction is encoded
as a single opcode byte, followed by zero or more "immediate" operands. This
package provides a complete lookup table with:

- The canonical text-format name (e.g. `i32.add`)
- The single-byte opcode value (e.g. `0x6A`)
- The instruction category (e.g. `numeric_i32`)
- The list of immediate operand types (e.g. `["memarg"]` for memory instructions)
- Stack effect metadata — how many values are consumed (`stackPop`) and
  produced (`stackPush`)

## Where it fits in the stack

```
wasm-types        — WASM value types and type system primitives
    └── wasm-opcodes  — complete opcode table (this package)
            └── wasm-leb128, wasm-module-parser, wasm-simulator, ...
```

## Installation

```bash
npm install @coding-adventures/wasm-opcodes
```

## Usage

```ts
import {
  getOpcode,
  getOpcodeByName,
  OPCODES,
  OPCODES_BY_NAME,
} from "@coding-adventures/wasm-opcodes";

// Look up by byte value
const info = getOpcode(0x6A);
// => { name: "i32.add", opcode: 106, category: "numeric_i32", immediates: [], stackPop: 2, stackPush: 1 }

// Look up by name
const info2 = getOpcodeByName("i32.add");
// => same OpcodeInfo object

// Unknown byte → undefined
getOpcode(0xFF);        // undefined
getOpcodeByName("foo"); // undefined

// Iterate all opcodes
for (const [byte, info] of OPCODES) {
  console.log(`0x${byte.toString(16).padStart(2, "0")}  ${info.name}`);
}

// Check immediates
getOpcodeByName("i32.load")?.immediates;       // ["memarg"]
getOpcodeByName("block")?.immediates;          // ["blocktype"]
getOpcodeByName("call_indirect")?.immediates;  // ["typeidx", "tableidx"]
getOpcodeByName("i32.add")?.immediates;        // []
```

## API

### `OpcodeInfo`

```ts
interface OpcodeInfo {
  readonly name: string;               // e.g. "i32.add"
  readonly opcode: number;             // e.g. 0x6A
  readonly category: string;           // e.g. "numeric_i32"
  readonly immediates: readonly string[];  // e.g. [], ["memarg"], ["blocktype"]
  readonly stackPop: number;           // values consumed from stack
  readonly stackPush: number;          // values produced onto stack
}
```

### `OPCODES: Map<number, OpcodeInfo>`

Primary lookup table keyed by opcode byte.

### `OPCODES_BY_NAME: Map<string, OpcodeInfo>`

Secondary lookup table keyed by instruction name.

### `getOpcode(byte: number): OpcodeInfo | undefined`

Look up an instruction by its opcode byte. Returns `undefined` for reserved or
unknown bytes.

### `getOpcodeByName(name: string): OpcodeInfo | undefined`

Look up an instruction by its canonical text-format name. Returns `undefined`
if not found.

## Categories

| Category      | Description                                              | Count |
|---------------|----------------------------------------------------------|-------|
| `control`     | Structured control flow (block, loop, if, br, call, ...) | 13    |
| `parametric`  | Stack manipulation (drop, select)                        | 2     |
| `variable`    | Local/global variable access                             | 5     |
| `memory`      | Loads, stores, memory.size, memory.grow                  | 25    |
| `numeric_i32` | 32-bit integer arithmetic, comparisons, bitwise          | 30    |
| `numeric_i64` | 64-bit integer arithmetic, comparisons, bitwise          | 30    |
| `numeric_f32` | 32-bit float arithmetic and comparisons                  | 21    |
| `numeric_f64` | 64-bit float arithmetic and comparisons                  | 21    |
| `conversion`  | Type conversions between numeric types                   | 25    |
| **Total**     |                                                          | **172** |

## Immediate operand types

| Type            | Meaning                                                   |
|-----------------|-----------------------------------------------------------|
| `"i32"`         | 32-bit integer, LEB128-encoded                           |
| `"i64"`         | 64-bit integer, LEB128-encoded                           |
| `"f32"`         | 32-bit float, 4 bytes little-endian                      |
| `"f64"`         | 64-bit float, 8 bytes little-endian                      |
| `"blocktype"`   | Result type of a block (-0x40 for void, or a valtype)    |
| `"labelidx"`    | Branch target depth, LEB128-encoded                      |
| `"vec_labelidx"`| br_table: count + N label indices                        |
| `"funcidx"`     | Function table index, LEB128-encoded                     |
| `"typeidx"`     | Type section index                                        |
| `"tableidx"`    | Table section index (always 0 in WASM 1.0)               |
| `"localidx"`    | Local variable index                                      |
| `"globalidx"`   | Global variable index                                     |
| `"memarg"`      | `{ align: u32, offset: u32 }` — both LEB128-encoded      |
| `"memidx"`      | Memory index (always 0 in WASM 1.0)                      |

## Dependencies

- `@coding-adventures/wasm-types` — WASM value types and type system primitives

## Development

```bash
npm install
npx vitest run --coverage
```

## License

MIT
