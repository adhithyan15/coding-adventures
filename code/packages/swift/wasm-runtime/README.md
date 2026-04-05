# wasm-runtime

Complete WebAssembly 1.0 runtime for Swift.

## Overview

The runtime is the user-facing entry point that composes all lower-level
WASM packages into a single API. It handles the full pipeline:

```
.wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
```

## Usage

```swift
import WasmRuntime

// Simple: compute square(5) from a .wasm binary
let runtime = WasmRuntime()
let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
// result == [25]

// With WASI for programs that do I/O:
let wasi = WasiStub()
let runtime = WasmRuntime(host: wasi)
try runtime.loadAndRun(helloWorldWasm)
print(wasi.stdoutOutput)
```

## Components

- **WasmRuntime**: Main entry point with load/validate/instantiate/call methods
- **WasmInstance**: Live module instance with allocated memory, tables, globals
- **WasiStub**: Minimal WASI implementation for fd_write and proc_exit

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- wasm-validator
- wasm-execution
- virtual-machine

## Development

```bash
# Run tests
bash BUILD
```
