-- Tests for the arithmetic package.
--
-- These tests verify that our binary arithmetic circuits produce correct
-- results for all input combinations. We test at three levels:
--
--   1. Half Adder   — exhaustive (only 4 input combinations)
--   2. Full Adder   — exhaustive (only 8 input combinations)
--   3. Ripple Carry — representative examples with known decimal equivalents
--   4. ALU          — all six operations with flag verification
--
-- The tests use binary numbers in LSB-first (little-endian) format, which
-- is how the circuits process them internally. Comments show the MSB-first
-- (human-readable) binary and decimal equivalents for clarity.

-- Add both arithmetic src/ and logic_gates src/ to the module search path.
-- This allows tests to run without luarocks install in the local dev environment.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. "../../logic_gates/src/?.lua;" .. "../../logic_gates/src/?/init.lua;" .. package.path

local arithmetic = require("coding_adventures.arithmetic")

-- ========================================================================
-- Helper function for comparing bit arrays
-- ========================================================================
--
-- Lua tables don't support equality comparison by default, so we need
-- a helper that checks element-by-element equality.
local function bits_equal(a, b)
    if #a ~= #b then return false end
    for i = 1, #a do
        if a[i] ~= b[i] then return false end
    end
    return true
end

-- Format a bit array as a string for readable test failure messages.
-- Shows MSB-first (the way humans read binary) with a decimal annotation.
local function bits_to_str(bits)
    local reversed = {}
    for i = #bits, 1, -1 do
        reversed[#reversed + 1] = tostring(bits[i])
    end
    return "{" .. table.concat(reversed, ",") .. "}"
end

-- ========================================================================
-- Half Adder Tests
-- ========================================================================
describe("half_adder", function()
    -- Exhaustive test: there are only 4 possible input combinations for
    -- a two-input circuit, so we test every single one.
    --
    -- Truth table:
    --   A | B | Sum | Carry
    --   0 | 0 |  0  |   0    (0 + 0 = 0, no carry)
    --   0 | 1 |  1  |   0    (0 + 1 = 1, no carry)
    --   1 | 0 |  1  |   0    (1 + 0 = 1, no carry)
    --   1 | 1 |  0  |   1    (1 + 1 = 10 binary: sum=0, carry=1)

    it("0 + 0 = 0 with no carry", function()
        local sum, carry = arithmetic.half_adder(0, 0)
        assert.are.equal(0, sum)
        assert.are.equal(0, carry)
    end)

    it("0 + 1 = 1 with no carry", function()
        local sum, carry = arithmetic.half_adder(0, 1)
        assert.are.equal(1, sum)
        assert.are.equal(0, carry)
    end)

    it("1 + 0 = 1 with no carry", function()
        local sum, carry = arithmetic.half_adder(1, 0)
        assert.are.equal(1, sum)
        assert.are.equal(0, carry)
    end)

    it("1 + 1 = 0 with carry 1", function()
        local sum, carry = arithmetic.half_adder(1, 1)
        assert.are.equal(0, sum)
        assert.are.equal(1, carry)
    end)
end)

-- ========================================================================
-- Full Adder Tests
-- ========================================================================
describe("full_adder", function()
    -- Exhaustive test: three inputs (A, B, CarryIn) give 2^3 = 8 combinations.
    -- The full adder must correctly handle all of them.

    it("0 + 0 + 0 = 0 carry 0", function()
        local sum, carry = arithmetic.full_adder(0, 0, 0)
        assert.are.equal(0, sum)
        assert.are.equal(0, carry)
    end)

    it("0 + 0 + 1 = 1 carry 0", function()
        local sum, carry = arithmetic.full_adder(0, 0, 1)
        assert.are.equal(1, sum)
        assert.are.equal(0, carry)
    end)

    it("0 + 1 + 0 = 1 carry 0", function()
        local sum, carry = arithmetic.full_adder(0, 1, 0)
        assert.are.equal(1, sum)
        assert.are.equal(0, carry)
    end)

    it("0 + 1 + 1 = 0 carry 1", function()
        local sum, carry = arithmetic.full_adder(0, 1, 1)
        assert.are.equal(0, sum)
        assert.are.equal(1, carry)
    end)

    it("1 + 0 + 0 = 1 carry 0", function()
        local sum, carry = arithmetic.full_adder(1, 0, 0)
        assert.are.equal(1, sum)
        assert.are.equal(0, carry)
    end)

    it("1 + 0 + 1 = 0 carry 1", function()
        local sum, carry = arithmetic.full_adder(1, 0, 1)
        assert.are.equal(0, sum)
        assert.are.equal(1, carry)
    end)

    it("1 + 1 + 0 = 0 carry 1", function()
        local sum, carry = arithmetic.full_adder(1, 1, 0)
        assert.are.equal(0, sum)
        assert.are.equal(1, carry)
    end)

    it("1 + 1 + 1 = 1 carry 1", function()
        local sum, carry = arithmetic.full_adder(1, 1, 1)
        assert.are.equal(1, sum)
        assert.are.equal(1, carry)
    end)
end)

-- ========================================================================
-- Ripple Carry Adder Tests
-- ========================================================================
describe("ripple_carry_adder", function()
    -- These tests use 4-bit numbers, the smallest width that exercises
    -- carry propagation across multiple columns.

    it("adds 5 + 3 = 8 (no carry out)", function()
        -- 5 = 0101 -> {1, 0, 1, 0} LSB first
        -- 3 = 0011 -> {1, 1, 0, 0} LSB first
        -- 8 = 1000 -> {0, 0, 0, 1} LSB first
        local sum, carry = arithmetic.ripple_carry_adder(
            {1, 0, 1, 0}, {1, 1, 0, 0}, 0
        )
        assert.is_true(bits_equal(sum, {0, 0, 0, 1}))
        assert.are.equal(0, carry)
    end)

    it("adds 15 + 1 = 16 (overflow produces carry out)", function()
        -- 15 = 1111 -> {1, 1, 1, 1}
        --  1 = 0001 -> {1, 0, 0, 0}
        -- 16 = 10000 -> {0, 0, 0, 0} with carry = 1
        local sum, carry = arithmetic.ripple_carry_adder(
            {1, 1, 1, 1}, {1, 0, 0, 0}, 0
        )
        assert.is_true(bits_equal(sum, {0, 0, 0, 0}))
        assert.are.equal(1, carry)
    end)

    it("adds 0 + 0 = 0", function()
        local sum, carry = arithmetic.ripple_carry_adder(
            {0, 0, 0, 0}, {0, 0, 0, 0}, 0
        )
        assert.is_true(bits_equal(sum, {0, 0, 0, 0}))
        assert.are.equal(0, carry)
    end)

    it("adds with carry_in: 5 + 3 + 1 = 9", function()
        -- 5 + 3 = 8, plus carry_in 1 = 9
        -- 9 = 1001 -> {1, 0, 0, 1} LSB first
        local sum, carry = arithmetic.ripple_carry_adder(
            {1, 0, 1, 0}, {1, 1, 0, 0}, 1
        )
        assert.is_true(bits_equal(sum, {1, 0, 0, 1}))
        assert.are.equal(0, carry)
    end)

    it("adds 7 + 7 = 14", function()
        -- 7 = 0111 -> {1, 1, 1, 0}
        -- 14 = 1110 -> {0, 1, 1, 1}
        local sum, carry = arithmetic.ripple_carry_adder(
            {1, 1, 1, 0}, {1, 1, 1, 0}, 0
        )
        assert.is_true(bits_equal(sum, {0, 1, 1, 1}))
        assert.are.equal(0, carry)
    end)

    it("handles single-bit addition", function()
        local sum, carry = arithmetic.ripple_carry_adder({1}, {1}, 0)
        assert.is_true(bits_equal(sum, {0}))
        assert.are.equal(1, carry)
    end)

    it("handles 8-bit addition: 170 + 85 = 255", function()
        -- 170 = 10101010 -> {0,1,0,1,0,1,0,1} LSB first
        -- 85  = 01010101 -> {1,0,1,0,1,0,1,0} LSB first
        -- 255 = 11111111 -> {1,1,1,1,1,1,1,1} LSB first
        local sum, carry = arithmetic.ripple_carry_adder(
            {0,1,0,1,0,1,0,1}, {1,0,1,0,1,0,1,0}, 0
        )
        assert.is_true(bits_equal(sum, {1,1,1,1,1,1,1,1}))
        assert.are.equal(0, carry)
    end)

    it("errors on mismatched lengths", function()
        assert.has_error(function()
            arithmetic.ripple_carry_adder({0, 1}, {1, 0, 1}, 0)
        end)
    end)

    it("errors on empty bit lists", function()
        assert.has_error(function()
            arithmetic.ripple_carry_adder({}, {}, 0)
        end)
    end)
end)

-- ========================================================================
-- ALU Tests
-- ========================================================================
describe("ALU", function()
    -- All ALU tests use 4-bit width, which is wide enough to exercise
    -- carry propagation and signed overflow, yet small enough to verify
    -- results by hand.

    describe("construction", function()
        it("creates an ALU with a given bit width", function()
            local alu = arithmetic.ALU.new(8)
            assert.are.equal(8, alu.bit_width)
        end)

        it("errors on bit width less than 1", function()
            assert.has_error(function()
                arithmetic.ALU.new(0)
            end)
        end)

        it("errors on negative bit width", function()
            assert.has_error(function()
                arithmetic.ALU.new(-1)
            end)
        end)
    end)

    describe("ADD", function()
        it("adds 5 + 3 = 8 with correct flags", function()
            local alu = arithmetic.ALU.new(4)
            -- 5 = 0101 -> {1, 0, 1, 0}
            -- 3 = 0011 -> {1, 1, 0, 0}
            -- 8 = 1000 -> {0, 0, 0, 1}
            local res = alu:execute(arithmetic.ADD,
                {1, 0, 1, 0}, {1, 1, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.carry)
            -- MSB is 1, so negative flag is true
            assert.are.equal(true, res.negative)
            -- Two positive numbers (MSB=0) gave a negative result (MSB=1): overflow!
            assert.are.equal(true, res.overflow)
        end)

        it("adds 0 + 0 = 0 with zero flag set", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.ADD,
                {0, 0, 0, 0}, {0, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
            assert.are.equal(false, res.carry)
            assert.are.equal(false, res.negative)
            assert.are.equal(false, res.overflow)
        end)

        it("adds 15 + 1 = 0 with carry (unsigned overflow)", function()
            local alu = arithmetic.ALU.new(4)
            -- 15 = 1111 -> {1, 1, 1, 1}
            --  1 = 0001 -> {1, 0, 0, 0}
            -- 16 = 10000, truncated to 0000 with carry
            local res = alu:execute(arithmetic.ADD,
                {1, 1, 1, 1}, {1, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
            assert.are.equal(true, res.carry)
            assert.are.equal(false, res.negative)
            -- In signed: -1 + 1 = 0 (same signs? -1 MSB=1, 1 MSB=0 → different → no overflow)
            assert.are.equal(false, res.overflow)
        end)

        it("adds 1 + 1 = 2 (simple, no flags)", function()
            local alu = arithmetic.ALU.new(4)
            -- 1 = 0001 -> {1, 0, 0, 0}
            -- 2 = 0010 -> {0, 1, 0, 0}
            local res = alu:execute(arithmetic.ADD,
                {1, 0, 0, 0}, {1, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 1, 0, 0}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.carry)
            assert.are.equal(false, res.negative)
            assert.are.equal(false, res.overflow)
        end)

        it("detects negative result from two negative operands (signed)", function()
            local alu = arithmetic.ALU.new(4)
            -- -2 in 4-bit two's complement = 1110 -> {0,1,1,1}
            -- -3 in 4-bit two's complement = 1101 -> {1,0,1,1}
            -- -2 + -3 = -5 = 1011 -> {1,1,0,1}
            local res = alu:execute(arithmetic.ADD,
                {0, 1, 1, 1}, {1, 0, 1, 1})
            assert.is_true(bits_equal(res.value, {1, 1, 0, 1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(true, res.carry)
            assert.are.equal(true, res.negative)
            -- Both operands negative, result negative → no overflow
            assert.are.equal(false, res.overflow)
        end)
    end)

    describe("SUB", function()
        it("subtracts 5 - 3 = 2 with correct flags", function()
            local alu = arithmetic.ALU.new(4)
            -- 5 = 0101 -> {1, 0, 1, 0}
            -- 3 = 0011 -> {1, 1, 0, 0}
            -- 2 = 0010 -> {0, 1, 0, 0}
            local res = alu:execute(arithmetic.SUB,
                {1, 0, 1, 0}, {1, 1, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 1, 0, 0}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.negative)
            assert.are.equal(false, res.overflow)
        end)

        it("subtracts 3 - 3 = 0 with zero flag", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.SUB,
                {1, 1, 0, 0}, {1, 1, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
            assert.are.equal(false, res.negative)
        end)

        it("subtracts 3 - 5 = -2 (negative result)", function()
            local alu = arithmetic.ALU.new(4)
            -- 3 = {1, 1, 0, 0}
            -- 5 = {1, 0, 1, 0}
            -- -2 in two's complement = 1110 -> {0, 1, 1, 1}
            local res = alu:execute(arithmetic.SUB,
                {1, 1, 0, 0}, {1, 0, 1, 0})
            assert.is_true(bits_equal(res.value, {0, 1, 1, 1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(true, res.negative)
            assert.are.equal(false, res.overflow)
        end)

        it("detects overflow on subtraction", function()
            local alu = arithmetic.ALU.new(4)
            -- -8 - 1 should overflow (result would be -9, outside 4-bit signed range)
            -- -8 = 1000 -> {0, 0, 0, 1}
            --  1 = 0001 -> {1, 0, 0, 0}
            -- -8 - 1 = -8 + (-1) = 1000 + 1111 = 10111 = 0111 (carry=1) = +7 (wrong!)
            local res = alu:execute(arithmetic.SUB,
                {0, 0, 0, 1}, {1, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {1, 1, 1, 0}))
            -- Both operands effectively negative (A=-8 MSB=1, NOT(B)=NOT(0001)=1110 MSB=1)
            -- Result = 0111, MSB=0 → sign changed → overflow
            assert.are.equal(true, res.overflow)
        end)
    end)

    describe("AND", function()
        it("computes bitwise AND", function()
            local alu = arithmetic.ALU.new(4)
            -- 1010 AND 1100 = 1000
            -- 10 = {0, 1, 0, 1}, 12 = {0, 0, 1, 1}
            local res = alu:execute(arithmetic.AND,
                {0, 1, 0, 1}, {0, 0, 1, 1})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(true, res.negative) -- MSB is 1
            assert.are.equal(false, res.carry)
            assert.are.equal(false, res.overflow)
        end)

        it("AND with zero gives zero", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.AND,
                {1, 1, 1, 1}, {0, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
        end)

        it("AND with all ones gives identity", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.AND,
                {1, 0, 1, 0}, {1, 1, 1, 1})
            assert.is_true(bits_equal(res.value, {1, 0, 1, 0}))
        end)
    end)

    describe("OR", function()
        it("computes bitwise OR", function()
            local alu = arithmetic.ALU.new(4)
            -- 1010 OR 1100 = 1110 (14)
            local res = alu:execute(arithmetic.OR,
                {0, 1, 0, 1}, {0, 0, 1, 1})
            assert.is_true(bits_equal(res.value, {0, 1, 1, 1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(true, res.negative)
        end)

        it("OR with zero gives identity", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.OR,
                {1, 0, 1, 0}, {0, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {1, 0, 1, 0}))
        end)

        it("OR with all ones gives all ones", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.OR,
                {0, 0, 0, 0}, {1, 1, 1, 1})
            assert.is_true(bits_equal(res.value, {1, 1, 1, 1}))
        end)
    end)

    describe("XOR", function()
        it("computes bitwise XOR", function()
            local alu = arithmetic.ALU.new(4)
            -- 1010 XOR 1100 = 0110 (6)
            local res = alu:execute(arithmetic.XOR,
                {0, 1, 0, 1}, {0, 0, 1, 1})
            assert.is_true(bits_equal(res.value, {0, 1, 1, 0}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.negative)
        end)

        it("XOR with itself gives zero", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.XOR,
                {1, 0, 1, 1}, {1, 0, 1, 1})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
        end)

        it("XOR with zero gives identity", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.XOR,
                {1, 0, 1, 0}, {0, 0, 0, 0})
            assert.is_true(bits_equal(res.value, {1, 0, 1, 0}))
        end)
    end)

    describe("NOT", function()
        it("inverts all bits", function()
            local alu = arithmetic.ALU.new(4)
            -- NOT 1010 = 0101
            local res = alu:execute(arithmetic.NOT,
                {0, 1, 0, 1}, {})
            assert.is_true(bits_equal(res.value, {1, 0, 1, 0}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.negative) -- MSB=0
        end)

        it("NOT of zero gives all ones", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.NOT,
                {0, 0, 0, 0}, {})
            assert.is_true(bits_equal(res.value, {1, 1, 1, 1}))
            assert.are.equal(true, res.negative)
        end)

        it("NOT of all ones gives zero", function()
            local alu = arithmetic.ALU.new(4)
            local res = alu:execute(arithmetic.NOT,
                {1, 1, 1, 1}, {})
            assert.is_true(bits_equal(res.value, {0, 0, 0, 0}))
            assert.are.equal(true, res.zero)
            assert.are.equal(false, res.negative)
        end)
    end)

    describe("input validation", function()
        it("errors when a length does not match bit_width", function()
            local alu = arithmetic.ALU.new(4)
            assert.has_error(function()
                alu:execute(arithmetic.ADD, {1, 0}, {1, 0, 0, 0})
            end)
        end)

        it("errors when b length does not match bit_width", function()
            local alu = arithmetic.ALU.new(4)
            assert.has_error(function()
                alu:execute(arithmetic.ADD, {1, 0, 0, 0}, {1, 0})
            end)
        end)

        it("errors on unknown operation", function()
            local alu = arithmetic.ALU.new(4)
            assert.has_error(function()
                alu:execute("nope", {1, 0, 0, 0}, {1, 0, 0, 0})
            end)
        end)
    end)

    describe("8-bit operations", function()
        it("adds 100 + 55 = 155 in 8-bit", function()
            local alu = arithmetic.ALU.new(8)
            -- 100 = 01100100 -> {0,0,1,0,0,1,1,0} LSB first
            --  55 = 00110111 -> {1,1,1,0,1,1,0,0} LSB first
            -- 155 = 10011011 -> {1,1,0,1,1,0,0,1} LSB first
            local res = alu:execute(arithmetic.ADD,
                {0,0,1,0,0,1,1,0}, {1,1,1,0,1,1,0,0})
            assert.is_true(bits_equal(res.value, {1,1,0,1,1,0,0,1}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.carry)
            assert.are.equal(true, res.negative)
            -- Both positive (MSB=0), result negative (MSB=1) → overflow
            assert.are.equal(true, res.overflow)
        end)

        it("subtracts 200 - 100 = 100 in 8-bit", function()
            local alu = arithmetic.ALU.new(8)
            -- 200 = 11001000 -> {0,0,0,1,0,0,1,1} LSB first
            -- 100 = 01100100 -> {0,0,1,0,0,1,1,0} LSB first
            -- 100 = 01100100 -> {0,0,1,0,0,1,1,0} LSB first
            local res = alu:execute(arithmetic.SUB,
                {0,0,0,1,0,0,1,1}, {0,0,1,0,0,1,1,0})
            assert.is_true(bits_equal(res.value, {0,0,1,0,0,1,1,0}))
            assert.are.equal(false, res.zero)
            assert.are.equal(false, res.negative)
        end)
    end)

    describe("ALUResult", function()
        it("has a tostring representation", function()
            local res = arithmetic.ALUResult.new(
                {0, 0, 0, 1}, false, false, true, true
            )
            local s = tostring(res)
            assert.is_truthy(s:find("ALUResult"))
            assert.is_truthy(s:find("1000"))
        end)
    end)
end)

-- ========================================================================
-- Module API Tests
-- ========================================================================
describe("module API", function()
    it("has a version string", function()
        assert.are.equal("0.1.0", arithmetic.VERSION)
    end)

    it("exports half_adder", function()
        assert.is_function(arithmetic.half_adder)
    end)

    it("exports full_adder", function()
        assert.is_function(arithmetic.full_adder)
    end)

    it("exports ripple_carry_adder", function()
        assert.is_function(arithmetic.ripple_carry_adder)
    end)

    it("exports ALU constructor", function()
        assert.is_table(arithmetic.ALU)
        assert.is_function(arithmetic.ALU.new)
    end)

    it("exports operation constants", function()
        assert.are.equal("add", arithmetic.ADD)
        assert.are.equal("sub", arithmetic.SUB)
        assert.are.equal("and", arithmetic.AND)
        assert.are.equal("or", arithmetic.OR)
        assert.are.equal("xor", arithmetic.XOR)
        assert.are.equal("not", arithmetic.NOT)
    end)
end)
