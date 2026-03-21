# wasm-simulator

WebAssembly bytecode simulator -- a stack-based virtual machine.

## What is this?

This crate simulates a subset of the WebAssembly instruction set. Unlike RISC-V and ARM (register machines), WASM is a stack machine: instructions implicitly pop their operands from the stack and push results back.

## Supported Instructions

| Opcode | Mnemonic    | Description                    |
|--------|-------------|--------------------------------|
| 0x0B   | `end`       | Halt execution                 |
| 0x20   | `local.get` | Push local variable onto stack |
| 0x21   | `local.set` | Pop stack into local variable  |
| 0x41   | `i32.const` | Push 32-bit constant           |
| 0x6A   | `i32.add`   | Pop two, push sum              |
| 0x6B   | `i32.sub`   | Pop two, push difference       |

## Usage

```rust
use wasm_simulator::*;

let mut sim = WasmSimulator::new(4);
let program = assemble_wasm(&[
    encode_i32_const(10),
    encode_i32_const(3),
    encode_i32_sub(),
    encode_end(),
]);
let traces = sim.run(&program, 100);
assert_eq!(sim.stack[0], 7);
```
