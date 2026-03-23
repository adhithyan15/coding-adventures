# @coding-adventures/block-ram

Block RAM — SRAM cells, arrays, and synchronous RAM modules for the computing stack.

## What is Block RAM?

Block RAM (BRAM) is dedicated memory embedded in FPGAs and used for CPU caches. This package models memory from the gate level up:

- **SRAMCell**: Single-bit storage element (6-transistor model)
- **SRAMArray**: 2D grid of SRAM cells with row/column addressing
- **SinglePortRAM**: Synchronous RAM with configurable read modes
- **DualPortRAM**: True dual-port RAM with collision detection
- **ConfigurableBRAM**: FPGA-style reconfigurable Block RAM

## Usage

```typescript
import { SinglePortRAM, ReadMode } from "@coding-adventures/block-ram";

// Create a 256-word x 8-bit RAM
const ram = new SinglePortRAM(256, 8, ReadMode.READ_FIRST);

// Write [1,0,1,0,0,1,0,1] to address 0
ram.tick(0, 0, [1,0,1,0,0,1,0,1], 1);  // clock low
ram.tick(1, 0, [1,0,1,0,0,1,0,1], 1);  // rising edge triggers write

// Read from address 0
ram.tick(0, 0, [0,0,0,0,0,0,0,0], 0);
const data = ram.tick(1, 0, [0,0,0,0,0,0,0,0], 0);
// data === [1, 0, 1, 0, 0, 1, 0, 1]
```

## Dependencies

- `@coding-adventures/logic-gates` — Bit type and validation

## How it fits in the stack

```
Layer 1: logic-gates (AND, OR, NOT, flip-flops)
Layer 3: block-ram (THIS PACKAGE — SRAM, RAM modules)
Layer 4: fpga (LUTs, CLBs, routing fabric using block-ram for storage)
```
