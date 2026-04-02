# coding-adventures-arm1-gatelevel (Lua)

Gate-level ARM1 processor simulator — every operation routes through actual logic gate function calls.

## What Makes This Different From The Behavioral Simulator?

The behavioral simulator executes instructions directly with host-language arithmetic:
```
ADD R0,R1,R2 → result = reg[1] + reg[2]
```

This gate-level simulator routes everything through gates:
```
ADD R0,R1,R2 → a_bits = int_to_bits(reg[1])
             → b_bits = int_to_bits(reg[2])
             → sum_bits, carry = ripple_carry_adder(a_bits, b_bits, 0)
             →   each full_adder calls: XOR(XOR(a,b),cin), AND(a,b), AND(xor,cin), OR(and1,and2)
             → result = bits_to_int(sum_bits)
```

Every `ADD` leaves a trace of ~200 gate function calls. Every `SUB` does `NOT` on 32 bits first (32 gate calls), then `ripple_carry_adder` (~160 gate calls). The barrel shifter uses a 5-level Mux2 tree — 160 multiplexer calls per shift.

## Why?

- **Count gates**: How many AND/OR/NOT operations does `ADD R0, R1, R2, LSL #3` require?
- **Trace signals**: Follow a bit from register R2 through the barrel shifter, through the adder, into R0.
- **Understand the barrel shifter**: The ARM1's most distinctive hardware feature — a crossbar of pass transistors — modeled as Mux2 gate trees.
- **See where the transistors go**: The ARM1 had ~25,000 transistors. Our simulator gives you a gate-level view of that complexity.

## Architecture

```
Register File (27 × 32 bits — stored as bit arrays, LSB first)
    │
    ▼
Barrel Shifter (5-level Mux2 tree, 160 gate calls per shift)
    │
    ▼
Gate-Level ALU:
  Logical ops: AND/OR/XOR/NOT applied to 32 bit pairs (32 gate calls)
  Arithmetic:  ripple_carry_adder → 32 full adders → ~160 gate calls
    │
    ▼
Condition Evaluator (4-5 gate calls for most conditions)
```

## Installation

```bash
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
```

## Usage

```lua
local GL   = require("coding_adventures.arm1_gatelevel")
local ARM1 = require("coding_adventures.arm1_simulator")

local cpu = GL.new(4096)
GL.load_instructions(cpu, {
    ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
    ARM1.encode_halt(),
})
local traces = GL.run(cpu, 100)
print(GL.read_register(cpu, 0))  -- 42
print(cpu.gate_ops)              -- gate operations performed
```

## API

The gate-level simulator has the same API as `arm1_simulator`:

```lua
local cpu = GL.new(memory_size)
GL.reset(cpu)
GL.read_register(cpu, n)
GL.write_register(cpu, n, value)
GL.get_pc(cpu)
GL.get_flags(cpu)       -- returns { n, z, c, v } (booleans)
GL.get_mode(cpu)
GL.read_word(cpu, addr)
GL.write_word(cpu, addr, value)
GL.read_byte(cpu, addr)
GL.write_byte(cpu, addr, value)
GL.load_instructions(cpu, {word, ...})
local trace = GL.step(cpu)
local traces = GL.run(cpu, max_steps)
```

## Gate-Level Functions

```lua
-- Convert between integers and bit arrays (LSB first)
GL.int_to_bits(value, width)  -- returns {0,1,...}
GL.bits_to_int(bits)          -- returns integer

-- Gate-level ALU (all 16 ARM1 opcodes)
GL.gate_alu_execute(opcode, a_bits, b_bits, carry_in, shifter_carry, old_v)
-- returns { result_bits, n, z, c, v }

-- Gate-level barrel shifter (5-level Mux2 tree)
GL.gate_barrel_shift(value_bits, shift_type, amount, carry_in, by_register)
-- returns result_bits, carry_out

-- Gate-level rotated immediate
GL.gate_decode_immediate(imm8, rotate)
-- returns result_bits, carry_out
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
