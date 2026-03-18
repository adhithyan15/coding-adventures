# coding_adventures_arithmetic

**Arithmetic circuits built from logic gates** -- Layer 9 of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) computing stack.

## What is this?

This gem implements the arithmetic circuits that live inside every CPU's datapath. Every calculation a computer performs -- from `1 + 2` to complex floating-point math -- ultimately passes through circuits like these.

We build arithmetic in layers:

1. **Half Adder** -- adds two single bits (sum + carry)
2. **Full Adder** -- adds two bits plus a carry-in from a previous stage
3. **Ripple-Carry Adder** -- chains N full adders for N-bit addition
4. **ALU (Arithmetic Logic Unit)** -- the computational heart of a CPU, supporting ADD, SUB, AND, OR, XOR, and NOT

Every operation is built entirely from logic gates (AND, OR, XOR, NOT). No Ruby arithmetic operators are used in the core logic.

## Where it fits in the stack

```
Logic Gates (Layer 10) -- foundation
    |
    v
Arithmetic (Layer 9)  <-- YOU ARE HERE
    |
    v
CPU Simulator (Layer 8)
    |
    v
...higher layers...
```

**Depends on:** `coding_adventures_logic_gates` (Layer 10)
**Used by:** `coding_adventures_cpu_simulator` (Layer 8)

## Installation

```ruby
# In your Gemfile
gem "coding_adventures_arithmetic"
```

## Usage

```ruby
require "coding_adventures_arithmetic"

include CodingAdventures

# Half adder: adds two single bits
result = Arithmetic.half_adder(1, 1)
result.sum   # => 0
result.carry # => 1

# Full adder: adds two bits + carry-in
result = Arithmetic.full_adder(1, 0, 1)
result.sum   # => 0
result.carry # => 1

# Ripple-carry adder: adds two 4-bit numbers (LSB first)
a = [1, 0, 1, 0]  # 5 in binary
b = [1, 1, 0, 0]  # 3 in binary
result = Arithmetic.ripple_carry_adder(a, b)
result.bits  # => [0, 0, 0, 1]  (8 in binary)
result.carry # => 0

# ALU: the computational core
alu = Arithmetic::ALU.new(bit_width: 8)

a = [1, 0, 0, 0, 0, 0, 0, 0]  # 1
b = [0, 1, 0, 0, 0, 0, 0, 0]  # 2
result = alu.execute(Arithmetic::ALUOp::ADD, a, b)
# result.result => [1, 1, 0, 0, 0, 0, 0, 0]  (3)
# result.zero     => false
# result.carry    => false
# result.negative => false
# result.overflow => false
```

## ALU Operations

| Operation | Description | Formula |
|-----------|-------------|---------|
| `ALUOp::ADD` | Addition | `A + B` via ripple-carry adder |
| `ALUOp::SUB` | Subtraction | `A + NOT(B) + 1` (two's complement) |
| `ALUOp::AND` | Bitwise AND | Each bit: `AND(a_i, b_i)` |
| `ALUOp::OR`  | Bitwise OR  | Each bit: `OR(a_i, b_i)` |
| `ALUOp::XOR` | Bitwise XOR | Each bit: `XOR(a_i, b_i)` |
| `ALUOp::NOT` | Bitwise NOT | Each bit: `NOT(a_i)` (b ignored) |

## Status Flags

| Flag | Meaning |
|------|---------|
| `zero` | Result is all zeros |
| `carry` | Unsigned overflow occurred |
| `negative` | Most significant bit is 1 (sign bit) |
| `overflow` | Signed overflow occurred |

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
