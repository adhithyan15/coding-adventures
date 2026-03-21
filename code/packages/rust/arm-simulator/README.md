# arm-simulator

ARMv7 subset simulator -- the architecture that powers your phone.

## What is this?

This crate simulates a subset of the ARM data processing instruction set. ARM was designed in 1985 with a focus on power efficiency and now powers billions of mobile devices.

## Supported Instructions

| Instruction      | Description                       |
|------------------|-----------------------------------|
| `MOV Rd, #imm`   | Move immediate to register       |
| `ADD Rd, Rn, Rm` | Add two registers                |
| `SUB Rd, Rn, Rm` | Subtract two registers           |
| `HLT`            | Halt (custom sentinel)           |

## How it fits in the stack

Builds on `cpu-simulator` by providing ARM-specific decoding (including the rotate-immediate trick) and execution.

## Usage

```rust
use arm_simulator::*;

let mut sim = ARMSimulator::new(65536);
let program = assemble(&[
    encode_mov_imm(0, 10),  // R0 = 10
    encode_mov_imm(1, 3),   // R1 = 3
    encode_sub(2, 0, 1),    // R2 = R0 - R1 = 7
    encode_hlt(),
]);
let traces = sim.run(&program);
assert_eq!(sim.cpu.registers.read(2), 7);
```
