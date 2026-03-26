# arithmetic

Integer arithmetic circuits built from logic gates — half adder, full adder, ripple-carry adder, ALU.

## Layer 9

This package is part of Layer 9 of the coding-adventures computing stack. It builds on top of the logic-gates package (Layer 10), wiring AND, OR, XOR, and NOT gates together to perform binary arithmetic.

## What's Inside

### Adder Circuits (`adder.lua`)

- **Half Adder** — Adds two single bits. Produces a sum (XOR) and a carry (AND). Called "half" because it cannot accept a carry input from a previous column.
- **Full Adder** — Adds two bits plus a carry-in by chaining two half adders and an OR gate. This is what makes multi-bit addition possible.
- **Ripple Carry Adder** — Chains N full adders to add two N-bit numbers. The carry "ripples" from the least significant bit to the most significant bit.

### ALU (`alu.lua`)

The Arithmetic Logic Unit is the mathematical heart of a CPU. It accepts two N-bit inputs (A and B) and an operation code, then produces a result along with four condition flags:

| Flag     | Meaning |
|----------|---------|
| Zero     | Every bit of the result is 0 |
| Carry    | Unsigned addition overflowed past the top bit |
| Negative | The most significant bit is 1 (negative in two's complement) |
| Overflow | Signed arithmetic produced an impossible sign change |

Supported operations: `add`, `sub`, `and`, `or`, `xor`, `not`.

Subtraction uses two's complement: `A - B = A + NOT(B) + 1`. The same adder circuit handles both addition and subtraction.

## Dependencies

- [logic-gates](../logic_gates/) (Layer 10)

## Usage

```lua
local arith = require("coding_adventures.arithmetic")

-- Half adder: add two bits
local sum, carry = arith.half_adder(1, 1)  -- sum=0, carry=1

-- Full adder: add two bits plus carry-in
local sum, cout = arith.full_adder(1, 1, 1)  -- sum=1, cout=1

-- Ripple carry adder: add two 4-bit numbers (5 + 3 = 8)
-- Numbers are LSB-first: 5 = 0101 -> {1,0,1,0}
local result, carry = arith.ripple_carry_adder(
    {1,0,1,0}, {1,1,0,0}, 0
)
-- result = {0,0,0,1} (8 in LSB-first binary), carry = 0

-- ALU: create a 4-bit ALU and execute operations
local alu = arith.ALU.new(4)
local res = alu:execute(arith.ADD, {1,0,1,0}, {1,1,0,0})
-- res.value = {0,0,0,1}
-- res.zero = false, res.carry = false
-- res.negative = true, res.overflow = true

-- Subtraction: 5 - 3 = 2
local res2 = alu:execute(arith.SUB, {1,0,1,0}, {1,1,0,0})
-- res2.value = {0,1,0,0} (2 in LSB-first binary)
```

## Bit Ordering

All multi-bit values use **little-endian** (LSB-first) ordering:

| Decimal | Binary (MSB-first) | Lua table (LSB-first) |
|---------|--------------------|-----------------------|
| 5       | 0101               | {1, 0, 1, 0}         |
| 3       | 0011               | {1, 1, 0, 0}         |
| 8       | 1000               | {0, 0, 0, 1}         |

## Development

```bash
# Run tests (from package root)
bash BUILD

# Run tests manually (from tests/ directory)
busted . --verbose --pattern=test_
```
