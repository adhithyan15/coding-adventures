# @coding-adventures/fpga

FPGA -- Field-Programmable Gate Array abstraction for the computing stack.

## What is an FPGA?

An FPGA is a chip containing configurable logic blocks, routing fabric, I/O blocks, and block RAM -- all programmable via a bitstream. This package models the FPGA architecture from the atomic LUT up through the complete fabric.

## Components

- **LUT**: K-input Look-Up Table -- stores a truth table in SRAM, evaluates via MUX tree
- **Slice**: 2 LUTs + 2 flip-flops + carry chain
- **CLB**: Configurable Logic Block -- 2 slices with carry chain interconnect
- **SwitchMatrix**: Programmable routing crossbar
- **IOBlock**: Bidirectional I/O pad (input, output, tri-state)
- **Bitstream**: JSON configuration format for programming the fabric
- **FPGA**: Top-level fabric model

## Usage

```typescript
import { FPGA, Bitstream } from "@coding-adventures/fpga";
import type { Bit } from "@coding-adventures/logic-gates";

// Create an AND gate truth table for a 4-input LUT
const andTt = Array(16).fill(0) as Bit[];
andTt[3] = 1; // I0=1 AND I1=1

// Configure the FPGA via bitstream
const bs = Bitstream.fromObject({
  clbs: {
    clb_0: {
      slice0: { lutA: andTt },
      slice1: {},
    },
  },
  io: {
    a: { mode: "input" },
    b: { mode: "input" },
    y: { mode: "output" },
  },
});

const fpga = new FPGA(bs);

// Evaluate the CLB
const out = fpga.evaluateCLB(
  "clb_0",
  [1, 1, 0, 0], [0, 0, 0, 0],
  [0, 0, 0, 0], [0, 0, 0, 0],
  0,
);

fpga.driveOutput("y", out.slice0.outputA);
fpga.readOutput("y"); // 1
```

## Dependencies

- `@coding-adventures/logic-gates` -- gates, MUX, flip-flops, combinational circuits
- `@coding-adventures/block-ram` -- SRAM cells for LUT storage

## How it fits in the stack

```
Layer 1: logic-gates (AND, OR, NOT, MUX, flip-flops)
Layer 3: block-ram (SRAM, RAM modules)
Layer 4: fpga (THIS PACKAGE -- LUTs, CLBs, routing, fabric)
```
