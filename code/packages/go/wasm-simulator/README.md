# WASM Simulator (Go Port)

**Layer 4-c of the computing stack** — implements the WebAssembly foundational stack-based simulator structure.

## Overview
WebAssembly (WASM) is a modern binary instruction format heavily used in modern browser runtimes (compiling Rust, C++, Go to the web). Instead of writing to statically mapped physical registers (like RISC-V and ARM), WASM computes mathematically using an **operand stack**. Operands are pushed onto a queue, and operations pop them implicitly without defining fixed computational targets.

This module fundamentally diverges from the standard `cpu-simulator` generic architecture to demonstrate the nature of variable-length bytecodes explicitly evaluating data on a dynamically sized Stack.

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/wasm-simulator"
)

// 1. Initialize Simulator with 4 available local variables
sim := wasmsimulator.NewWasmSimulator(4)

// 2. Assemble test instructions
program := wasmsimulator.AssembleWasm([][]byte{
    wasmsimulator.EncodeI32Const(1), // push 1
    wasmsimulator.EncodeI32Const(2), // push 2
    wasmsimulator.EncodeI32Add(),    // pop 2 and 1, evaluate 3 -> push 3
    wasmsimulator.EncodeEnd(),       // halt
})

// 3. Execution Pipeline
traces := sim.Run(program, 1000)
```
