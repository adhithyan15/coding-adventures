# coding-adventures-arm1-gatelevel (Lua)

Gate-level ARM1 processor simulation in Lua 5.4.

Where the behavioral simulator computes `result = a + b` directly, this
simulator computes the same result by routing every bit through logic gate
function calls — AND, OR, NOT, XOR, XNOR — using a ripple-carry adder.
The barrel shifter is a 5-level Mux2 tree: 32 multiplexers per level,
5 levels, ~160 gate calls per shift.

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, NOT, XOR, XNOR primitives
- `coding-adventures-arithmetic` — ripple_carry_adder
- `coding-adventures-arm1-simulator` — instruction decode, memory, encode helpers

## Installation

```bash
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Usage

```lua
local GL   = require("coding_adventures.arm1_gatelevel")
local ARM1 = require("coding_adventures.arm1_simulator")

-- Build a small program
local cpu = GL.new(4096)
ARM1.load_instructions(cpu, 0, {
    ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),   -- MOV R0, #42
    ARM1.encode_halt(),
})

GL.run(cpu, 100)
print(ARM1.read_register(cpu, 0))   -- 42
print(cpu.gate_ops)                 -- number of gate calls made
```

## Gate Count

Each data processing instruction contributes approximately 200 gate calls:
- 32 calls to convert each operand to bits
- 32 calls for a logical op, or ~160 for an adder
- ~1 call for condition evaluation

The `cpu.gate_ops` field tracks the running total.
