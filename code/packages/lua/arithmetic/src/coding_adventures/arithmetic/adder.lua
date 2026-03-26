-- adder — Binary addition circuits built from logic gates
--
-- # Moving from Logic to Math
--
-- In the logic-gates package, we saw how transistors combine to form gates
-- that perform basic Boolean operations (AND, OR, XOR). But how do we get a
-- computer to do actual math?
--
-- This module answers that question. By creatively wiring together those
-- fundamental logic gates, we can build circuits that add binary numbers.
-- From a simple "Half Adder" that adds two individual bits, we build up to
-- a "Ripple Carry Adder" that handles multi-bit numbers — just like
-- grade-school column addition.
--
-- Each circuit is a pure function: bits in, bits out. No state, no side
-- effects. This mirrors how real hardware works — combinational logic
-- computes its outputs entirely from the current inputs.

local lg = require("coding_adventures.logic_gates")

local adder = {}

-- ========================================================================
-- Half Adder
-- ========================================================================
--
-- A half adder adds two single bits and produces two outputs:
--   * Sum   — the result in the current column
--   * Carry — the overflow into the next column
--
-- Why "half"? Because it can GENERATE a carry, but it cannot ACCEPT a
-- carry from a previous column. It only has two inputs, not three.
--
-- Think of it like adding two single-digit numbers in decimal: 7 + 8 = 15.
-- The "5" is the sum, and the "1" is the carry to the tens column. In
-- binary, 1 + 1 = 10 — the "0" is the sum, and the "1" is the carry.
--
-- Truth table:
--
--   A | B | Sum | Carry
--   --|---|-----|------
--   0 | 0 |  0  |   0
--   0 | 1 |  1  |   0
--   1 | 0 |  1  |   0
--   1 | 1 |  0  |   1
--
-- If you look closely at the truth table:
--   * Sum is exactly the XOR operation (1 only when inputs differ).
--   * Carry is exactly the AND operation (1 only when both inputs are 1).
--
-- Circuit diagram:
--
--   A ──┬──[XOR]── Sum
--       │
--   B ──┼──[AND]── Carry
--       │
--
-- @param a  First bit (0 or 1)
-- @param b  Second bit (0 or 1)
-- @return sum   The sum bit
-- @return carry The carry bit
function adder.half_adder(a, b)
    local sum = lg.XOR(a, b)
    local carry = lg.AND(a, b)
    return sum, carry
end

-- ========================================================================
-- Full Adder
-- ========================================================================
--
-- A full adder extends the half adder by accepting a THIRD input: a carry
-- bit from a previous column. This is what makes multi-bit addition
-- possible — every column beyond the first might receive a carry from the
-- column to its right.
--
-- How to build it? We chain two half adders together:
--
--   Step 1: Add A and B with a half adder.
--           This gives a partial_sum and a partial_carry.
--
--   Step 2: Add partial_sum to carry_in with a second half adder.
--           This gives the final sum and a second carry (carry2).
--
--   Step 3: If EITHER half adder generated a carry, the final carry_out
--           is 1. We use an OR gate to combine them.
--
-- Why does OR work here (instead of, say, XOR)? Because the two half
-- adders can never BOTH produce a carry of 1 at the same time. When
-- partial_sum is 0 (meaning A and B were both 1, so partial_carry = 1),
-- adding 0 + carry_in can produce at most a sum of carry_in with carry2 = 0.
-- So OR and XOR give the same result, but OR is the conventional choice
-- because it maps directly to the hardware wiring.
--
-- Truth table:
--
--   A | B | Cin | Sum | Cout
--   --|---|-----|-----|-----
--   0 | 0 |  0  |  0  |  0
--   0 | 0 |  1  |  1  |  0
--   0 | 1 |  0  |  1  |  0
--   0 | 1 |  1  |  0  |  1
--   1 | 0 |  0  |  1  |  0
--   1 | 0 |  1  |  0  |  1
--   1 | 1 |  0  |  0  |  1
--   1 | 1 |  1  |  1  |  1
--
-- @param a        First bit (0 or 1)
-- @param b        Second bit (0 or 1)
-- @param carry_in Carry from previous column (0 or 1)
-- @return sum       The sum bit
-- @return carry_out The carry to the next column
function adder.full_adder(a, b, carry_in)
    local partial_sum, partial_carry = adder.half_adder(a, b)
    local sum, carry2 = adder.half_adder(partial_sum, carry_in)
    local carry_out = lg.OR(partial_carry, carry2)
    return sum, carry_out
end

-- ========================================================================
-- Ripple Carry Adder
-- ========================================================================
--
-- A ripple carry adder chains N full adders together to add two N-bit
-- binary numbers. It works exactly like grade-school long addition:
-- start from the rightmost (least significant) column and move left,
-- passing the carry from each column into the next.
--
-- The name "ripple" comes from how the carry propagates: the carry from
-- bit 0 feeds into bit 1, whose carry feeds into bit 2, and so on. In
-- the worst case (like 1111 + 0001), the carry must "ripple" through
-- every single adder before the final result is ready.
--
-- In real hardware, this ripple delay limits speed. Modern CPUs use
-- faster designs like "Carry Lookahead Adders" that compute carries in
-- parallel. But the ripple carry adder is the simplest to understand
-- and the foundation that all faster designs build upon.
--
-- Bit ordering: Little-Endian (LSB at index 1 in Lua).
--
-- Example: Adding 5 + 3 = 8
--
--   5 in binary: 0101 → {1, 0, 1, 0} (LSB first)
--   3 in binary: 0011 → {1, 1, 0, 0} (LSB first)
--
--   Column 0: FullAdder(1, 1, 0) → Sum=0, Carry=1
--   Column 1: FullAdder(0, 1, 1) → Sum=0, Carry=1
--   Column 2: FullAdder(1, 0, 1) → Sum=0, Carry=1
--   Column 3: FullAdder(0, 0, 1) → Sum=1, Carry=0
--
--   Result: {0, 0, 0, 1} → 1000 in binary → 8. Correct!
--
-- @param a        Table of bits (0 or 1), LSB at index 1
-- @param b        Table of bits (0 or 1), same length as a
-- @param carry_in Initial carry input (usually 0)
-- @return sum_bits  Table of result bits, same length as inputs
-- @return carry_out Final carry out (1 means unsigned overflow)
function adder.ripple_carry_adder(a, b, carry_in)
    assert(#a == #b, "a and b must have the same length")
    assert(#a > 0, "bit lists must not be empty")

    local sum_bits = {}
    local carry = carry_in

    for i = 1, #a do
        local sum_bit
        sum_bit, carry = adder.full_adder(a[i], b[i], carry)
        sum_bits[i] = sum_bit
    end

    return sum_bits, carry
end

return adder
