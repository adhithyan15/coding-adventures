# clock

Cycle-accurate clock simulation with edge detection and observers.

## Layer 8

This package is part of Layer 8 of the coding-adventures computing stack. It simulates the system clock that drives all sequential logic in a digital circuit -- flip-flops, registers, counters, CPU pipeline stages, and GPU cores all depend on a clock signal to synchronize their operations.

Ported from the Go implementation at `code/packages/go/clock/`.

## API

### Clock

The main square-wave generator. Creates a clock signal that alternates between 0 and 1.

```lua
local clock = require("coding_adventures.clock")

-- Create a 1 MHz clock
local clk = clock.Clock.new(1000000)

-- Advance one half-cycle (returns a ClockEdge)
local edge = clk:tick()         -- rising edge, cycle 1
edge = clk:tick()               -- falling edge, cycle 1
edge = clk:tick()               -- rising edge, cycle 2

-- Run a complete cycle (rising + falling)
local rising, falling = clk:full_cycle()

-- Run N complete cycles, collect all edges
local edges = clk:run(10)       -- returns 20 ClockEdge records

-- Query clock state
clk.frequency_hz                -- frequency in Hz
clk.cycle                       -- current cycle count
clk.value                       -- current signal level (0 or 1)
clk:total_ticks()               -- total half-cycles elapsed
clk:period_ns()                 -- period in nanoseconds

-- Reset timing state (preserves listeners)
clk:reset()
```

### Listeners (Observer Pattern)

Components register listeners to react to clock edges, mirroring how real hardware connects to a clock wire.

```lua
clk:register_listener(function(edge)
    if edge.is_rising then
        print("Rising edge at cycle " .. edge.cycle)
    end
end)

clk:listener_count()            -- number of registered listeners
clk:unregister_listener(1)      -- remove listener at index (1-based)
```

### ClockEdge

An immutable record of a single clock transition.

```lua
local edge = clock.ClockEdge.new(1, 1, true, false)

edge.cycle       -- which cycle (starts at 1)
edge.value       -- signal level after transition (0 or 1)
edge.is_rising   -- true if 0->1 transition
edge.is_falling  -- true if 1->0 transition
```

### ClockDivider

Generates a slower clock from a faster source by counting rising edges.

```lua
-- Create a 4 GHz master, divide by 4 to get 1 GHz
local master = clock.Clock.new(4000000000)
local divider = clock.ClockDivider.new(master, 4)

master:run(8)                          -- 8 master cycles
print(divider.output.cycle)            -- 2 output cycles
print(divider.output.frequency_hz)     -- 1000000000
```

### MultiPhaseClock

Generates multiple non-overlapping clock phases from a single source. Used in CPU pipelines where different stages (fetch, decode, execute, writeback) need offset clocks.

```lua
-- Create a 4-phase pipeline clock
local master = clock.Clock.new(1000000)
local mpc = clock.MultiPhaseClock.new(master, 4)

master:tick()                   -- rising: phase 0 active
mpc:get_phase(0)                -- 1 (active)
mpc:get_phase(1)                -- 0 (inactive)

master:full_cycle()             -- next rising: phase 1 active
mpc:get_phase(0)                -- 0
mpc:get_phase(1)                -- 1
```

Phase indices are 0-based (phase 0 through phase N-1).

## Development

```bash
# Run tests (requires busted)
cd tests && busted . --verbose --pattern=test_

# Or use the BUILD script
bash BUILD
```
