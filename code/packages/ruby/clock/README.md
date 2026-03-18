# coding_adventures_clock (Ruby)

System clock generator -- the heartbeat of every digital circuit.

## What is this?

In real hardware, a crystal oscillator generates a square wave that synchronizes every component in a computer. This gem simulates that clock signal.

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

- **ClockGenerator**: Square-wave generator with configurable frequency, cycle counting, and an observer pattern for connected components.
- **ClockDivider**: Derives a slower clock from a faster master clock by counting rising edges.
- **MultiPhaseClock**: Generates multiple non-overlapping clock phases for pipeline stages.
- **ClockEdge**: Immutable record of a clock transition (Data.define).

## Installation

```bash
gem install coding_adventures_clock
```

Or for development:

```bash
bundle install
```

## Usage

```ruby
require "coding_adventures_clock"

# Create a 1 MHz clock
clk = CodingAdventures::Clock::ClockGenerator.new(frequency_hz: 1_000_000)

# Tick produces edges
edge = clk.tick       # rising edge, cycle 1
edge = clk.tick       # falling edge, cycle 1

# Run for N complete cycles
edges = clk.run(100)  # 200 edges (100 rising + 100 falling)

# Connect components via listeners
listener = ->(edge) { puts "Cycle #{edge.cycle}: rising!" if edge.rising? }
clk.register_listener(listener)

# Divide a fast clock for a slower bus
master = CodingAdventures::Clock::ClockGenerator.new(frequency_hz: 1_000_000_000)
divider = CodingAdventures::Clock::ClockDivider.new(source: master, divisor: 4)
# divider.output runs at 250 MHz

# Generate pipeline phases
phases = CodingAdventures::Clock::MultiPhaseClock.new(source: master, phases: 4)
master.tick
phases.get_phase(0)  # => 1 (active)
phases.get_phase(1)  # => 0 (inactive)
```

## Running tests

```bash
bundle install
bundle exec rake test
```
