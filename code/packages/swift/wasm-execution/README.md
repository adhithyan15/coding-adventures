# wasm-execution

WebAssembly 1.0 execution engine for Swift.

## Overview

This package implements a complete WASM 1.0 interpreter built on top of the
GenericVM (virtual-machine package). It provides:

- **WasmValue**: Typed WASM values (i32, i64, f32, f64)
- **LinearMemory**: Byte-addressable heap with page-based growth
- **Table**: Function reference table for indirect calls
- **WasmExecutionEngine**: The main interpreter with recursive call dispatch
- **All 172 WASM 1.0 instructions**: numeric, variable, memory, control flow, conversions

## Architecture

Function calls are handled recursively. When a `call` instruction executes,
the engine decodes the callee, builds a fresh VM, and calls itself. This
avoids the complexity of inline code switching and mirrors how real
interpreters work.

## Usage

```swift
import WasmExecution
import WasmTypes

// Create a simple function body: i32.const 5, i32.const 5, i32.mul, end
let body = FunctionBody(locals: [], code: [0x41, 0x05, 0x41, 0x05, 0x6C, 0x0B])
let funcType = FuncType(params: [], results: [.i32])

let engine = WasmExecutionEngine(
    memory: nil, tables: [], globals: [],
    globalTypes: [], funcTypes: [funcType],
    funcBodies: [body], hostFunctions: [nil]
)

let result = try engine.callFunction(0, [])
// result == [.i32(25)]
```

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- virtual-machine

## Development

```bash
# Run tests
bash BUILD
```
