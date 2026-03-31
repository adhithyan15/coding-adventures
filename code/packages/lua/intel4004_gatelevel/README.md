# coding-adventures-intel4004-gatelevel (Lua)

Gate-level simulation of the Intel 4004 microprocessor. Every arithmetic
operation routes through actual logic gate functions from the `logic-gates`
and `arithmetic` packages — NOT through host language integer arithmetic.

## What makes this different from the behavioral simulator?

The behavioral simulator (`intel4004_simulator`) executes instructions
directly using Lua integers. This gate-level simulator:

1. Routes ADD through `ripple_carry_adder` → `full_adder` → `half_adder` → XOR/AND gates
2. Stores registers in D flip-flop state (via `Register` from logic_gates)
3. Increments the PC using a chain of 12 half-adders
4. Complements values using NOT gates

Both produce identical results for any program — the difference is how
they compute.

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, NOT, XOR, Register (flip-flops)
- `coding-adventures-arithmetic` — half_adder, full_adder, ripple_carry_adder

## Installation

```
luarocks make --local coding-adventures-intel4004-gatelevel-0.1.0-1.rockspec
```

## Usage

```lua
local Intel4004GL = require("coding_adventures.intel4004_gatelevel")

local cpu = Intel4004GL.new()

-- x = 1 + 2 (all addition routed through ripple_carry_adder)
local traces = cpu:run({0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}, 100)
print(cpu:_read_reg(1))  -- 3

-- Inspect gate count
local gc = cpu:gate_count()
print(gc.total)  -- ~716 gates (close to real 4004's ~786)
```
