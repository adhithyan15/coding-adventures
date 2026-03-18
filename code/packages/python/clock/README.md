# coding-adventures-clock (Python)

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
pip install coding-adventures-clock
```

Or for development:

```bash
uv venv && uv pip install -e ".[dev]"
```

## Usage

```python
from clock import Clock, ClockDivider, MultiPhaseClock

# Create a 1 MHz clock
clk = Clock(frequency_hz=1_000_000)

# Tick produces edges
edge = clk.tick()       # rising edge, cycle 1
edge = clk.tick()       # falling edge, cycle 1

# Run for N complete cycles
edges = clk.run(100)    # 200 edges (100 rising + 100 falling)

# Connect components via listeners
def on_edge(edge):
    if edge.is_rising:
        print(f"Cycle {edge.cycle}: rising edge!")

clk.register_listener(on_edge)

# Divide a fast clock for a slower bus
master = Clock(frequency_hz=1_000_000_000)  # 1 GHz
divider = ClockDivider(master, divisor=4)
# divider.output runs at 250 MHz

# Generate pipeline phases
phases = MultiPhaseClock(master, phases=4)
master.tick()
phases.get_phase(0)  # 1 (active)
phases.get_phase(1)  # 0 (inactive)
```

## Running tests

```bash
uv venv && uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v --cov=clock --cov-report=term-missing
```
