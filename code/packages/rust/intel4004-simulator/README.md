# intel4004-simulator

Intel 4004 simulator -- the world's first commercial microprocessor (1971).

## What is this?

This crate simulates the complete Intel 4004, a 4-bit accumulator-based processor. All 46 real instructions are implemented, plus a simulator-only HLT instruction. The simulator is standalone and does not depend on any generic VM framework.

## Architecture

- **Data width:** 4 bits (values 0-15)
- **Registers:** 16 x 4-bit (R0-R15), organized as 8 pairs
- **Accumulator:** 4-bit -- most arithmetic goes through here
- **Carry flag:** 1 bit -- set on overflow/borrow
- **Program counter:** 12 bits (addresses 4096 bytes of ROM)
- **Stack:** 3-level hardware stack (12-bit return addresses, wraps mod 3)
- **RAM:** 4 banks x 4 registers x (16 main + 4 status) nibbles
- **ROM:** Up to 4096 x 8-bit (program storage)

## Supported Instructions (46 + HLT)

| Byte(s)   | Mnemonic     | Description                              |
|-----------|--------------|------------------------------------------|
| 0x00      | NOP          | No operation                             |
| 0x01      | HLT          | Halt (simulator-only)                    |
| 0x1C AA   | JCN c,addr   | Conditional jump                         |
| 0x2P DD   | FIM Pp,data  | Fetch immediate to register pair (even)  |
| 0x2P+1    | SRC Pp       | Send register control (odd)              |
| 0x3P      | FIN Pp       | Fetch indirect from ROM (even)           |
| 0x3P+1    | JIN Pp       | Jump indirect (odd)                      |
| 0x4H LL   | JUN addr     | Unconditional jump (12-bit)              |
| 0x5H LL   | JMS addr     | Jump to subroutine                       |
| 0x6R      | INC Rn       | Increment register                       |
| 0x7R AA   | ISZ Rn,addr  | Increment and skip if zero               |
| 0x8R      | ADD Rn       | Add register to accumulator with carry    |
| 0x9R      | SUB Rn       | Subtract register (complement-add)       |
| 0xAR      | LD Rn        | Load register into accumulator           |
| 0xBR      | XCH Rn       | Exchange accumulator and register         |
| 0xCN      | BBL n        | Branch back and load                     |
| 0xDN      | LDM n        | Load immediate into accumulator          |
| 0xE0-E7   | WRM..WR3     | RAM/ROM write operations                 |
| 0xE8-EF   | SBM..RD3     | RAM/ROM read and subtract/add operations |
| 0xF0-FD   | CLB..DCL     | Accumulator/carry manipulation           |

## Usage

```rust
use intel4004_simulator::*;

// Compute 3 + 4 = 7
let mut sim = Intel4004Simulator::new(4096);
let program = vec![
    encode_ldm(3),   // A = 3
    encode_xch(0),   // R0 = 3, A = 0
    encode_ldm(4),   // A = 4
    encode_clc(),    // Clear carry before add
    encode_add(0),   // A = 4 + 3 = 7
    encode_hlt(),
];
sim.run(&program, 100);
assert_eq!(sim.accumulator, 7);
```

### Subroutine example

```rust
use intel4004_simulator::*;

let mut sim = Intel4004Simulator::new(4096);
let jms = encode_jms(0x010);
let mut program = vec![
    encode_ldm(5),       // A = 5
    encode_xch(0),       // R0 = 5
    jms[0], jms[1],      // Call subroutine at 0x010
    encode_hlt(),        // Halt after return
];
// Pad to address 0x010
while program.len() < 0x10 { program.push(0x00); }
// Subroutine: double R0
program.extend_from_slice(&[
    encode_ld(0),        // A = R0
    encode_clc(),
    encode_add(0),       // A = R0 + R0
    encode_xch(0),       // R0 = 2 * R0
    encode_bbl(0),       // Return
]);
sim.run(&program, 100);
assert_eq!(sim.registers[0], 10); // 5 * 2 = 10
```

### RAM operations

```rust
use intel4004_simulator::*;

let mut sim = Intel4004Simulator::new(4096);
let fim = encode_fim(0, 0x00); // Set P0 for SRC addressing
let program = vec![
    fim[0], fim[1],
    encode_src(0),     // Point to RAM[0][0][0]
    encode_ldm(11),    // A = 11
    encode_wrm(),      // Write to RAM
    encode_ldm(0),     // Clear A
    encode_rdm(),      // Read back from RAM
    encode_hlt(),
];
sim.run(&program, 100);
assert_eq!(sim.accumulator, 11);
```
