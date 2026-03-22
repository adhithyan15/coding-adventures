# intel4004-simulator

Intel 4004 simulator -- the world's first commercial microprocessor (1971).

## What is this?

This crate simulates the Intel 4004, a 4-bit accumulator-based processor with 16 registers, a 3-level hardware stack, 4-bank RAM with status characters, and ROM/RAM I/O ports. All data values are 4 bits wide (0-15). All 46 instructions from the original 4004 instruction set are implemented, plus a custom HLT instruction for testing.

## Architecture

```text
+-------------------------------------------+
|           Intel 4004                       |
|  Accumulator:     4 bits                   |
|  Registers:       16 x 4 bits             |
|  Carry:           1 bit                   |
|  Program memory:  up to 4096 bytes (ROM)  |
|  Data RAM:        4 x 4 x 16 nibbles     |
|  RAM status:      4 x 4 x 4 nibbles      |
|  Hardware stack:  3-level x 12-bit addrs  |
|  ROM port:        4 bits                  |
|  RAM output port: 4 bits per bank         |
+-------------------------------------------+
```

## Supported Instructions (46 total)

### Machine control
| Byte | Mnemonic | Description |
|------|----------|-------------|
| 0x00 | NOP | No operation |
| 0x01 | HLT | Halt (custom) |

### Register and immediate
| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| 0xA_ | LD Rn | Load register into accumulator |
| 0xB_ | XCH Rn | Exchange accumulator with register |
| 0xD_ | LDM N | Load immediate N into accumulator |
| 0x6_ | INC Rn | Increment register |

### Arithmetic
| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| 0x8_ | ADD Rn | Add register to accumulator (with carry) |
| 0x9_ | SUB Rn | Subtract register from accumulator (complement-add) |

### Jump and subroutine
| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| 0x1_ | JCN | Conditional jump (2-byte) |
| 0x4_ | JUN | Unconditional jump (2-byte) |
| 0x5_ | JMS | Jump to subroutine (2-byte) |
| 0x7_ | ISZ | Increment and skip if zero (2-byte) |
| 0xC_ | BBL N | Branch back and load (return) |

### Register pair
| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| 0x2_ (even) | FIM | Fetch immediate to register pair (2-byte) |
| 0x2_ (odd) | SRC | Send register control (set RAM address) |
| 0x3_ (even) | FIN | Fetch indirect from ROM |
| 0x3_ (odd) | JIN | Jump indirect |

### I/O and RAM (0xE0-0xEF)
WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3

### Accumulator group (0xF0-0xFD)
CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL

## Usage

```rust
use intel4004_simulator::*;

let mut sim = Intel4004Simulator::new(4096);

// BCD addition: 8 + 5 = 13 (carry=1, digit=3)
let program = vec![
    encode_ldm(5),   // A = 5
    encode_xch(0),   // R0 = 5, A = 0
    encode_ldm(8),   // A = 8
    encode_clc(),    // Clear carry
    encode_add(0),   // A = 8 + 5 = 13
    encode_daa(),    // Decimal adjust: A = 3, carry = 1
    encode_hlt(),
];
sim.run(&program, 100);
assert_eq!(sim.accumulator, 3);
assert!(sim.carry);
```

## Subroutine example

```rust
use intel4004_simulator::*;

let mut program = vec![0u8; 256];

// Main: call subroutine at 0x010
let (b1, b2) = encode_jms(0x010);
program[0] = b1;
program[1] = b2;
program[2] = encode_hlt();

// Subroutine: return with A = 7
program[0x010] = encode_bbl(7);

let mut sim = Intel4004Simulator::new(4096);
sim.run(&program, 100);
assert_eq!(sim.accumulator, 7);
```
