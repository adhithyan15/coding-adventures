# @coding-adventures/intel4004-simulator

Intel 4004 microprocessor simulator -- Layer 4d of the computing stack.

## What is this?

A TypeScript implementation of the world's first commercial microprocessor, the Intel 4004 (1971). This simulator implements all 46 real 4004 instructions, demonstrating the accumulator architecture with 4-bit data width, where all computation flows through a single register.

## Architecture

| Component | Size | Description |
|-----------|------|-------------|
| Data width | 4 bits | Values 0-15 |
| Accumulator | 4 bits | Center of all computation |
| Registers | 16 x 4-bit | R0-R15, organized as 8 pairs |
| Carry flag | 1 bit | Set on overflow/borrow |
| Program counter | 12 bits | Addresses 4096 bytes of ROM |
| Stack | 3-level | 12-bit return addresses, wraps mod 3 |
| ROM | 4096 bytes | Program storage |
| RAM | 4 banks x 4 regs x 20 nibbles | 16 main + 4 status per register |

## Supported Instructions (46)

| Instruction | Encoding | Description |
|------------|----------|-------------|
| `NOP` | 0x00 | No operation |
| `HLT` | 0x01 | Halt execution (simulator-only) |
| `JCN c,a` | 0x1C 0xAA | Conditional jump |
| `FIM Pp,d` | 0x2P 0xDD | Fetch immediate to register pair |
| `SRC Pp` | 0x2P+1 | Send register control (RAM address) |
| `FIN Pp` | 0x3P | Fetch indirect from ROM via P0 |
| `JIN Pp` | 0x3P+1 | Jump indirect via register pair |
| `JUN a` | 0x4H 0xLL | Unconditional jump (12-bit) |
| `JMS a` | 0x5H 0xLL | Jump to subroutine |
| `INC Rn` | 0x6N | Increment register |
| `ISZ Rn,a` | 0x7N 0xAA | Increment and skip if zero |
| `ADD Rn` | 0x8N | Add register to accumulator with carry |
| `SUB Rn` | 0x9N | Subtract register (complement-add) |
| `LD Rn` | 0xAN | Load register into accumulator |
| `XCH Rn` | 0xBN | Exchange accumulator and register |
| `BBL n` | 0xCN | Branch back and load (return) |
| `LDM n` | 0xDN | Load immediate into accumulator |
| `WRM` | 0xE0 | Write accumulator to RAM |
| `WMP` | 0xE1 | Write accumulator to RAM output port |
| `WRR` | 0xE2 | Write accumulator to ROM I/O port |
| `WPM` | 0xE3 | Write program RAM (NOP in simulator) |
| `WR0`-`WR3` | 0xE4-0xE7 | Write accumulator to RAM status |
| `SBM` | 0xE8 | Subtract RAM from accumulator |
| `RDM` | 0xE9 | Read RAM into accumulator |
| `RDR` | 0xEA | Read ROM I/O port into accumulator |
| `ADM` | 0xEB | Add RAM to accumulator with carry |
| `RD0`-`RD3` | 0xEC-0xEF | Read RAM status into accumulator |
| `CLB` | 0xF0 | Clear both (A=0, carry=0) |
| `CLC` | 0xF1 | Clear carry |
| `IAC` | 0xF2 | Increment accumulator |
| `CMC` | 0xF3 | Complement carry |
| `CMA` | 0xF4 | Complement accumulator |
| `RAL` | 0xF5 | Rotate left through carry |
| `RAR` | 0xF6 | Rotate right through carry |
| `TCC` | 0xF7 | Transfer carry to accumulator |
| `DAC` | 0xF8 | Decrement accumulator |
| `TCS` | 0xF9 | Transfer carry subtract |
| `STC` | 0xFA | Set carry |
| `DAA` | 0xFB | Decimal adjust accumulator |
| `KBP` | 0xFC | Keyboard process |
| `DCL` | 0xFD | Designate command line (select RAM bank) |

## Usage

```typescript
import { Intel4004Simulator } from "@coding-adventures/intel4004-simulator";

// x = 1 + 2: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
const sim = new Intel4004Simulator();
const traces = sim.run(new Uint8Array([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]));
console.log(sim.registers[1]); // => 3

// Subroutine call
const sub = new Intel4004Simulator();
sub.run(new Uint8Array([
  0x50, 0x04,  // JMS 0x004 (call subroutine)
  0x01,        // HLT (return point)
  0x00,        // NOP
  0xC7,        // BBL 7 (subroutine: return with A=7)
]));
console.log(sub.accumulator); // => 7

// RAM read/write
const ram = new Intel4004Simulator();
ram.run(new Uint8Array([
  0x20, 0x00,  // FIM P0,0x00 (set RAM address)
  0x21,        // SRC P0
  0xD9,        // LDM 9
  0xE0,        // WRM (write 9 to RAM)
  0xD0,        // LDM 0
  0xE9,        // RDM (read RAM back)
  0x01,        // HLT
]));
console.log(ram.accumulator); // => 9
```

## How it fits in the stack

This is a TypeScript port of the Python intel4004-simulator package. It sits at Layer 4d, demonstrating how the first microprocessor worked with its accumulator-based architecture. Unlike the Python version, this TypeScript implementation is standalone (no virtual-machine dependency).

## Key design decisions

- **SUB uses complement-add**: `A = A + ~Rn + (carry ? 0 : 1)`. Carry=true means NO borrow.
- **ADD includes carry**: `A = A + Rn + carry`. This enables multi-digit BCD arithmetic.
- **3-level stack wraps mod 3**: No stack overflow -- the 4th push silently overwrites.
- **KBP table**: `{0->0, 1->1, 2->2, 4->3, 8->4, else->15}` for keyboard scanning.
- **DAA**: If A>9 or carry, add 6 for BCD correction.
