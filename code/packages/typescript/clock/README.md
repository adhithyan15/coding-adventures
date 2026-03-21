# @coding-adventures/clock (TypeScript)

System clock generator -- the heartbeat of every digital circuit.

## What is this?

In real hardware, a crystal oscillator generates a square wave that synchronizes every component in a computer. This package simulates that clock signal.

Every sequential circuit -- flip-flops, registers, counters, CPU pipeline stages -- is driven by a clock. The clock alternates between 0 and 1:

```
+--+  +--+  +--+  +--+
|  |  |  |  |  |  |  |
---+  +--+  +--+  +--+  +--
```

On each rising edge (0->1), components capture their inputs. This is synchronous design.

## How it fits in the stack

The clock is a fundamental hardware abstraction that sits alongside logic gates. While logic gates handle combinational logic (outputs depend only on current inputs), the clock enables sequential logic (outputs depend on history). Together, they form the foundation for flip-flops, registers, counters, and eventually a full CPU.

## Components

- **Clock**: Square-wave generator with configurable frequency, cycle counting, and an observer pattern for connected components.
- **ClockDivider**: Derives a slower clock from a faster master clock by counting rising edges.
- **MultiPhaseClock**: Generates multiple non-overlapping clock phases for pipeline stages.

## Installation

```bash
npm install @coding-adventures/clock
```

Or for development:

```bash
npm install
```

## Usage

```typescript
import { Clock, ClockDivider, MultiPhaseClock } from "@coding-adventures/clock";

// Create a 1 MHz clock
const clk = new Clock(1_000_000);

// Tick produces edges
let edge = clk.tick();       // rising edge, cycle 1
edge = clk.tick();           // falling edge, cycle 1

// Run for N complete cycles
const edges = clk.run(100);  // 200 edges (100 rising + 100 falling)

// Connect components via listeners
clk.registerListener((edge) => {
  if (edge.isRising) {
    console.log(`Cycle ${edge.cycle}: rising edge!`);
  }
});

// Divide a fast clock for a slower bus
const master = new Clock(1_000_000_000);  // 1 GHz
const divider = new ClockDivider(master, 4);
// divider.output runs at 250 MHz

// Generate pipeline phases
const phases = new MultiPhaseClock(master, 4);
master.tick();
phases.getPhase(0);  // 1 (active)
phases.getPhase(1);  // 0 (inactive)
```

## Running tests

```bash
npm install
npx vitest run --coverage
```
