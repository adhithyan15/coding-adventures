# @coding-adventures/intel4004-simulator

Intel 4004 microprocessor simulator -- Layer 4d of the computing stack.

## What is this?

A TypeScript implementation of the world's first commercial microprocessor, the Intel 4004 (1971). This simulator demonstrates the accumulator architecture with 4-bit data width, where all computation flows through a single register.

## Supported Instructions

| Instruction | Encoding | Description |
|------------|----------|-------------|
| `LDM N` | 0xDN | Load immediate N into accumulator |
| `XCH RN` | 0xBN | Exchange accumulator with register N |
| `ADD RN` | 0x8N | Add register N to accumulator |
| `SUB RN` | 0x9N | Subtract register N from accumulator |
| `HLT` | 0x01 | Halt execution |

## Usage

```typescript
import { Intel4004Simulator } from "@coding-adventures/intel4004-simulator";

const sim = new Intel4004Simulator();
// x = 1 + 2: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
const traces = sim.run(new Uint8Array([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]));
console.log(sim.registers[1]); // => 3
```

## How it fits in the stack

This is a TypeScript port of the Python intel4004-simulator package. It sits at Layer 4d, demonstrating how the first microprocessor worked with its accumulator-based architecture.
