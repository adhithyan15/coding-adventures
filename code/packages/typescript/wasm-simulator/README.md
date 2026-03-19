# @coding-adventures/wasm-simulator

WebAssembly (WASM) stack-based virtual machine simulator -- Layer 4c of the computing stack.

## What is this?

A TypeScript implementation of a WebAssembly bytecode simulator. WASM is a binary instruction format designed as a portable compilation target for the web. This simulator executes a subset of real WASM instructions, demonstrating stack-based execution with variable-width instruction encoding.

## Supported Instructions

| Instruction | Encoding | Description |
|------------|----------|-------------|
| `i32.const V` | 5 bytes | Push a 32-bit integer constant |
| `i32.add` | 1 byte | Pop two i32s, push their sum |
| `i32.sub` | 1 byte | Pop two i32s, push their difference |
| `local.get N` | 2 bytes | Push local variable N onto the stack |
| `local.set N` | 2 bytes | Pop the stack into local variable N |
| `end` | 1 byte | Halt execution |

## Usage

```typescript
import {
  WasmSimulator,
  assembleWasm,
  encodeI32Const,
  encodeI32Add,
  encodeLocalSet,
  encodeEnd,
} from "@coding-adventures/wasm-simulator";

const sim = new WasmSimulator(4);
const program = assembleWasm([
  encodeI32Const(1),  // push 1
  encodeI32Const(2),  // push 2
  encodeI32Add(),     // pop 2 and 1, push 3
  encodeLocalSet(0),  // pop 3, store in local 0
  encodeEnd(),        // halt
]);
const traces = sim.run(program);
console.log(sim.locals[0]); // => 3
```

## How it fits in the stack

This is a TypeScript port of the Python wasm-simulator package. It sits at Layer 4c, demonstrating how modern stack-based virtual machines work with variable-width bytecode encoding.
