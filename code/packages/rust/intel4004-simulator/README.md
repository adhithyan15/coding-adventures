# intel4004-simulator

Intel 4004 simulator -- the world's first commercial microprocessor (1971).

## What is this?

This crate simulates the Intel 4004, a 4-bit accumulator-based processor with 16 registers. All values are 4 bits wide (0-15).

## Supported Instructions

| Opcode | Mnemonic | Description                           |
|--------|----------|---------------------------------------|
| 0xD    | LDM N   | Load immediate N into Accumulator     |
| 0xB    | XCH Rn  | Exchange Accumulator with register Rn |
| 0x8    | ADD Rn  | Add register Rn to Accumulator        |
| 0x9    | SUB Rn  | Subtract register Rn from Accumulator |
| 0x01   | HLT     | Halt                                  |

## Usage

```rust
use intel4004_simulator::*;

let mut sim = Intel4004Simulator::new(4096);
let program = vec![
    encode_ldm(5),   // A = 5
    encode_xch(0),   // R0 = 5, A = 0
    encode_ldm(3),   // A = 3
    encode_add(0),   // A = 3 + 5 = 8
    encode_hlt(),
];
sim.run(&program, 100);
assert_eq!(sim.accumulator, 8);
```
