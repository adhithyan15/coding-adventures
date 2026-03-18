# clock (Go)

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
- **ClockEdge**: Struct recording a clock transition.

## Installation

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/clock"
```

## Usage

```go
package main

import (
    "fmt"
    "github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

func main() {
    // Create a 1 MHz clock
    clk := clock.New(1_000_000)

    // Tick produces edges
    edge := clk.Tick()     // rising edge, cycle 1
    edge = clk.Tick()      // falling edge, cycle 1

    // Run for N complete cycles
    edges := clk.Run(100)  // 200 edges (100 rising + 100 falling)

    // Connect components via listeners
    clk.RegisterListener(func(e clock.ClockEdge) {
        if e.IsRising {
            fmt.Printf("Cycle %d: rising edge!\n", e.Cycle)
        }
    })

    // Divide a fast clock for a slower bus
    master := clock.New(1_000_000_000)  // 1 GHz
    divider, _ := clock.NewClockDivider(master, 4)
    // divider.Output runs at 250 MHz

    // Generate pipeline phases
    phases, _ := clock.NewMultiPhaseClock(master, 4)
    master.Tick()
    phases.GetPhase(0)  // 1 (active)
    phases.GetPhase(1)  // 0 (inactive)
}
```

## Running tests

```bash
go test ./... -v -cover
```
